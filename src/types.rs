use tokio::sync::oneshot;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One in-flight HTTP request handed to the Lua thread. The hyper worker
/// parks on `respond` until Lua produces a `Response`.
pub struct Job {
    pub request: Request,
    pub respond: oneshot::Sender<Response>,
}

/// One WebSocket handshake handed to the Lua thread. The hyper worker
/// parks on `respond`: `Accept` completes the upgrade, `Reject` answers
/// with a plain HTTP response (404, 401 from a middleware, ...).
pub struct WsUpgrade {
    pub id: u64,
    pub request: Request,
    pub respond: oneshot::Sender<WsDecision>,
}

pub enum WsDecision {
    Accept,
    Reject(Response),
}

/// One event on an open WebSocket connection, delivered to Lua in order.
/// `Close` is always the last event a connection emits.
pub enum WsEvent {
    Message { data: Vec<u8>, binary: bool },
    Close { code: Option<u16>, reason: String },
    Error { message: String },
}

/// Everything that can wake the Lua thread: an HTTP request to dispatch,
/// the lifecycle of a WebSocket connection, or (dev mode) a batch of
/// changed `*.lua` files to hot-reload.
pub enum Msg {
    Job(Job),
    WsUpgrade(WsUpgrade),
    WsOpen {
        id: u64,
        handle: crate::ws::WsHandle,
    },
    WsEvent {
        id: u64,
        event: WsEvent,
    },
    Reload(Vec<String>),
}
