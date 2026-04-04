use chat_system::{RichText, RichTextNode};

#[test]
fn plain_text_strips_formatting() {
    let rt = RichText(vec![
        RichTextNode::Bold(vec![RichTextNode::Plain("hello".into())]),
        RichTextNode::Plain(" world".into()),
    ]);
    assert_eq!(rt.to_plain_text(), "hello world");
}

#[test]
fn discord_bold() {
    let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
        "hi".into(),
    )])]);
    assert!(rt.to_discord_markdown().contains("**hi**"));
}

#[test]
fn matrix_html_bold() {
    let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
        "hi".into(),
    )])]);
    assert!(rt.to_matrix_html().contains("<b>hi</b>"));
}

#[test]
fn irc_bold_uses_control_char() {
    let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
        "hi".into(),
    )])]);
    let s = rt.to_irc_formatted();
    assert!(s.contains('\x02'));
}

#[test]
fn whatsapp_bold() {
    let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
        "hi".into(),
    )])]);
    assert!(rt.to_whatsapp_formatted().contains("*hi*"));
}

#[test]
fn from_markdown_bold() {
    let rt = RichText::from_markdown("**bold text**");
    assert!(rt.0.iter().any(|n| matches!(n, RichTextNode::Bold(_))));
}

#[test]
fn discord_link() {
    let rt = RichText(vec![RichTextNode::Link {
        url: "https://example.com".into(),
        text: vec![RichTextNode::Plain("click".into())],
    }]);
    let s = rt.to_discord_markdown();
    assert!(s.contains("[click](https://example.com)"));
}

#[test]
fn slack_link() {
    let rt = RichText(vec![RichTextNode::Link {
        url: "https://example.com".into(),
        text: vec![RichTextNode::Plain("click".into())],
    }]);
    let s = rt.to_slack_mrkdwn();
    assert!(s.contains("<https://example.com|click>"));
}

#[test]
fn matrix_link() {
    let rt = RichText(vec![RichTextNode::Link {
        url: "https://example.com".into(),
        text: vec![RichTextNode::Plain("click".into())],
    }]);
    let s = rt.to_matrix_html();
    assert!(s.contains(r#"href="https://example.com""#));
}
