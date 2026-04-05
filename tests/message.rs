use chat_system::{MediaAttachment, Message, Reaction, SendOptions};

#[test]
fn message_creation() {
    let msg = Message {
        id: "123".to_string(),
        sender: "alice".to_string(),
        content: "Hello".to_string(),
        timestamp: 1_000_000,
        channel: Some("#general".to_string()),
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: None,
    };
    assert_eq!(msg.id, "123");
    assert_eq!(msg.sender, "alice");
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.timestamp, 1_000_000);
    assert_eq!(msg.channel, Some("#general".to_string()));
    assert!(msg.reply_to.is_none());
    assert!(msg.media.is_none());
    assert!(!msg.is_direct);
}

#[test]
fn message_clone() {
    let msg = Message {
        id: "1".to_string(),
        sender: "alice".to_string(),
        content: "hi".to_string(),
        timestamp: 1000,
        channel: None,
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: None,
    };
    let cloned = msg.clone();
    assert_eq!(msg.id, cloned.id);
    assert_eq!(msg.sender, cloned.sender);
    assert_eq!(msg.content, cloned.content);
    assert_eq!(msg.timestamp, cloned.timestamp);
}

#[test]
fn message_serialization_roundtrip() {
    let msg = Message {
        id: "456".to_string(),
        sender: "bob".to_string(),
        content: "Hi".to_string(),
        timestamp: 2000,
        channel: None,
        reply_to: Some("123".to_string()),
        media: None,
        is_direct: true,
        reactions: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let de: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, msg.id);
    assert_eq!(de.sender, msg.sender);
    assert_eq!(de.content, msg.content);
    assert_eq!(de.timestamp, msg.timestamp);
    assert!(de.channel.is_none());
    assert_eq!(de.reply_to, Some("123".to_string()));
    assert!(de.is_direct);
}

#[test]
fn message_defaults_for_optional_fields() {
    let json = r#"{"id":"1","sender":"x","content":"y","timestamp":0}"#;
    let msg: Message = serde_json::from_str(json).unwrap();
    assert!(msg.channel.is_none());
    assert!(msg.reply_to.is_none());
    assert!(msg.media.is_none());
    assert!(!msg.is_direct);
    assert!(msg.reactions.is_none());
}

#[test]
fn message_with_media() {
    let msg = Message {
        id: "789".to_string(),
        sender: "charlie".to_string(),
        content: "See attached".to_string(),
        timestamp: 3000,
        channel: Some("#media".to_string()),
        reply_to: None,
        media: Some(vec![MediaAttachment {
            url: Some("https://example.com/file.pdf".to_string()),
            path: None,
            mime_type: Some("application/pdf".to_string()),
            filename: Some("file.pdf".to_string()),
        }]),
        is_direct: false,
        reactions: None,
    };
    let media = msg.media.as_ref().unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].filename, Some("file.pdf".to_string()));
    assert_eq!(media[0].mime_type, Some("application/pdf".to_string()));
}

#[test]
fn media_attachment_creation() {
    let a = MediaAttachment {
        url: Some("https://example.com/img.png".to_string()),
        path: Some("/tmp/img.png".to_string()),
        mime_type: Some("image/png".to_string()),
        filename: Some("img.png".to_string()),
    };
    assert_eq!(a.url, Some("https://example.com/img.png".to_string()));
    assert_eq!(a.path, Some("/tmp/img.png".to_string()));
    assert_eq!(a.mime_type, Some("image/png".to_string()));
    assert_eq!(a.filename, Some("img.png".to_string()));
}

#[test]
fn media_attachment_all_optional_fields() {
    let a = MediaAttachment {
        url: None,
        path: None,
        mime_type: None,
        filename: None,
    };
    assert!(a.url.is_none());
    assert!(a.path.is_none());
    assert!(a.mime_type.is_none());
    assert!(a.filename.is_none());
}

#[test]
fn media_attachment_serialization_roundtrip() {
    let a = MediaAttachment {
        url: Some("https://example.com/img.png".to_string()),
        path: None,
        mime_type: Some("image/png".to_string()),
        filename: Some("img.png".to_string()),
    };
    let json = serde_json::to_string(&a).unwrap();
    let de: MediaAttachment = serde_json::from_str(&json).unwrap();
    assert_eq!(de.url, a.url);
    assert_eq!(de.mime_type, a.mime_type);
    assert_eq!(de.filename, a.filename);
}

#[test]
fn send_options_default() {
    let opts = SendOptions {
        recipient: "alice",
        content: "hello",
        ..Default::default()
    };
    assert_eq!(opts.recipient, "alice");
    assert_eq!(opts.content, "hello");
    assert!(opts.reply_to.is_none());
    assert!(!opts.silent);
    assert!(opts.media.is_none());
}

#[test]
fn send_options_full() {
    let opts = SendOptions {
        recipient: "#channel",
        content: "Hello channel!",
        reply_to: Some("123"),
        silent: true,
        media: Some("https://example.com/img.png"),
    };
    assert_eq!(opts.recipient, "#channel");
    assert_eq!(opts.content, "Hello channel!");
    assert_eq!(opts.reply_to, Some("123"));
    assert!(opts.silent);
    assert_eq!(opts.media, Some("https://example.com/img.png"));
}

#[test]
fn message_is_direct_flag() {
    let dm = Message {
        id: "d1".to_string(),
        sender: "dave".to_string(),
        content: "private".to_string(),
        timestamp: 4000,
        channel: None,
        reply_to: None,
        media: None,
        is_direct: true,
        reactions: None,
    };
    assert!(dm.is_direct);
}

#[test]
fn reaction_creation_and_clone() {
    let r = Reaction {
        emoji: "👍".to_string(),
        count: 3,
        user_ids: vec!["alice".to_string(), "bob".to_string(), "carol".to_string()],
    };
    let r2 = r.clone();
    assert_eq!(r.emoji, r2.emoji);
    assert_eq!(r.count, r2.count);
    assert_eq!(r.user_ids, r2.user_ids);
}

#[test]
fn reaction_serialization_roundtrip() {
    let r = Reaction {
        emoji: "❤️".to_string(),
        count: 1,
        user_ids: vec!["dave".to_string()],
    };
    let json = serde_json::to_string(&r).unwrap();
    let de: Reaction = serde_json::from_str(&json).unwrap();
    assert_eq!(de.emoji, r.emoji);
    assert_eq!(de.count, r.count);
    assert_eq!(de.user_ids, r.user_ids);
}

#[test]
fn reaction_user_ids_default_empty() {
    let json = r#"{"emoji":"👍","count":5}"#;
    let r: Reaction = serde_json::from_str(json).unwrap();
    assert_eq!(r.emoji, "👍");
    assert_eq!(r.count, 5);
    assert!(r.user_ids.is_empty());
}

#[test]
fn message_with_reactions() {
    let msg = Message {
        id: "r1".to_string(),
        sender: "alice".to_string(),
        content: "Great idea!".to_string(),
        timestamp: 5000,
        channel: Some("#general".to_string()),
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: Some(vec![
            Reaction { emoji: "👍".to_string(), count: 2, user_ids: vec![] },
            Reaction { emoji: "🎉".to_string(), count: 1, user_ids: vec!["bob".to_string()] },
        ]),
    };
    let reactions = msg.reactions.as_ref().unwrap();
    assert_eq!(reactions.len(), 2);
    assert_eq!(reactions[0].emoji, "👍");
    assert_eq!(reactions[0].count, 2);
    assert_eq!(reactions[1].emoji, "🎉");
    assert_eq!(reactions[1].user_ids, vec!["bob".to_string()]);
}

#[test]
fn message_reactions_serialization_roundtrip() {
    let msg = Message {
        id: "r2".to_string(),
        sender: "bob".to_string(),
        content: "Hello".to_string(),
        timestamp: 6000,
        channel: None,
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: Some(vec![Reaction {
            emoji: "❤️".to_string(),
            count: 3,
            user_ids: vec!["alice".to_string()],
        }]),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let de: Message = serde_json::from_str(&json).unwrap();
    let reactions = de.reactions.as_ref().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].emoji, "❤️");
    assert_eq!(reactions[0].count, 3);
}

