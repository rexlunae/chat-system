//! Generic interface showcase — demonstrates [`GenericMessenger`] and the full
//! [`Messenger`] trait surface using the `console` backend (no network needed).
//!
//! This is the **recommended starting point** for new projects.  Swap the config
//! to switch protocols without changing any application code.
//!
//! Run with:
//!   cargo run --example generic_config_client

use chat_system::{
    config::{ConsoleConfig, IrcConfig, IrcServerConfig, MessengerConfig, ServerConfig},
    GenericMessenger, GenericServer, Messenger, MessengerManager, PresenceStatus, SearchQuery,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== chat-system: generic interface demo ===\n");

    // ── 1. Config serialization / deserialization ─────────────────────────────
    // MessengerConfig is a serde-tagged enum.  The `protocol` field picks the
    // backend.  This means you can store the whole config in TOML / JSON / YAML.
    println!("--- Config round-trip ---");
    let irc_config = MessengerConfig::Irc(IrcConfig {
        name: "irc-bot".into(),
        server: "irc.libera.chat".into(),
        port: 6697,
        nick: "generic-bot".into(),
        channels: vec!["#rust".into()],
        tls: true,
    });

    // Equivalent JSON:
    //   {"protocol":"irc","name":"irc-bot","server":"irc.libera.chat",
    //    "port":6697,"nick":"generic-bot","channels":["#rust"],"tls":true}
    let json = serde_json::to_string_pretty(&irc_config)?;
    println!("Serialized config:\n{json}");

    let decoded: MessengerConfig = serde_json::from_str(&json)?;
    println!(
        "\nRound-trip OK  →  protocol={} name={}\n",
        decoded.protocol_name(),
        decoded.name()
    );

    // ── 2. Core send / receive with the console backend ───────────────────────
    // The console backend writes to stdout and reads from stdin — perfect for
    // demos and tests.  Replace the config to use any other platform.
    println!("--- Core send / receive (console backend) ---");
    let console_cfg = MessengerConfig::Console(ConsoleConfig {
        name: "console-bot".into(),
    });
    let mut bot = GenericMessenger::new(console_cfg);
    bot.initialize().await?;

    bot.send_message("world", "Hello from GenericMessenger!")
        .await?;
    bot.disconnect().await?;

    // ── 3. Presence status ────────────────────────────────────────────────────
    // set_status is a no-op on platforms that don't support it; real platforms
    // (Slack, Discord, Matrix, …) will propagate the status to other users.
    println!("\n--- Presence status ---");
    let console_cfg2 = MessengerConfig::Console(ConsoleConfig {
        name: "status-bot".into(),
    });
    let mut bot2 = GenericMessenger::new(console_cfg2);
    bot2.initialize().await?;

    for status in [
        PresenceStatus::Online,
        PresenceStatus::Away,
        PresenceStatus::Busy,
        PresenceStatus::Invisible,
        PresenceStatus::Offline,
    ] {
        bot2.set_status(status).await?;
        println!("  set_status({status:?}) → ok");
    }

    // ── 4. Text status / custom status message ────────────────────────────────
    // A short human-readable string displayed next to the username on platforms
    // like Slack and Discord.  Separate from the presence indicator above.
    println!("\n--- Text status ---");
    bot2.set_text_status("Building something in Rust 🦀")
        .await?;
    println!("  set_text_status → ok");
    bot2.set_text_status("").await?;
    println!("  cleared text status → ok");

    // ── 5. Reactions ──────────────────────────────────────────────────────────
    // add_reaction / remove_reaction are no-ops on platforms without reaction
    // support (IRC, Console, …).  On Slack, Discord, Matrix they send the emoji.
    println!("\n--- Reactions ---");
    bot2.add_reaction("msg-abc123", "#general", "👍").await?;
    println!("  add_reaction(👍) → ok");
    bot2.add_reaction("msg-abc123", "#general", "🎉").await?;
    println!("  add_reaction(🎉) → ok");
    bot2.remove_reaction("msg-abc123", "#general", "👍").await?;
    println!("  remove_reaction(👍) → ok");

    // ── 6. Profile pictures ───────────────────────────────────────────────────
    println!("\n--- Profile pictures ---");
    let pic = bot2.get_profile_picture("alice").await?;
    println!("  get_profile_picture(\"alice\") → {pic:?}");
    bot2.set_profile_picture("https://example.com/avatar.png")
        .await?;
    println!("  set_profile_picture → ok");

    // ── 7. Message search ─────────────────────────────────────────────────────
    // Returns empty on platforms without server-side search support.
    // On Slack / Discord / Matrix this sends a real search query.
    println!("\n--- Search ---");
    let results = bot2
        .search_messages(SearchQuery {
            text: "hello".into(),
            channel: Some("#general".into()),
            limit: Some(10),
            ..Default::default()
        })
        .await?;
    println!("  search(\"hello\") → {} result(s)", results.len());
    bot2.disconnect().await?;

    // ── 8. MessengerManager — multi-platform dispatch ─────────────────────────
    println!("\n--- MessengerManager (multi-bot) ---");
    let mut mgr = MessengerManager::new();
    for (i, name) in ["alpha", "beta", "gamma"].iter().enumerate() {
        let cfg = MessengerConfig::Console(ConsoleConfig {
            name: (*name).into(),
        });
        let mut gm = GenericMessenger::new(cfg);
        gm.initialize().await?;
        println!("  added messenger #{i}: {name}");
        mgr.add(Box::new(gm));
    }
    mgr.broadcast("world", "broadcast message").await;
    println!(
        "  broadcast → ok (sent to {} messengers)",
        mgr.messengers().len()
    );
    mgr.disconnect_all().await?;

    // ── 9. Server config round-trip ───────────────────────────────────────────
    println!("\n--- Server config round-trip ---");
    let server_cfg = ServerConfig::Irc(IrcServerConfig {
        name: "irc-server".into(),
        binds: vec!["127.0.0.1:16667".into()],
    });
    let server_json = serde_json::to_string_pretty(&server_cfg)?;
    println!("Server config:\n{server_json}");
    println!("bind addresses: {:?}", server_cfg.bind_addresses());

    let _server = GenericServer::new(server_cfg);
    // Uncomment to actually run the server:
    // _server.run(|msg| async move {
    //     println!("received: {}", msg.content);
    //     Ok(Some(format!("echo: {}", msg.content)))
    // }).await?;

    println!("\nDone.");
    Ok(())
}
