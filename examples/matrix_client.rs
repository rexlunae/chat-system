//! Matrix client example (requires `matrix` feature).

fn main() {
    #[cfg(not(feature = "matrix"))]
    eprintln!("Run with: cargo run --example matrix_client --features matrix");

    #[cfg(feature = "matrix")]
    {
        use chat_system::messengers::MatrixMessenger;
        use chat_system::Messenger;

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let homeserver =
                std::env::var("MATRIX_HOMESERVER").unwrap_or_else(|_| "https://matrix.org".into());
            let username = std::env::var("MATRIX_USER").expect("Set MATRIX_USER env var");
            let password = std::env::var("MATRIX_PASSWORD").expect("Set MATRIX_PASSWORD env var");

            let mut client = MatrixMessenger::new("matrix-example", homeserver, username, password);
            client.initialize().await.unwrap();
            println!("Matrix connected!");
            client.disconnect().await.unwrap();
        });
    }
}
