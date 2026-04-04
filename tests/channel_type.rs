use chat_system::{ChannelType, InboundMode};
use std::str::FromStr;

#[test]
fn all_channel_types_constant_has_12_entries() {
    assert_eq!(ChannelType::ALL.len(), 12);
}

#[test]
fn channel_type_as_str() {
    assert_eq!(ChannelType::Telegram.as_str(), "telegram");
    assert_eq!(ChannelType::Whatsapp.as_str(), "whatsapp");
    assert_eq!(ChannelType::MsTeams.as_str(), "msteams");
    assert_eq!(ChannelType::Discord.as_str(), "discord");
    assert_eq!(ChannelType::Slack.as_str(), "slack");
    assert_eq!(ChannelType::Irc.as_str(), "irc");
    assert_eq!(ChannelType::Matrix.as_str(), "matrix");
    assert_eq!(ChannelType::Signal.as_str(), "signal");
    assert_eq!(ChannelType::GoogleChat.as_str(), "googlechat");
    assert_eq!(ChannelType::IMessage.as_str(), "imessage");
    assert_eq!(ChannelType::Console.as_str(), "console");
    assert_eq!(ChannelType::Webhook.as_str(), "webhook");
}

#[test]
fn channel_type_display_matches_as_str() {
    for ct in ChannelType::ALL {
        assert_eq!(ct.to_string(), ct.as_str());
    }
}

#[test]
fn channel_type_display_name() {
    assert_eq!(ChannelType::Telegram.display_name(), "Telegram");
    assert_eq!(ChannelType::Whatsapp.display_name(), "WhatsApp");
    assert_eq!(ChannelType::MsTeams.display_name(), "Microsoft Teams");
    assert_eq!(ChannelType::Discord.display_name(), "Discord");
    assert_eq!(ChannelType::Slack.display_name(), "Slack");
    assert_eq!(ChannelType::Irc.display_name(), "IRC");
    assert_eq!(ChannelType::Matrix.display_name(), "Matrix");
    assert_eq!(ChannelType::Signal.display_name(), "Signal");
    assert_eq!(ChannelType::GoogleChat.display_name(), "Google Chat");
    assert_eq!(ChannelType::IMessage.display_name(), "iMessage");
    assert_eq!(ChannelType::Console.display_name(), "Console");
    assert_eq!(ChannelType::Webhook.display_name(), "Webhook");
}

#[test]
fn channel_type_from_str_lowercase() {
    assert_eq!(
        ChannelType::from_str("telegram").unwrap(),
        ChannelType::Telegram
    );
    assert_eq!(
        ChannelType::from_str("whatsapp").unwrap(),
        ChannelType::Whatsapp
    );
    assert_eq!(
        ChannelType::from_str("msteams").unwrap(),
        ChannelType::MsTeams
    );
    assert_eq!(
        ChannelType::from_str("discord").unwrap(),
        ChannelType::Discord
    );
    assert_eq!(ChannelType::from_str("slack").unwrap(), ChannelType::Slack);
    assert_eq!(ChannelType::from_str("irc").unwrap(), ChannelType::Irc);
    assert_eq!(
        ChannelType::from_str("matrix").unwrap(),
        ChannelType::Matrix
    );
    assert_eq!(
        ChannelType::from_str("signal").unwrap(),
        ChannelType::Signal
    );
    assert_eq!(
        ChannelType::from_str("googlechat").unwrap(),
        ChannelType::GoogleChat
    );
    assert_eq!(
        ChannelType::from_str("imessage").unwrap(),
        ChannelType::IMessage
    );
    assert_eq!(
        ChannelType::from_str("console").unwrap(),
        ChannelType::Console
    );
    assert_eq!(
        ChannelType::from_str("webhook").unwrap(),
        ChannelType::Webhook
    );
}

#[test]
fn channel_type_from_str_aliases() {
    assert_eq!(
        ChannelType::from_str("teams").unwrap(),
        ChannelType::MsTeams
    );
    assert_eq!(
        ChannelType::from_str("microsoft teams").unwrap(),
        ChannelType::MsTeams
    );
    assert_eq!(
        ChannelType::from_str("google chat").unwrap(),
        ChannelType::GoogleChat
    );
}

#[test]
fn channel_type_from_str_is_case_insensitive() {
    assert_eq!(
        ChannelType::from_str("TELEGRAM").unwrap(),
        ChannelType::Telegram
    );
    assert_eq!(
        ChannelType::from_str("Discord").unwrap(),
        ChannelType::Discord
    );
    assert_eq!(ChannelType::from_str("SLACK").unwrap(), ChannelType::Slack);
    assert_eq!(ChannelType::from_str("IRC").unwrap(), ChannelType::Irc);
}

#[test]
fn channel_type_from_str_unknown_fails() {
    assert!(ChannelType::from_str("nonexistent").is_err());
    assert!(ChannelType::from_str("").is_err());
    assert!(ChannelType::from_str("chat").is_err());
}

#[test]
fn channel_type_serialization_roundtrip() {
    for ct in ChannelType::ALL {
        let json = serde_json::to_string(ct).unwrap();
        let de: ChannelType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, *ct);
    }
}

#[test]
fn channel_type_msteams_serializes_as_msteams() {
    let json = serde_json::to_string(&ChannelType::MsTeams).unwrap();
    assert_eq!(json, "\"msteams\"");
}

#[test]
fn telegram_capabilities() {
    let d = ChannelType::Telegram.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::Polling);
    assert!(d.capabilities.supports_outbound);
    assert!(d.capabilities.supports_otp);
    assert!(d.capabilities.supports_location);
    assert!(d.capabilities.supports_voice_ingest);
    assert!(!d.capabilities.supports_threads);
    assert!(!d.capabilities.supports_reactions);
    assert!(!d.capabilities.supports_interactive);
    assert_eq!(d.display_name, "Telegram");
    assert_eq!(d.channel_type, ChannelType::Telegram);
}

#[test]
fn discord_capabilities() {
    let d = ChannelType::Discord.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::GatewayLoop);
    assert!(d.capabilities.supports_threads);
    assert!(d.capabilities.supports_interactive);
    assert!(d.capabilities.supports_outbound);
    assert!(d.capabilities.supports_streaming);
    assert!(d.capabilities.supports_location);
    assert!(!d.capabilities.supports_reactions);
}

#[test]
fn slack_capabilities() {
    let d = ChannelType::Slack.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::SocketMode);
    assert!(d.capabilities.supports_threads);
    assert!(d.capabilities.supports_reactions);
    assert!(d.capabilities.supports_interactive);
    assert!(!d.capabilities.supports_location);
    assert!(!d.capabilities.supports_voice_ingest);
}

#[test]
fn irc_capabilities() {
    let d = ChannelType::Irc.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::GatewayLoop);
    assert!(d.capabilities.supports_outbound);
    assert!(!d.capabilities.supports_threads);
    assert!(!d.capabilities.supports_reactions);
    assert!(!d.capabilities.supports_streaming);
    assert!(!d.capabilities.supports_interactive);
}

#[test]
fn matrix_capabilities() {
    let d = ChannelType::Matrix.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::GatewayLoop);
    assert!(d.capabilities.supports_threads);
    assert!(d.capabilities.supports_reactions);
    assert!(d.capabilities.supports_streaming);
}

#[test]
fn whatsapp_capabilities() {
    let d = ChannelType::Whatsapp.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::GatewayLoop);
    assert!(d.capabilities.supports_pairing);
    assert!(d.capabilities.supports_otp);
    assert!(d.capabilities.supports_voice_ingest);
}

#[test]
fn signal_capabilities() {
    let d = ChannelType::Signal.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::GatewayLoop);
    assert!(d.capabilities.supports_voice_ingest);
    assert!(!d.capabilities.supports_threads);
}

#[test]
fn msteams_capabilities() {
    let d = ChannelType::MsTeams.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::Webhook);
    assert!(d.capabilities.supports_outbound);
    assert!(d.capabilities.supports_location);
    assert!(!d.capabilities.supports_reactions);
}

#[test]
fn google_chat_capabilities() {
    let d = ChannelType::GoogleChat.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::Webhook);
    assert!(d.capabilities.supports_outbound);
    assert!(!d.capabilities.supports_streaming);
}

#[test]
fn imessage_capabilities() {
    let d = ChannelType::IMessage.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::Polling);
    assert!(d.capabilities.supports_outbound);
    assert!(!d.capabilities.supports_threads);
}

#[test]
fn console_capabilities() {
    let d = ChannelType::Console.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::None);
    assert!(d.capabilities.supports_outbound);
    assert!(!d.capabilities.supports_streaming);
    assert!(!d.capabilities.supports_reactions);
}

#[test]
fn webhook_capabilities() {
    let d = ChannelType::Webhook.descriptor();
    assert_eq!(d.capabilities.inbound_mode, InboundMode::None);
    assert!(d.capabilities.supports_outbound);
    assert!(!d.capabilities.supports_streaming);
}

#[test]
fn descriptor_channel_type_and_display_name_consistent() {
    for ct in ChannelType::ALL {
        let descriptor = ct.descriptor();
        assert_eq!(descriptor.display_name, ct.display_name());
        assert_eq!(descriptor.channel_type, *ct);
    }
}

#[test]
fn all_channel_types_support_outbound() {
    for ct in ChannelType::ALL {
        let d = ct.descriptor();
        assert!(
            d.capabilities.supports_outbound,
            "{} should support outbound",
            ct.as_str()
        );
    }
}
