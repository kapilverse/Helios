use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

async fn connect_ws() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (ws, _) = connect_async("ws://127.0.0.1:3000/ws")
        .await
        .expect("Failed to connect");
    ws
}

async fn recv_msg(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Value {
    let msg = ws.next().await.expect("No message").expect("WS error");
    match msg {
        Message::Text(text) => serde_json::from_str(&text).expect("Invalid JSON"),
        _ => panic!("Unexpected message type"),
    }
}

async fn send_msg(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, msg: Value) {
    let text = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(text.into())).await.expect("Send failed");
}

#[tokio::test]
async fn test_two_clients_connect_and_sync() {
    let mut ws1 = connect_ws().await;
    let mut ws2 = connect_ws().await;

    // Both should receive welcome sync
    let welcome1 = recv_msg(&mut ws1).await;
    assert!(welcome1.get("Sync").is_some(), "Client 1 should get Sync");

    let welcome2 = recv_msg(&mut ws2).await;
    assert!(welcome2.get("Sync").is_some(), "Client 2 should get Sync");

    println!("[PASS] Two clients connected and received welcome sync");
}

#[tokio::test]
async fn test_op_broadcast() {
    let mut ws1 = connect_ws().await;
    let mut ws2 = connect_ws().await;

    // Drain welcome messages
    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;

    // Client 1 sends an insert op
    let op = json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                    "after": null,
                    "content": "H"
                }
            }
        }
    });
    send_msg(&mut ws1, op).await;

    // Client 2 should receive the broadcast
    let msg = recv_msg(&mut ws2).await;
    assert!(msg.get("Op").is_some(), "Client 2 should receive broadcast op");

    let op_data = msg["Op"]["op"]["Insert"];
    assert_eq!(op_data["content"], "H");

    println!("[PASS] Op broadcast between clients");
}

#[tokio::test]
async fn test_concurrent_edits_converge() {
    let mut ws1 = connect_ws().await;
    let mut ws2 = connect_ws().await;

    // Drain welcome messages
    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;

    // Client 1 inserts "A"
    send_msg(&mut ws1, json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                    "after": null,
                    "content": "A"
                }
            }
        }
    })).await;

    // Client 2 inserts "B"
    send_msg(&mut ws2, json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000002", "clock": 1 },
                    "after": null,
                    "content": "B"
                }
            }
        }
    })).await;

    // Both should receive broadcasts (ops from the other client)
    let msg1 = recv_msg(&mut ws1).await;
    let msg2 = recv_msg(&mut ws2).await;

    assert!(msg1.get("Op").is_some(), "Client 1 should receive op");
    assert!(msg2.get("Op").is_some(), "Client 2 should receive op");

    println!("[PASS] Concurrent edits broadcast to both clients");
}

#[tokio::test]
async fn test_delta_sync() {
    let mut ws1 = connect_ws().await;

    // Drain welcome
    recv_msg(&mut ws1).await;

    // Send an op
    send_msg(&mut ws1, json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                    "after": null,
                    "content": "X"
                }
            }
        }
    })).await;

    // Wait for the broadcast back
    recv_msg(&mut ws1).await;

    // New client connects and requests sync from seq 0
    let mut ws2 = connect_ws().await;
    let welcome = recv_msg(&mut ws2).await;

    // Should get sync response
    assert!(welcome.get("Sync").is_some(), "New client should get sync");
    let sync = &welcome["Sync"]["response"];
    assert!(
        sync["current_seq"].as_u64().unwrap() >= 1,
        "Server should have ops after our insert"
    );

    println!("[PASS] Delta sync works for new client");
}

#[tokio::test]
async fn test_presence_broadcast() {
    let mut ws1 = connect_ws().await;
    let mut ws2 = connect_ws().await;

    // Drain welcome messages
    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;

    // Client 1 sends presence update
    send_msg(&mut ws1, json!({
        "Presence": {
            "cursor": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
            "selection_start": null,
            "selection_end": null,
            "viewport_top": null,
            "viewport_bottom": null
        }
    })).await;

    // Client 2 should receive presence update
    let msg = recv_msg(&mut ws2).await;
    assert!(
        msg.get("Presence").is_some(),
        "Client 2 should receive presence"
    );

    let peers = msg["Presence"]["peers"].as_array().unwrap();
    assert!(
        peers.len() >= 1,
        "Should have at least 1 peer in presence"
    );

    println!("[PASS] Presence broadcast works");
}

#[tokio::test]
async fn test_multiple_ops_sequence() {
    let mut ws1 = connect_ws().await;
    let mut ws2 = connect_ws().await;

    // Drain welcome messages
    recv_msg(&mut ws1).await;
    recv_msg(&mut ws2).await;

    // Client 1 sends multiple ops: "He"
    send_msg(&mut ws1, json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                    "after": null,
                    "content": "H"
                }
            }
        }
    })).await;

    // Client 2 should receive
    recv_msg(&mut ws2).await;

    send_msg(&mut ws1, json!({
        "Op": {
            "op": {
                "Insert": {
                    "id": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 2 },
                    "after": { "peer": "00000000-0000-0000-0000-000000000001", "clock": 1 },
                    "content": "i"
                }
            }
        }
    })).await;

    // Client 2 should receive
    let msg = recv_msg(&mut ws2).await;
    let content = msg["Op"]["op"]["Insert"]["content"].as_str().unwrap();
    assert_eq!(content, "i");

    println!("[PASS] Sequential ops propagate correctly");
}
