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
