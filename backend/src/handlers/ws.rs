use crate::AppState;
use crate::models::Claims;
use axum::{
    Extension,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, claims))
}

async fn handle_socket(socket: WebSocket, state: AppState, claims: Claims) {
    let (mut sender, mut receiver) = socket.split();
    let org_id = claims.organization_id;

    // 組織ごとのブロードキャストチャンネルを取得または作成
    // 本来はAppStateで管理すべきだが、まずはシンプルな実装から
    let mut rx = state.tx.subscribe();

    // 送信タスク
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // メッセージが同じ組織宛かチェック
            if msg.organization_id == org_id {
                let json = serde_json::to_string(&msg).unwrap();
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // 受信タスク（クライアントからのメッセージは現在はログ出力のみ）
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            println!("Received message from org {}: {}", org_id, text);
        }
    });

    // どちらかのタスクが終了したら、もう一方も終了させる
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
