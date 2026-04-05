use chat_system::servers::IrcServer;
use chat_system::{ChatServer, IrcListener};
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
async fn irc_server_single_listener_receives_message() {
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let mut server = IrcServer::new("127.0.0.1:0");

    // Run the server in the background; it exits when all listeners shut down.
    let run_handle = tokio::spawn(async move {
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

    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // We need the bound address.  Start a helper server on port 0 to find a
    // free port, then create a fresh IrcServer bound to that port so we know
    // the address up-front.
    drop(run_handle);

    // ── simpler approach: bind to a known-free port via std ──────────────────
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let received2: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received2_clone = received2.clone();

    let addr = format!("127.0.0.1:{port}");
    let addr_clone = addr.clone();

    let mut server2 = IrcServer::new(addr.clone());
    let server_handle = tokio::spawn(async move {
        server2
            .run(move |msg| {
                let recv = received2_clone.clone();
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

    // Shut the server down by dropping the handle and aborting the task.
    server_handle.abort();
    let _ = server_handle.await;

    let msgs = received2.lock().unwrap().clone();
    assert_eq!(msgs, vec!["hello single"]);
}

#[tokio::test]
async fn irc_server_multiple_listeners_all_deliver_messages() {
    // Grab two free ports.
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

    let mut server = IrcServer::new(addr1.clone());
    server.add_listener(IrcListener::new(addr2.clone()));

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
async fn irc_server_addresses_returns_all_listener_addresses() {
    let l1 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port1 = l1.local_addr().unwrap().port();
    let l2 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port2 = l2.local_addr().unwrap().port();
    drop(l1);
    drop(l2);

    let addr1 = format!("127.0.0.1:{port1}");
    let addr2 = format!("127.0.0.1:{port2}");

    let mut server = IrcServer::new(addr1.clone());
    server.add_listener(IrcListener::new(addr2.clone()));

    assert_eq!(server.address(), addr1);
    let addrs = server.addresses();
    assert_eq!(addrs.len(), 2);
    assert!(addrs.contains(&addr1.as_str()));
    assert!(addrs.contains(&addr2.as_str()));
}

#[tokio::test]
async fn irc_server_new_single_address() {
    let server = IrcServer::new("127.0.0.1:6667");
    assert_eq!(server.address(), "127.0.0.1:6667");
    assert_eq!(server.addresses(), vec!["127.0.0.1:6667"]);
}

#[tokio::test]
async fn generic_server_config_extra_binds_roundtrip() {
    use chat_system::config::{IrcServerConfig, ServerConfig};
    use chat_system::ChatServer;

    let cfg = ServerConfig::Irc(IrcServerConfig {
        name: "srv".into(),
        bind: "127.0.0.1:6667".into(),
        extra_binds: vec!["127.0.0.1:6668".into(), "127.0.0.1:6669".into()],
    });

    assert_eq!(cfg.bind_addresses(), vec!["127.0.0.1:6667", "127.0.0.1:6668", "127.0.0.1:6669"]);

    let json = serde_json::to_string(&cfg).unwrap();
    let decoded: ServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(
        decoded.bind_addresses(),
        vec!["127.0.0.1:6667", "127.0.0.1:6668", "127.0.0.1:6669"]
    );

    use chat_system::GenericServer;
    let gs = GenericServer::new(decoded);
    assert_eq!(
        gs.addresses(),
        vec!["127.0.0.1:6667", "127.0.0.1:6668", "127.0.0.1:6669"]
    );
}
