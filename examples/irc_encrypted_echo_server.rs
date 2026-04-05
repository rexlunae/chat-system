//! IRC encrypted echo server example — accepts TLS/SSL IRC connections and echoes messages back.
//!
//! This example demonstrates how to build an IRC server with TLS/SSL encryption support.
//! It uses the `tokio_rustls` crate for TLS termination and echoes back received messages.
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
//! cargo run --example irc_encrypted_echo_server
//! ```
//!
//! To connect from another terminal, use `openssl s_client`:
//! ```sh
//! openssl s_client -connect localhost:6697 -showcerts
//! ```
//! Or use the `irc_encrypted_client` example after modifying it to connect to localhost.

use anyhow::Result;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

#[tokio::main]
async fn main() -> Result<()> {
    // Load TLS certificate and key
    let tls_config = load_tls_config().await?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr = "127.0.0.1:6697";
    let listener = TcpListener::bind(addr).await?;
    println!("IRC TLS echo server listening on {} (encrypted)", addr);
    println!("Certificate: cert.pem");
    println!("Key: key.pem");
    println!();

    loop {
        let (stream, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        println!("[TLS] New connection from {}", peer);

        tokio::spawn(async move {
            match handle_tls_client(stream, acceptor).await {
                Ok(_) => println!("[TLS] Client {} disconnected gracefully", peer),
                Err(e) => eprintln!("[TLS] Client {} error: {}", peer, e),
            }
        });
    }
}

async fn handle_tls_client(stream: tokio::net::TcpStream, acceptor: TlsAcceptor) -> Result<()> {
    // Perform TLS handshake
    let tls_stream = acceptor.accept(stream).await?;
    let (reader, mut writer) = tokio::io::split(tls_stream);
    let mut lines = BufReader::new(reader).lines();
    let mut nick = "unknown".to_string();
    let mut registered = false;
    let mut user_seen = false;

    // Send security notice
    writer
        .write_all(b":server NOTICE AUTH :*** This is a TLS-encrypted IRC server ***\r\n")
        .await?;

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
            writer
                .write_all(format!("PONG {}\r\n", token).as_bytes())
                .await?;
        } else if line.starts_with("PRIVMSG ") {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                let target = parts[1];
                let msg = parts[2].trim_start_matches(':');
                let reply = format!(
                    ":echo!echo@localhost PRIVMSG {} :[TLS ECHO] {}\r\n",
                    target, msg
                );
                writer.write_all(reply.as_bytes()).await?;
            }
        } else if line == "QUIT" || line.starts_with("QUIT ") {
            break;
        }

        // Send welcome message after NICK and USER received
        if !registered && !nick.is_empty() && user_seen {
            let welcome = format!(
                ":localhost 001 {} :Welcome to the Encrypted Echo IRC Server\r\n",
                nick
            );
            writer.write_all(welcome.as_bytes()).await?;
            let motd = format!(
                ":localhost NOTICE {} :Your connection is encrypted with TLS\r\n",
                nick
            );
            writer.write_all(motd.as_bytes()).await?;
            registered = true;
        }
    }
    Ok(())
}

async fn load_tls_config() -> Result<rustls::server::ServerConfig> {
    use std::fs;

    // Try to load certificate and key files
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

    // Read certificate
    let cert_bytes = fs::read(cert_path)?;
    let mut cert_cursor = std::io::Cursor::new(cert_bytes);
    let certs: Vec<CertificateDer> =
        rustls_pemfile::certs(&mut cert_cursor).collect::<Result<Vec<_>, _>>()?;

    // Read private key
    let key_bytes = fs::read(key_path)?;
    let mut key_reader = std::io::Cursor::new(key_bytes);

    // Try to read PKCS8 keys first
    let key = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .next()
        .ok_or_else(|| anyhow::anyhow!("No PKCS8 private key found"))?
        .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

    let config = rustls::server::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))?;

    Ok(config)
}
