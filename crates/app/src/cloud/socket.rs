//! The sync WebSocket on every target: `tokio-tungstenite` on the Mac and
//! Android, the browser's `WebSocket` in the web app. The engine only sees
//! text frames, closes and errors.

/// What the socket delivered.
#[derive(Debug)]
pub enum Frame {
    Text(String),
    /// The server closed it (with its close code, when it sent one).
    Closed(Option<u16>),
    /// Pings, pongs and binary frames: nothing to do.
    Other,
    Error(String),
}

#[derive(Debug)]
pub enum ConnectError {
    /// The hub said 401: the token is older than this device thinks.
    Unauthorized,
    /// Anything else. In the browser a refused upgrade looks like this too, so
    /// the engine refreshes the token before the next try there.
    Failed(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => f.write_str("401 unauthorized"),
            Self::Failed(e) => f.write_str(e),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::Socket;
#[cfg(target_arch = "wasm32")]
pub use web::Socket;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::{Error, Message};
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

    use super::{ConnectError, Frame};

    pub struct Socket(WebSocketStream<MaybeTlsStream<TcpStream>>);

    impl Socket {
        pub async fn connect(url: &str, access: &str) -> Result<Self, ConnectError> {
            let mut request = url.into_client_request().map_err(|e| ConnectError::Failed(e.to_string()))?;
            if let Ok(v) = format!("Bearer {access}").parse() {
                request.headers_mut().insert("authorization", v);
            }
            match tokio_tungstenite::connect_async(request).await {
                Ok((ws, _)) => Ok(Self(ws)),
                Err(Error::Http(res)) if res.status().as_u16() == 401 => Err(ConnectError::Unauthorized),
                Err(e) => Err(ConnectError::Failed(e.to_string())),
            }
        }

        pub async fn next(&mut self) -> Frame {
            match self.0.next().await {
                Some(Ok(Message::Text(text))) => Frame::Text(text.to_string()),
                Some(Ok(Message::Close(close))) => Frame::Closed(close.map(|c| u16::from(c.code))),
                Some(Ok(_)) => Frame::Other,
                Some(Err(e)) => Frame::Error(e.to_string()),
                None => Frame::Closed(None),
            }
        }

        pub async fn send_text(&mut self, text: String) -> Result<(), String> {
            self.0.send(Message::Text(text.into())).await.map_err(|e| e.to_string())
        }

        /// Keeps NAT mappings and the Cloudflare edge from dropping an idle socket.
        pub async fn ping(&mut self) -> Result<(), String> {
            self.0.send(Message::Ping(Vec::new().into())).await.map_err(|e| e.to_string())
        }

        pub async fn close(mut self) {
            let _ = self.0.close(None).await;
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::Closure;
    use web_sys::{CloseEvent, MessageEvent, WebSocket};

    use super::{ConnectError, Frame};

    enum Event {
        Open,
        Frame(Frame),
    }

    /// The browser's WebSocket and the handlers feeding its events in.
    pub struct Socket {
        ws: WebSocket,
        events: UnboundedReceiver<Event>,
        _handlers: [Closure<dyn FnMut(web_sys::Event)>; 4],
    }

    impl Socket {
        /// Browsers can't set headers on a WebSocket, so the access token
        /// rides in the query (the hub accepts it there for `/v1/sync` only).
        pub async fn connect(url: &str, access: &str) -> Result<Self, ConnectError> {
            let url = format!("{url}?access_token={access}");
            let ws = WebSocket::new(&url).map_err(|e| ConnectError::Failed(format!("{e:?}")))?;
            let (tx, events) = unbounded_channel();
            let handler = |f: Box<dyn Fn(web_sys::Event) -> Event>| {
                let tx = tx.clone();
                Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
                    let _ = tx.send(f(e));
                })
            };
            let on_open = handler(Box::new(|_| Event::Open));
            let on_message = handler(Box::new(|e| {
                let text = e.dyn_into::<MessageEvent>().ok().and_then(|m| m.data().as_string());
                Event::Frame(text.map_or(Frame::Other, Frame::Text))
            }));
            let on_close = handler(Box::new(|e| {
                Event::Frame(Frame::Closed(e.dyn_into::<CloseEvent>().ok().map(|c| c.code())))
            }));
            let on_error = handler(Box::new(|_| Event::Frame(Frame::Error("socket error".into()))));
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            let mut socket = Self { ws, events, _handlers: [on_open, on_message, on_close, on_error] };
            match socket.events.recv().await {
                Some(Event::Open) => Ok(socket),
                Some(Event::Frame(f)) => Err(ConnectError::Failed(format!("{f:?}"))),
                None => Err(ConnectError::Failed("closed".into())),
            }
        }

        pub async fn next(&mut self) -> Frame {
            loop {
                match self.events.recv().await {
                    Some(Event::Frame(f)) => return f,
                    Some(Event::Open) => continue,
                    None => return Frame::Closed(None),
                }
            }
        }

        pub async fn send_text(&mut self, text: String) -> Result<(), String> {
            self.ws.send_with_str(&text).map_err(|e| format!("{e:?}"))
        }

        /// Browsers can't send protocol pings, so the app-level one keeps an
        /// idle socket open.
        pub async fn ping(&mut self) -> Result<(), String> {
            let ping = serde_json::to_string(&steps_protocol::sync::ClientMsg::Ping).unwrap_or_default();
            self.send_text(ping).await
        }

        pub async fn close(self) {
            let _ = self.ws.close();
        }
    }

    impl Drop for Socket {
        fn drop(&mut self) {
            self.ws.set_onopen(None);
            self.ws.set_onmessage(None);
            self.ws.set_onclose(None);
            self.ws.set_onerror(None);
            let _ = self.ws.close();
        }
    }
}
