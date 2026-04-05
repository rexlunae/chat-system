//! IRC echo server example — accepts IRC connections and echoes messages back.
//!
//! Uses the [`Server`] + [`IrcListener`] API from `chat-system` to spin up an
//! IRC server that echoes every received message back to the sender.
//!
//! Run with:
//! ```sh
//! cargo run --example irc_echo_server
//! ```
//!
//! Then connect from another terminal:
//! ```sh
//! cargo run --example irc_client
//! ```

use chat_system::server::Server;
use chat_system::servers::IrcListener;
use chat_system::ChatServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:6667";

    let mut server = Server::new("echo-server")
        .add_listener(IrcListener::new(addr));

    println!("IRC echo server listening on {addr}");
    println!("Press Ctrl+C to stop.\n");

    server
        .run(|msg| async move {
            println!(
                "[{}] {}: {}",
                msg.channel.as_deref().unwrap_or("?"),
                msg.sender,
                msg.content
            );
            Ok(Some(format!("echo: {}", msg.content)))
        })
        .await?;

    Ok(())
}
