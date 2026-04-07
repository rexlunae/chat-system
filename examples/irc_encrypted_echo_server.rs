//! IRC encrypted echo server example — accepts TLS/SSL IRC connections and echoes messages back.
//!
//! This example demonstrates how to build an IRC server with TLS/SSL encryption
//! using the [`Server`] + [`TlsIrcListener`] builder API.
//!
//! **Setup:**
//!
//! Before running this example, you need to generate a self-signed certificate and key:
//!
//! ```sh
//! # Generate a self-signed certificate (valid for 365 days)
//! openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
//!   -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"
//! ```
//!
//! Then run the example:
//! ```sh
//! cargo run --example irc_encrypted_echo_server --features tls
//! ```
//!
//! To connect from another terminal, use `openssl s_client`:
//! ```sh
//! openssl s_client -connect localhost:6697 -showcerts
//! ```
//! Or use the `irc_encrypted_client` example after modifying it to connect to localhost.

use anyhow::Result;
use chat_system::ChatServer;
use chat_system::server::Server;
use chat_system::servers::TlsIrcListener;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let tls_config = load_tls_config()?;
    let addr = "127.0.0.1:6697";

    let mut server = Server::new("tls-echo-server")
        .add_listener(TlsIrcListener::new(addr, Arc::new(tls_config)));

    println!("IRC TLS echo server listening on {addr} (encrypted)");
    println!("Press Ctrl+C to stop.\n");

    server
        .run(|msg| async move {
            println!(
                "[{}] {}: {}",
                msg.channel.as_deref().unwrap_or("?"),
                msg.sender,
                msg.content
            );
            Ok(Some(format!("[TLS ECHO] {}", msg.content)))
        })
        .await?;

    Ok(())
}

fn load_tls_config() -> Result<rustls::ServerConfig> {
    use std::fs;

    let cert_path = "cert.pem";
    let key_path = "key.pem";

    if !std::path::Path::new(cert_path).exists() || !std::path::Path::new(key_path).exists() {
        eprintln!("ERROR: TLS certificate or key file not found!");
        eprintln!();
        eprintln!("Please generate them with:");
        eprintln!();
        eprintln!(
            "  openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \\"
        );
        eprintln!("    -subj \"/C=US/ST=State/L=City/O=Organization/CN=localhost\"");
        eprintln!();
        anyhow::bail!("Missing TLS certificates");
    }

    let cert_bytes = fs::read(cert_path)?;
    let mut cert_cursor = std::io::Cursor::new(cert_bytes);
    let certs: Vec<CertificateDer> =
        rustls_pemfile::certs(&mut cert_cursor).collect::<Result<Vec<_>, _>>()?;

    let key_bytes = fs::read(key_path)?;
    let mut key_reader = std::io::Cursor::new(key_bytes);
    let key = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .next()
        .ok_or_else(|| anyhow::anyhow!("No PKCS8 private key found"))?
        .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

    let config = rustls::server::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))?;

    Ok(config)
}
