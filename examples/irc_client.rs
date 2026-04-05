//! IRC client example — connect to irc.libera.chat and listen.

use chat_system::messengers::IrcMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let nick = std::env::var("IRC_NICK").unwrap_or_else(|_| "chat-system-bot".into());
    let mut client = IrcMessenger::new("irc-example", "irc.libera.chat", 6667, nick)
        .with_tls(false)
        .with_channels(vec!["#rust-chat-test"]);

    println!("Connecting to IRC...");
    client.initialize().await?;
    println!("Connected! Sending hello...");

    client
        .send_message("#rust-chat-test", "Hello from chat-system!")
        .await?;

    println!("Listening for 5 messages (or 30 seconds)...");
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

    client.disconnect().await?;
    println!("Disconnected.");
    Ok(())
}
