//! Discord bot example (REST-based).

use chat_system::messengers::DiscordMessenger;
use chat_system::Messenger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("DISCORD_BOT_TOKEN").expect("Set DISCORD_BOT_TOKEN env var");
    let channel_id = std::env::var("DISCORD_CHANNEL_ID").expect("Set DISCORD_CHANNEL_ID env var");

    let mut bot = DiscordMessenger::new("discord-example".into(), token);
    bot.initialize().await?;
    println!("Discord bot connected!");

    let msg_id = bot
        .send_message(&channel_id, "Hello from chat-system!")
        .await?;
    println!("Sent message ID: {}", msg_id);

    bot.disconnect().await?;
    Ok(())
}
