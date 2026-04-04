//! State-management tests for `WhatsAppMessenger` (requires `whatsapp` feature).

#[cfg(feature = "whatsapp")]
mod tests {
    use chat_system::messengers::WhatsAppMessenger;
    use chat_system::Messenger;

    #[tokio::test]
    async fn whatsapp_name_and_type() {
        let m = WhatsAppMessenger::new(
            "my-whatsapp".to_string(),
            "/tmp/test_wa_name_type.db".to_string(),
        );
        assert_eq!(m.name(), "my-whatsapp");
        assert_eq!(m.messenger_type(), "whatsapp");
    }

    #[tokio::test]
    async fn whatsapp_not_connected_before_initialize() {
        let m = WhatsAppMessenger::new(
            "wa".to_string(),
            "/tmp/test_wa_not_connected.db".to_string(),
        );
        assert!(!m.is_connected());
    }

    #[tokio::test]
    async fn whatsapp_receive_messages_returns_empty_without_initialize() {
        let m = WhatsAppMessenger::new(
            "wa".to_string(),
            "/tmp/test_wa_receive_empty.db".to_string(),
        );
        let msgs = m.receive_messages().await.unwrap();
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn whatsapp_disconnect_without_init_is_ok() {
        let mut m = WhatsAppMessenger::new(
            "wa".to_string(),
            "/tmp/test_wa_disconnect_noinit.db".to_string(),
        );
        m.disconnect().await.unwrap();
        assert!(!m.is_connected());
    }

    #[tokio::test]
    async fn whatsapp_send_message_without_init_returns_err() {
        let m = WhatsAppMessenger::new("wa".to_string(), "/tmp/test_wa_send_noinit.db".to_string());
        let result = m.send_message("15551234567", "hello").await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not initialised"),
            "Error should mention initialisation"
        );
    }
}
