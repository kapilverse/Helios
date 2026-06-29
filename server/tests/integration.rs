use futures_util::{SinkExt, StreamExt};
use helios_network::{app, AppState};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

async fn start_server() -> (String, oneshot::Sender<()>) {
    let state = Arc::new(AppState::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}/ws", addr);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let server = axum::serve(listener, app(state));
        tokio::select! {
            _ = server => {}
            _ = shutdown_rx => {}
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    (url, shutdown_tx)
}

async fn connect_ws(url: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (ws, _) = connect_async(url).await.expect("Failed to connect");
    ws
}

async fn recv_msg(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Value {
    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timeout waiting for message")
        .expect("WS stream ended")
        .expect("WS error");
    match msg {
        Message::Text(text) => serde_json::from_str(&text).expect("Invalid JSON"),
        other => panic!("Unexpected message: {:?}", other),
    }
}

async fn drain_until_op(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Value {
    loop {
        let msg = recv_msg(ws).await;
        if msg.get("Op").is_some() {
            return msg;
        }
    }
}

async fn send_msg(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, msg: Value) {
    let text = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(text.into()))
        .await
        .expect("Send failed");
}

#[tokio::test]
async fn test_two_clients_connect_and_sync() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;
    let mut ws2 = connect_ws(&url).await;

    let welcome1 = recv_msg(&mut ws1).await;
    assert!(welcome1.get("Sync").is_some());

    let welcome2 = recv_msg(&mut ws2).await;
    assert!(welcome2.get("Sync").is_some());

    println!("[PASS] Two clients connected and received welcome sync");
}

#[tokio::test]
async fn test_op_broadcast() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;
    let mut ws2 = connect_ws(&url).await;

    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drain any presence broadcasts on ws2
    loop {
        if tokio::time::timeout(Duration::from_millis(200), ws2.next())
            .await
            .is_err()
        {
            break;
        }
    }

    send_msg(
        &mut ws1,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                        "after": null,
                        "content": "H"
                    }
                }
            }
        }),
    )
    .await;

    let msg = drain_until_op(&mut ws2).await;
    let op_data = &msg["Op"]["op"]["Insert"];
    assert_eq!(op_data["content"], "H");

    println!("[PASS] Op broadcast between clients");
}

#[tokio::test]
async fn test_concurrent_edits_converge() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;
    let mut ws2 = connect_ws(&url).await;

    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drain presence on both
    for ws in [&mut ws1, &mut ws2].iter_mut() {
        loop {
            if tokio::time::timeout(Duration::from_millis(200), ws.next())
                .await
                .is_err()
            {
                break;
            }
        }
    }

    send_msg(
        &mut ws1,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                        "after": null,
                        "content": "A"
                    }
                }
            }
        }),
    )
    .await;

    send_msg(
        &mut ws2,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000002", "clock": 1 },
                        "after": null,
                        "content": "B"
                    }
                }
            }
        }),
    )
    .await;

    let msg1 = drain_until_op(&mut ws1).await;
    let msg2 = drain_until_op(&mut ws2).await;

    assert!(msg1.get("Op").is_some());
    assert!(msg2.get("Op").is_some());

    println!("[PASS] Concurrent edits broadcast to both clients");
}

#[tokio::test]
async fn test_delta_sync() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;

    recv_msg(&mut ws1).await;

    send_msg(
        &mut ws1,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                        "after": null,
                        "content": "X"
                    }
                }
            }
        }),
    )
    .await;

    recv_msg(&mut ws1).await;

    let mut ws2 = connect_ws(&url).await;
    let welcome = recv_msg(&mut ws2).await;

    assert!(welcome.get("Sync").is_some());
    let sync = &welcome["Sync"]["response"];
    assert!(sync["current_seq"].as_u64().unwrap() >= 1);

    println!("[PASS] Delta sync works for new client");
}

#[tokio::test]
async fn test_presence_broadcast() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;
    let mut ws2 = connect_ws(&url).await;

    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    send_msg(
        &mut ws1,
        json!({
            "Presence": {
                "cursor": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                "selection_start": null,
                "selection_end": null,
                "viewport_top": null,
                "viewport_bottom": null
            }
        }),
    )
    .await;

    // Drain until we get a presence message
    loop {
        let msg = recv_msg(&mut ws2).await;
        if msg.get("Presence").is_some() {
            let peers = msg["Presence"]["peers"].as_array().unwrap();
            assert!(peers.len() >= 1);
            println!("[PASS] Presence broadcast works");
            return;
        }
    }
}

#[tokio::test]
async fn test_multiple_ops_sequence() {
    let (url, _shutdown) = start_server().await;
    let mut ws1 = connect_ws(&url).await;
    let mut ws2 = connect_ws(&url).await;

    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drain presence
    loop {
        if tokio::time::timeout(Duration::from_millis(200), ws2.next())
            .await
            .is_err()
        {
            break;
        }
    }

    send_msg(
        &mut ws1,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                        "after": null,
                        "content": "H"
                    }
                }
            }
        }),
    )
    .await;
    drain_until_op(&mut ws2).await;

    send_msg(
        &mut ws1,
        json!({
            "Op": {
                "op": {
                    "Insert": {
                        "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 2 },
                        "after": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                        "content": "i"
                    }
                }
            }
        }),
    )
    .await;

    let msg = drain_until_op(&mut ws2).await;
    let content = msg["Op"]["op"]["Insert"]["content"].as_str().unwrap();
    assert_eq!(content, "i");

    println!("[PASS] Sequential ops propagate correctly");
}
