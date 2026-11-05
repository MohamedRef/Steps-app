//! The account's Durable Object: device registry + sync hub, with WebSocket
//! hibernation so idle sockets cost nothing.
// adapted from batna-app main: crates/server/src/wasm/account.rs (no push
// reminders, no image blobs)

use steps_protocol::cloud::{DeviceList, ErrorCode};
use steps_protocol::sync::{SeqOp, ServerMsg, SyncErrorCode};
use worker::{
    DurableObject, Env, Method, Request, Response, Result, State, WebSocket, WebSocketIncomingMessage,
    WebSocketPair, console_error, durable_object,
};

use super::do_sql::DoSql;
use super::{api_error, now_ms, random_hex};
use crate::devices::{Devices, RefreshError};
use crate::hub::{Conn, Hub};
use crate::internal::{AccountLoginState, DEVICE_HEADER, Expiry, LoginName, Refresh, Register, Revoke};
use crate::login::AccountLogin;

/// Close code sent to a signed-out device.
const CLOSE_REVOKED: u16 = 4001;

#[durable_object]
pub struct Account {
    state: State,
    /// Why storage couldn't be set up at start, if it couldn't: every
    /// request then gets that reason instead of a stray SQL error.
    storage: std::result::Result<(), String>,
}

impl Account {
    fn db(&self) -> DoSql {
        DoSql(self.state.storage().sql())
    }

    fn send(ws: &WebSocket, msg: &ServerMsg) {
        if let Err(e) = ws.send(msg) {
            console_error!("send failed: {e}");
        }
    }

    fn close_device(&self, device: &str) {
        for ws in self.state.get_websockets_with_tag(device) {
            Self::send(&ws, &ServerMsg::Revoked);
            let _ = ws.close(Some(CLOSE_REVOKED), Some("signed out"));
        }
    }

    fn device_header(req: &Request) -> Result<String> {
        req.headers()
            .get(DEVICE_HEADER)?
            .ok_or_else(|| worker::Error::RustError("missing device header".into()))
    }

    /// Sends stored ops to every ready socket except `except` (a conn id).
    fn broadcast(&self, ops: Vec<SeqOp>, except: Option<&str>) {
        if ops.is_empty() {
            return;
        }
        let msg = ServerMsg::Ops { ops, more: false };
        for other in self.state.get_websockets() {
            let Ok(Some(c)) = other.deserialize_attachment::<Conn>() else { continue };
            if c.ready && Some(c.conn_id.as_str()) != except {
                Self::send(&other, &msg);
            }
        }
    }

    fn accept_socket(&self, req: &Request) -> Result<Response> {
        let device = Self::device_header(req)?;
        let db = self.db();
        if !Devices::open(&db).is_active(&device).map_err(sql_err)? {
            return api_error(ErrorCode::DeviceRevoked, "this device was signed out");
        }
        let pair = WebSocketPair::new()?;
        // Unique ids are what keep a broadcast from skipping every socket, so
        // no randomness means no socket.
        let Ok(conn_id) = random_hex(8) else {
            return api_error(ErrorCode::Internal, "no secure randomness for the connection id");
        };
        let conn = Conn { device: device.clone(), conn_id, ready: false };
        self.state.accept_websocket_with_tags(&pair.server, &[&device]);
        pair.server.serialize_attachment(&conn)?;
        Devices::open(&db).touch(&device, now_ms(), false).map_err(sql_err)?;
        Response::from_websocket(pair.client)
    }
}

fn sql_err(e: crate::sql::SqlError) -> worker::Error {
    worker::Error::RustError(e.to_string())
}

impl DurableObject for Account {
    fn new(state: State, _env: Env) -> Self {
        let db = DoSql(state.storage().sql());
        // Synchronous, so it finishes before any event is delivered.
        let storage = crate::account::open(&db, |w| console_error!("{w}"));
        if let Err(e) = &storage {
            console_error!("migration failed: {e}");
        }
        Self { state, storage }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        if let Err(why) = &self.storage {
            let answer = crate::account::unavailable(why);
            return api_error(answer.code, answer.message);
        }
        let db = self.db();
        let devices = Devices::open(&db);
        let now = now_ms();
        match (req.method(), req.path().as_str()) {
            (Method::Get, "/ws") => self.accept_socket(&req),
            (Method::Post, "/register") => {
                let body: Register = req.json().await?;
                let expires =
                    devices.register(&body.info, &body.email, &body.refresh_hash, now).map_err(sql_err)?;
                devices.set_by_password(&body.info.device_id, body.by_password).map_err(sql_err)?;
                Response::from_json(&Expiry { refresh_expires_ms: expires, email: body.email })
            }
            (Method::Post, "/refresh") => {
                let body: Refresh = req.json().await?;
                match devices.refresh(&body.device, &body.presented_hash, &body.new_hash, now) {
                    Ok(r) => {
                        Response::from_json(&Expiry { refresh_expires_ms: r.expires_ms, email: r.email })
                    }
                    Err(RefreshError::Sql(e)) => Err(sql_err(e)),
                    Err(e @ (RefreshError::Revoked | RefreshError::Reused)) => {
                        self.close_device(&body.device);
                        api_error(ErrorCode::DeviceRevoked, e.to_string())
                    }
                    Err(e) => api_error(ErrorCode::Unauthorized, e.to_string()),
                }
            }
            // A signed-out device's access token lives on for up to 15
            // minutes; it gets no say over the other devices (#259).
            (Method::Get, "/devices") => {
                let current = Self::device_header(&req)?;
                if !devices.is_active(&current).map_err(sql_err)? {
                    return api_error(ErrorCode::DeviceRevoked, "this device was signed out");
                }
                let list = devices.list(&current).map_err(sql_err)?;
                Response::from_json(&DeviceList { devices: list })
            }
            // Username + password (`crate::login`): only a signed-in device
            // may read or change it.
            (Method::Get, "/login") => {
                let caller = Self::device_header(&req)?;
                let Some(email) = devices.email(&caller).map_err(sql_err)? else {
                    return api_error(ErrorCode::DeviceRevoked, "this device was signed out");
                };
                let username = AccountLogin::open(&db).username().map_err(sql_err)?;
                let by_password = devices.by_password(&caller).map_err(sql_err)?;
                Response::from_json(&AccountLoginState { email, username, by_password })
            }
            (Method::Post, "/login") => {
                let caller = Self::device_header(&req)?;
                if !devices.is_active(&caller).map_err(sql_err)? {
                    return api_error(ErrorCode::DeviceRevoked, "this device was signed out");
                }
                let body: LoginName = req.json().await?;
                AccountLogin::open(&db).set_username(body.username.as_deref()).map_err(sql_err)?;
                Response::empty()
            }
            (Method::Post, "/revoke") => {
                let caller = Self::device_header(&req)?;
                let body: Revoke = req.json().await?;
                // Signing itself out again is harmless, so only others are checked.
                if body.device != caller && !devices.is_active(&caller).map_err(sql_err)? {
                    return api_error(ErrorCode::DeviceRevoked, "this device was signed out");
                }
                if devices.revoke(&body.device, now).map_err(sql_err)? {
                    self.close_device(&body.device);
                    Response::empty()
                } else {
                    api_error(ErrorCode::NotFound, "no such device")
                }
            }
            _ => api_error(ErrorCode::NotFound, "not found"),
        }
    }

    async fn websocket_message(&self, ws: WebSocket, message: WebSocketIncomingMessage) -> Result<()> {
        let WebSocketIncomingMessage::String(text) = message else {
            Self::send(
                &ws,
                &ServerMsg::Error { code: SyncErrorCode::BadMessage, message: "text frames only".into() },
            );
            return Ok(());
        };
        if let Err(why) = &self.storage {
            let message = crate::account::unavailable(why).message;
            Self::send(&ws, &ServerMsg::Error { code: SyncErrorCode::Internal, message });
            let _ = ws.close(Some(1011), Some("storage unavailable"));
            return Ok(());
        }
        let Some(mut conn) = ws.deserialize_attachment::<Conn>()? else {
            let _ = ws.close(Some(1011), Some("lost connection state"));
            return Ok(());
        };
        let db = self.db();
        let devices = Devices::open(&db);
        if !devices.is_active(&conn.device).map_err(sql_err)? {
            Self::send(&ws, &ServerMsg::Revoked);
            let _ = ws.close(Some(CLOSE_REVOKED), Some("signed out"));
            return Ok(());
        }
        let was_ready = conn.ready;
        let now = now_ms();
        let outcome = Hub::open(&db).handle(&mut conn, &text, now).map_err(sql_err)?;
        if conn.ready != was_ready {
            ws.serialize_attachment(&conn)?;
        }
        for reply in &outcome.replies {
            Self::send(&ws, reply);
        }
        self.broadcast(outcome.broadcast, Some(&conn.conn_id));
        devices.touch(&conn.device, now, outcome.synced).map_err(sql_err)?;
        Ok(())
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        // Complete the closing handshake. 1005/1006 can't be sent back.
        let code =
            if (1000..=4999).contains(&code) && code != 1005 && code != 1006 { code as u16 } else { 1000 };
        let _ = ws.close(Some(code), Some("bye"));
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, error: worker::Error) -> Result<()> {
        console_error!("websocket error: {error}");
        Ok(())
    }
}
