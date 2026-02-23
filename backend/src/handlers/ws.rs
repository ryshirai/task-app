use crate::AppState;
use worker::{Request, Response, Result as WorkerResult, RouteContext, WebSocketPair};

pub async fn ws_handler(req: Request, _ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let upgrade = req
        .headers()
        .get("Upgrade")?
        .unwrap_or_default()
        .to_ascii_lowercase();
    if upgrade != "websocket" {
        return Response::error("Expected websocket upgrade", 426);
    }

    let pair = WebSocketPair::new()?;
    let server = pair.server;
    let client = pair.client;
    server.accept()?;

    // NOTE: This Worker only upgrades the connection and does not keep shared state.
    // Cross-organization real-time fan-out requires Durable Objects (or another external pub/sub).
    let _ = server.send_with_str("{\"type\":\"connected\",\"mode\":\"stateless\"}");

    Response::from_websocket(client)
}
