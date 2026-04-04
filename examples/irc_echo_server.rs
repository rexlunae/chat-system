//! IRC echo server example — accepts IRC connections and echoes messages back.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:6667";
    let listener = TcpListener::bind(addr).await?;
    println!("IRC echo server listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("New connection from {}", peer);
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                eprintln!("Client error: {}", e);
            }
        });
    }
}

async fn handle_client(stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut nick = "unknown".to_string();
    let mut registered = false;
    let mut user_seen = false;

    while let Some(line) = lines.next_line().await? {
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
            writer.write_all(format!("PONG {}\r\n", token).as_bytes()).await?;
        } else if line.starts_with("PRIVMSG ") {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                let target = parts[1];
                let msg = parts[2].trim_start_matches(':');
                let reply = format!(":echo!echo@localhost PRIVMSG {} :echo: {}\r\n", target, msg);
                writer.write_all(reply.as_bytes()).await?;
            }
        } else if line == "QUIT" || line.starts_with("QUIT ") {
            break;
        }

        if !registered && !nick.is_empty() && user_seen {
            let welcome = format!(":localhost 001 {} :Welcome to the Echo IRC server\r\n", nick);
            writer.write_all(welcome.as_bytes()).await?;
            registered = true;
        }
    }
    Ok(())
}
