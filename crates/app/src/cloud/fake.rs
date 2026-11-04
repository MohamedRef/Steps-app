//! A stand-in for Steps Cloud on 127.0.0.1, for the engine's tests: the
//! HTTP API answers from a table of canned replies, and `/v1/sync` is a
//! real WebSocket whose answers a test script decides. Nothing here leaves
//! the machine.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use steps_protocol::cloud::{DeviceList, DeviceSummary, Platform, Session};
use steps_protocol::sync::{ClientMsg, ServerMsg};
use tokio_tungstenite::tungstenite::{self, Message};

/// What the hub sends back for one message from the device.
pub enum Reply {
    Msg(ServerMsg),
    /// A frame the device can't read.
    Raw(String),
    /// Closes the socket with this code.
    Close(u16),
}

type Handler = Box<dyn FnMut(&ClientMsg) -> Vec<Reply> + Send>;

/// One HTTP request the fake saw.
#[derive(Clone, Debug)]
pub struct Seen {
    /// `POST /v1/auth/refresh`
    pub route: String,
    pub auth: Option<String>,
    pub body: String,
}

#[derive(Default)]
pub struct Log {
    pub http: Vec<Seen>,
    /// The `authorization` header of every socket that opened.
    pub sockets: Vec<Option<String>>,
    pub ws: Vec<ClientMsg>,
}

pub struct Fake {
    pub url: String,
    routes: Arc<Mutex<HashMap<String, VecDeque<(u16, String)>>>>,
    refuse: Arc<Mutex<VecDeque<u16>>>,
    handler: Arc<Mutex<Handler>>,
    pub log: Arc<Mutex<Log>>,
}

impl Fake {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let fake = Self {
            url,
            routes: Default::default(),
            refuse: Default::default(),
            handler: Arc::new(Mutex::new(Box::new(|_| Vec::new()))),
            log: Default::default(),
        };
        let (routes, refuse, handler, log) =
            (fake.routes.clone(), fake.refuse.clone(), fake.handler.clone(), fake.log.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (routes, refuse, handler, log) =
                    (routes.clone(), refuse.clone(), handler.clone(), log.clone());
                std::thread::spawn(move || serve(stream, &routes, &refuse, &handler, &log));
            }
        });
        fake
    }

    /// Answers `route` (`GET /v1/devices`) with `status` and `body`. Replies
    /// queue up; the last one keeps answering.
    pub fn route(&self, route: &str, status: u16, body: impl Into<String>) -> &Self {
        self.routes.lock().unwrap().entry(route.to_owned()).or_default().push_back((status, body.into()));
        self
    }

    /// The next socket upgrade is answered with `status` instead.
    pub fn refuse_socket(&self, status: u16) -> &Self {
        self.refuse.lock().unwrap().push_back(status);
        self
    }

    /// How the hub answers each message on the socket.
    pub fn on_message(&self, f: impl FnMut(&ClientMsg) -> Vec<Reply> + Send + 'static) -> &Self {
        *self.handler.lock().unwrap() = Box::new(f);
        self
    }

    pub fn requests(&self, route: &str) -> Vec<Seen> {
        self.log.lock().unwrap().http.iter().filter(|s| s.route == route).cloned().collect()
    }

    pub fn sockets(&self) -> Vec<Option<String>> {
        self.log.lock().unwrap().sockets.clone()
    }

    pub fn received(&self) -> Vec<ClientMsg> {
        self.log.lock().unwrap().ws.clone()
    }
}

fn serve(
    stream: TcpStream,
    routes: &Mutex<HashMap<String, VecDeque<(u16, String)>>>,
    refuse: &Mutex<VecDeque<u16>>,
    handler: &Mutex<Handler>,
    log: &Mutex<Log>,
) {
    // Look at the request line without taking it off the stream: a socket
    // upgrade goes to tungstenite whole.
    let mut head = [0u8; 512];
    let mut line = String::new();
    for _ in 0..200 {
        let n = stream.peek(&mut head).unwrap_or(0);
        line = String::from_utf8_lossy(&head[..n]).into_owned();
        if line.contains("\r\n") || n == head.len() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
    if path.starts_with("/v1/sync") {
        socket(stream, refuse, handler, log);
    } else {
        http(stream, routes, log);
    }
}

fn http(stream: TcpStream, routes: &Mutex<HashMap<String, VecDeque<(u16, String)>>>, log: &Mutex<Log>) {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let mut parts = first.split_whitespace();
    let route = format!("{} {}", parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let (mut auth, mut len) = (None, 0usize);
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "authorization" => auth = Some(v.trim().to_owned()),
                "content-length" => len = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let mut body = vec![0u8; len];
    let _ = reader.read_exact(&mut body);
    let body = String::from_utf8_lossy(&body).into_owned();
    log.lock().unwrap().http.push(Seen { route: route.clone(), auth, body });
    let (status, text) = {
        let mut routes = routes.lock().unwrap();
        match routes.get_mut(&route) {
            Some(q) if q.len() > 1 => q.pop_front().unwrap(),
            Some(q) if !q.is_empty() => q[0].clone(),
            _ => (404, r#"{"code":"not_found","message":"no such route"}"#.to_owned()),
        }
    };
    let mut stream = reader.into_inner();
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    );
}

fn socket(mut stream: TcpStream, refuse: &Mutex<VecDeque<u16>>, handler: &Mutex<Handler>, log: &Mutex<Log>) {
    if let Some(status) = refuse.lock().unwrap().pop_front() {
        // Read the request off first, so the client sees the answer.
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        let _ = write!(stream, "HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        return;
    }
    let mut auth = None;
    let ws = tungstenite::accept_hdr(
        stream,
        |req: &tungstenite::handshake::server::Request, res: tungstenite::handshake::server::Response| {
            auth = req.headers().get("authorization").and_then(|v| v.to_str().ok()).map(str::to_owned);
            Ok(res)
        },
    );
    let Ok(mut ws) = ws else { return };
    log.lock().unwrap().sockets.push(auth);
    loop {
        let text = match ws.read() {
            Ok(Message::Text(t)) => t.to_string(),
            Ok(Message::Close(_)) | Err(_) => return,
            Ok(_) => continue,
        };
        let Ok(msg) = serde_json::from_str::<ClientMsg>(&text) else { continue };
        log.lock().unwrap().ws.push(msg.clone());
        let replies = (handler.lock().unwrap())(&msg);
        for r in replies {
            let sent = match r {
                Reply::Msg(m) => ws.send(Message::Text(serde_json::to_string(&m).unwrap().into())),
                Reply::Raw(t) => ws.send(Message::Text(t.into())),
                Reply::Close(code) => {
                    let frame = tungstenite::protocol::CloseFrame { code: code.into(), reason: "".into() };
                    let _ = ws.close(Some(frame));
                    // Let the close go out before the socket drops.
                    while ws.read().is_ok() {}
                    return;
                }
            };
            if sent.is_err() {
                return;
            }
        }
    }
}

/// A session that is good for a day (`access` is the access token).
pub fn session(device: &str, access: &str, access_expires_at_ms: u64) -> Session {
    let now = super::engine::now_ms();
    Session {
        account_id: "acc-mohamed".into(),
        device_id: device.into(),
        email: Some("mohamed@example.com".into()),
        access_token: access.into(),
        access_expires_at_ms,
        refresh_token: format!("refresh-{access}"),
        refresh_expires_at_ms: now + 86_400_000,
    }
}

pub fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

/// The Devices list: this one and the Tab S11.
pub fn devices(me: &str) -> String {
    let row = |id: &str, name: &str, platform, current| DeviceSummary {
        device_id: id.into(),
        name: name.into(),
        model: None,
        platform,
        signed_in_at_ms: 1,
        last_seen_ms: 2,
        last_sync_ms: Some(3),
        current,
        revoked: false,
    };
    json(&DeviceList {
        devices: vec![
            row(me, "Steps (dev)", Platform::Other, true),
            row("tab", "SM-X730", Platform::Android, false),
        ],
    })
}
