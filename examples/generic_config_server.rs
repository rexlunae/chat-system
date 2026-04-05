//! Config-file-driven IRC server — loads [`ServerConfig`] from a JSON file and
//! runs [`GenericServer`].
//!
//! This is the recommended approach when the server topology (number of
//! listeners, addresses, protocols) should be controlled at deployment time
//! rather than compile time.
//!
//! ## Setup
//!
//! Create a `server.json` file in the directory you run the example from:
//!
//! ```json
//! {
//!   "name": "my-server",
//!   "listeners": [
//!     { "protocol": "irc", "address": "127.0.0.1:6667" }
//!   ]
//! }
//! ```
//!
//! If the file does not exist the example writes a sample config and exits so
//! you can review it before starting the server.
//!
//! ## Running
//!
//! ```sh
//! cargo run --example generic_config_server
//! ```
//!
//! Then connect from another terminal:
//!
//! ```sh
//! cargo run --example irc_client
//! ```

use chat_system::{ChatServer, GenericServer, ServerConfig};
use std::path::Path;

const CONFIG_PATH: &str = "server.json";

const SAMPLE_CONFIG: &str = r#"{
  "name": "my-server",
  "listeners": [
    { "protocol": "irc", "address": "127.0.0.1:6667" }
  ]
}
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Load (or scaffold) the config file ───────────────────────────────────
    if !Path::new(CONFIG_PATH).exists() {
        std::fs::write(CONFIG_PATH, SAMPLE_CONFIG)?;
        println!("No {CONFIG_PATH} found — wrote a sample config:");
        println!("{SAMPLE_CONFIG}");
        println!("Edit it as needed, then re-run the example.");
        return Ok(());
    }

    let json = std::fs::read_to_string(CONFIG_PATH)?;
    let config: ServerConfig = serde_json::from_str(&json)?;

    println!("Loaded server config from {CONFIG_PATH}:");
    println!("  name      : {}", config.name());
    for lc in config.listener_configs() {
        println!("  listener  : protocol={} address={}", lc.protocol(), lc.address());
    }
    println!();

    // ── Build and run the server ──────────────────────────────────────────────
    let mut server = GenericServer::new(config);

    println!("Server running.  Press Ctrl+C to stop.\n");

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
