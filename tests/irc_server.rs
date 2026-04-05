use chat_system::server::Server;
use chat_system::servers::IrcListener;
use chat_system::{ChatListener, ChatServer};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Connect a minimal IRC client to `addr`, register with `nick`, send a single
/// PRIVMSG to `#test`, and disconnect.
async fn send_privmsg(addr: &str, nick: &str, msg: &str) {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let (reader, mut writer) = stream.split();
    let mut lines = BufReader::new(reader).lines();

    // Register
    writer
        .write_all(format!("NICK {nick}\r\nUSER u 0 * :u\r\n").as_bytes())
        .await
        .unwrap();

    // Wait for the 001 welcome line before sending PRIVMSG.
    while let Ok(Some(line)) = lines.next_line().await {
        if line.contains("001") {
            break;
        }
    }

    writer
        .write_all(format!("PRIVMSG #test :{msg}\r\n").as_bytes())
        .await
        .unwrap();
    // Brief pause so the server can process the message before we disconnect.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    writer.write_all(b"QUIT\r\n").await.unwrap();
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn server_single_listener_receives_message() {
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let addr = format!("127.0.0.1:{port}");
    let addr_clone = addr.clone();

    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let mut server = Server::new("test-server")
        .add_listener(IrcListener::new(addr));

    let server_handle = tokio::spawn(async move {
        server
            .run(move |msg| {
                let recv = received_clone.clone();
                async move {
                    recv.lock().unwrap().push(msg.content.clone());
                    Ok(None)
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    send_privmsg(&addr_clone, "testbot", "hello single").await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    server_handle.abort();
    let _ = server_handle.await;

    let msgs = received.lock().unwrap().clone();
    assert_eq!(msgs, vec!["hello single"]);
}

#[tokio::test]
async fn server_multiple_listeners_all_deliver_messages() {
    let l1 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port1 = l1.local_addr().unwrap().port();
    let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port2 = l2.local_addr().unwrap().port();
    drop(l1);
    drop(l2);

    let addr1 = format!("127.0.0.1:{port1}");
    let addr2 = format!("127.0.0.1:{port2}");

    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let mut server = Server::new("test-server")
        .add_listener(IrcListener::new(addr1.clone()))
        .add_listener(IrcListener::new(addr2.clone()));

    let server_handle = tokio::spawn(async move {
        server
            .run(move |msg| {
                let recv = received_clone.clone();
                async move {
                    recv.lock().unwrap().push(msg.content.clone());
                    Ok(None)
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Send one message through each listener.
    send_privmsg(&addr1, "bot1", "from port 1").await;
    send_privmsg(&addr2, "bot2", "from port 2").await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    server_handle.abort();
    let _ = server_handle.await;

    let mut msgs = received.lock().unwrap().clone();
    msgs.sort();
    assert_eq!(msgs, vec!["from port 1", "from port 2"]);
}

#[tokio::test]
async fn server_listeners_returns_all_attached_listeners() {
    let server = Server::new("test-server")
        .add_listener(IrcListener::new("127.0.0.1:6667"))
        .add_listener(IrcListener::new("127.0.0.1:6697"));

    assert_eq!(server.name(), "test-server");
    let listeners = server.listeners();
    assert_eq!(listeners.len(), 2);
    assert_eq!(listeners[0].address(), "127.0.0.1:6667");
    assert_eq!(listeners[0].protocol(), "irc");
    assert_eq!(listeners[1].address(), "127.0.0.1:6697");
    assert_eq!(listeners[1].protocol(), "irc");
}

#[tokio::test]
async fn irc_listener_protocol_is_irc() {
    let listener = IrcListener::new("127.0.0.1:6667");
    assert_eq!(listener.protocol(), "irc");
    assert_eq!(listener.address(), "127.0.0.1:6667");
}

#[tokio::test]
async fn server_empty_has_no_listeners() {
    let server = Server::new("empty-server");
    assert_eq!(server.name(), "empty-server");
    assert!(server.listeners().is_empty());
}

#[tokio::test]
async fn generic_server_config_roundtrip() {
    use chat_system::config::{IrcListenerConfig, ServerConfig};

    let cfg = ServerConfig {
        name: "srv".into(),
        listeners: vec![
            Box::new(IrcListenerConfig {
                address: "127.0.0.1:6667".into(),
            }),
            Box::new(IrcListenerConfig {
                address: "127.0.0.1:6668".into(),
            }),
            Box::new(IrcListenerConfig {
                address: "127.0.0.1:6669".into(),
            }),
        ],
    };

    assert_eq!(cfg.name(), "srv");
    assert_eq!(cfg.listeners.len(), 3);
    assert_eq!(cfg.listeners[0].address(), "127.0.0.1:6667");
    assert_eq!(cfg.listeners[1].address(), "127.0.0.1:6668");
    assert_eq!(cfg.listeners[2].address(), "127.0.0.1:6669");

    let json = serde_json::to_string(&cfg).unwrap();
    let decoded: ServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.name(), "srv");
    assert_eq!(decoded.listeners.len(), 3);

    use chat_system::GenericServer;
    let gs = GenericServer::new(decoded);
    assert_eq!(gs.name(), "srv");
}
