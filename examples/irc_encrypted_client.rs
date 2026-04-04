//! IRC encrypted client example — connect to irc.libera.chat via TLS.
//!
//! This example demonstrates how to use the IRC messenger with TLS/SSL encryption
//! for secure communication. The connection uses port 6697, which is the standard
//! IRC+TLS port.
//!
//! Run with:
//! ```sh
//! cargo run --example irc_encrypted_client
//! ```

use chat_system::messengers::IrcMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read IRC nick from environment or use default
    let nick = std::env::var("IRC_NICK").unwrap_or_else(|_| "chat-system-secure".into());

    // Create IRC messenger with TLS encryption enabled
    // Port 6697 is the standard IRC+TLS port (RFC 7194)
    let mut client = IrcMessenger::new(
        "irc-encrypted-example".into(),
        "irc.libera.chat".into(),
        6697,  // Standard TLS port for IRC
        nick,
    )
    .with_tls(true)  // Enable TLS encryption
    .with_channels(vec!["#rust-chat-test".into()]);

    println!("Connecting to IRC server via TLS...");
    println!("Server: irc.libera.chat:6697");
    println!("Using encrypted connection (TLS 1.2+)");
    println!();

    // Initialize the connection
    client.initialize().await?;
    println!("✓ Connected securely via TLS!");
    println!();

    // Send a message to the channel
    println!("Sending message...");
    client
        .send_message(
            "#rust-chat-test",
            "Hello from chat-system via encrypted IRC!",
        )
        .await?;
    println!("✓ Message sent");
    println!();

    // Listen for incoming messages
    println!("Listening for messages (5 messages or 30 seconds)...");
    println!();
    let mut count = 0;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);

    while count < 5 && tokio::time::Instant::now() < deadline {
        let msgs = client.receive_messages().await?;
        for msg in msgs {
            println!(
                "[{}] {}: {}",
                msg.channel.as_deref().unwrap_or("?"),
                msg.sender,
                msg.content
            );
            count += 1;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!();
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("✓ Disconnected from IRC");
    println!();
    println!("All communication was encrypted via TLS!");

    Ok(())
}
