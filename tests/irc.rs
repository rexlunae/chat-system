use chat_system::Messenger;
use chat_system::messengers::IrcMessenger;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// Spawns a minimal IRC echo server on a random port and returns the port number.
/// The server handles NICK, USER, PING, PRIVMSG (echoes back), JOIN (ignored), and QUIT.
async fn start_irc_echo_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(handle_irc_connection(stream));
        }
    });

    port
}

async fn handle_irc_connection(stream: tokio::net::TcpStream) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut nick = "unknown".to_string();
    let mut user_seen = false;
    let mut registered = false;

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("NICK ") {
            nick = rest.trim().to_string();
        } else if line.starts_with("USER ") {
            user_seen = true;
        } else if line.starts_with("PING ") {
            let token = line.trim_start_matches("PING ");
            let _ = writer
                .write_all(format!("PONG {}\r\n", token).as_bytes())
                .await;
        } else if line.starts_with("PRIVMSG ") {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                let target = parts[1];
                let msg = parts[2].trim_start_matches(':');
                let reply = format!(":echo!echo@localhost PRIVMSG {} :echo: {}\r\n", target, msg);
                let _ = writer.write_all(reply.as_bytes()).await;
            }
        } else if line == "QUIT" || line.starts_with("QUIT ") {
            break;
        }

        if !registered && !nick.is_empty() && user_seen {
            let welcome = format!(
                ":localhost 001 {} :Welcome to the test IRC server\r\n",
                nick
            );
            let _ = writer.write_all(welcome.as_bytes()).await;
            registered = true;
        }
    }
}

#[tokio::test]
async fn irc_name_and_type() {
    let client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        6667,
        "bot".to_string(),
    );
    assert_eq!(client.name(), "test-irc");
    assert_eq!(client.messenger_type(), "irc");
}

#[tokio::test]
async fn irc_not_connected_before_initialize() {
    let client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        6667,
        "bot".to_string(),
    );
    assert!(!client.is_connected());
}

#[tokio::test]
async fn irc_initialize_connects_and_registers() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    );
    client.initialize().await.unwrap();
    assert!(client.is_connected());
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn irc_disconnect_clears_connected() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    );
    client.initialize().await.unwrap();
    assert!(client.is_connected());
    client.disconnect().await.unwrap();
    assert!(!client.is_connected());
}

#[tokio::test]
async fn irc_connect_fails_on_unreachable_address() {
    // Port 1 is almost certainly not listening; expect a connection error.
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        1,
        "testbot".to_string(),
    );
    let result = client.initialize().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn irc_send_message_returns_irc_prefixed_id() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    );
    client.initialize().await.unwrap();
    let id = client.send_message("#test", "hello world").await.unwrap();
    assert!(id.contains("irc:"));
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn irc_send_and_receive_echo() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    )
    .with_channels(vec!["#test".to_string()]);

    client.initialize().await.unwrap();
    client.send_message("#test", "ping message").await.unwrap();

    // Brief pause to allow the echo server to process and send the reply.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let msgs = client.receive_messages().await.unwrap();
    assert!(!msgs.is_empty(), "Expected echo reply but got none");
    let echo = &msgs[0];
    assert_eq!(echo.sender, "echo");
    assert!(
        echo.content.contains("ping message"),
        "Echo content '{}' should contain original message",
        echo.content
    );
    assert_eq!(echo.channel, Some("#test".to_string()));

    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn irc_receive_parses_sender_and_channel() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    );
    client.initialize().await.unwrap();
    client
        .send_message("#mychannel", "test content")
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let msgs = client.receive_messages().await.unwrap();
    assert!(!msgs.is_empty());
    assert_eq!(msgs[0].channel, Some("#mychannel".to_string()));
    assert!(!msgs[0].id.is_empty());

    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn irc_with_channels_builder_connects_and_joins() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    )
    .with_channels(vec!["#general".to_string(), "#rust".to_string()]);

    client.initialize().await.unwrap();
    assert!(client.is_connected());
    client.disconnect().await.unwrap();
}

#[tokio::test]
async fn irc_receive_messages_returns_empty_when_no_privmsg() {
    let port = start_irc_echo_server().await;
    let mut client = IrcMessenger::new(
        "test-irc".to_string(),
        "127.0.0.1".to_string(),
        port,
        "testbot".to_string(),
    );
    client.initialize().await.unwrap();
    // Don't send anything — just poll for messages (should time out and return empty).
    let msgs = client.receive_messages().await.unwrap();
    assert!(msgs.is_empty());
    client.disconnect().await.unwrap();
}
