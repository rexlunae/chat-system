//! Demonstrates config-driven messenger and server setup using [`GenericMessenger`]
//! and [`GenericServer`].
//!
//! In a real application the config would be loaded from a TOML / JSON / YAML
//! file; here we construct it programmatically to keep the example self-contained.
//!
//! Run with:
//!   cargo run --example generic_config_client

use chat_system::{
    config::{
        ConsoleConfig, IrcConfig, IrcServerConfig, MessengerConfig, ServerConfig,
    },
    GenericMessenger, GenericServer, Messenger,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── client ────────────────────────────────────────────────────────────────
    // A MessengerConfig can be deserialized from any serde-compatible source.
    // Here we build one directly; the JSON equivalent would be:
    //   {"protocol":"irc","name":"bot","server":"...","nick":"bot","port":6667}
    let client_config = MessengerConfig::Irc(IrcConfig {
        name: "irc-bot".into(),
        server: "irc.libera.chat".into(),
        port: 6667,
        nick: "generic-bot".into(),
        channels: vec!["#rust-chat-test".into()],
        tls: false,
    });

    println!("protocol : {}", client_config.protocol_name());
    println!("name     : {}", client_config.name());

    // Serialize to JSON to demonstrate config-file round-trip.
    let json = serde_json::to_string_pretty(&client_config)?;
    println!("\nSerialized config:\n{json}");

    // Deserialize back.
    let decoded: MessengerConfig = serde_json::from_str(&json)?;
    println!("\nDeserialized protocol: {}", decoded.protocol_name());

    // GenericMessenger implements Messenger — protocol is just a runtime setting.
    let _client = GenericMessenger::new(client_config);
    // Uncomment to actually connect:
    // _client.initialize().await?;
    // _client.send_message("#rust-chat-test", "Hello from GenericMessenger!").await?;
    // _client.disconnect().await?;

    // ── console client (no network needed) ────────────────────────────────────
    let console_config = MessengerConfig::Console(ConsoleConfig { name: "console".into() });
    let mut console = GenericMessenger::new(console_config);
    console.initialize().await?;
    console.send_message("world", "Hello from GenericMessenger!").await?;
    console.disconnect().await?;

    // ── server ────────────────────────────────────────────────────────────────
    let server_config = ServerConfig::Irc(IrcServerConfig {
        name: "irc-server".into(),
        bind: "127.0.0.1:16667".into(),
    });

    println!("\nServer bind address: {}", server_config.bind_address());

    let server_json = serde_json::to_string_pretty(&server_config)?;
    println!("Serialized server config:\n{server_json}");

    let _server = GenericServer::new(server_config);
    // Uncomment to actually run:
    // _server.run(|msg| async move {
    //     println!("Received: {}", msg.content);
    //     Ok(Some(format!("echo: {}", msg.content)))
    // }).await?;

    Ok(())
}
