//! Rich text representation and format conversion.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// A node in a rich text tree.
///
/// Inspired by formatting features across Discord, Telegram, Matrix, Slack,
/// and IRC — this enum covers the union of formatting primitives used by all
/// major chat platforms.
#[derive(Debug, Clone)]
pub enum RichTextNode {
    Plain(String),
    Bold(Vec<RichTextNode>),
    Italic(Vec<RichTextNode>),
    Underline(Vec<RichTextNode>),
    Strikethrough(Vec<RichTextNode>),
    /// Spoiler / hidden text (Discord `||text||`, Telegram `<tg-spoiler>`).
    Spoiler(Vec<RichTextNode>),
    Code(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    /// Block quote (Markdown `>`, Discord `> text`, Telegram `<blockquote>`).
    Blockquote(Vec<RichTextNode>),
    /// Heading (levels 1–6).
    Heading {
        level: u8,
        children: Vec<RichTextNode>,
    },
    Link {
        url: String,
        text: Vec<RichTextNode>,
    },
    Mention {
        id: String,
        name: String,
    },
    Emoji(String),
    Paragraph(Vec<RichTextNode>),
    ListItem(Vec<RichTextNode>),
}

/// A rich text document as a sequence of nodes.
pub struct RichText(pub Vec<RichTextNode>);

impl RichTextNode {
    fn to_plain_text(&self) -> String {
        match self {
            RichTextNode::Plain(s) => s.clone(),
            RichTextNode::Bold(children)
            | RichTextNode::Italic(children)
            | RichTextNode::Underline(children)
            | RichTextNode::Strikethrough(children)
            | RichTextNode::Spoiler(children)
            | RichTextNode::Blockquote(children)
            | RichTextNode::Paragraph(children)
            | RichTextNode::ListItem(children) => {
                children.iter().map(|n| n.to_plain_text()).collect()
            }
            RichTextNode::Heading { children, .. } => {
                children.iter().map(|n| n.to_plain_text()).collect()
            }
            RichTextNode::Code(s) => s.clone(),
            RichTextNode::CodeBlock { code, .. } => code.clone(),
            RichTextNode::Link { text, .. } => text.iter().map(|n| n.to_plain_text()).collect(),
            RichTextNode::Mention { name, .. } => format!("@{}", name),
            RichTextNode::Emoji(e) => e.clone(),
        }
    }

    fn to_markdown(&self) -> String {
        match self {
            RichTextNode::Plain(s) => s.clone(),
            RichTextNode::Bold(children) => {
                format!(
                    "**{}**",
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Italic(children) => {
                format!(
                    "*{}*",
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Underline(children) => {
                // CommonMark has no underline; use <u> HTML
                format!(
                    "<u>{}</u>",
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Strikethrough(children) => {
                format!(
                    "~~{}~~",
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Spoiler(children) => {
                // Discord-style spoiler tags
                format!(
                    "||{}||",
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Code(s) => format!("`{}`", s),
            RichTextNode::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    format!("```{}\n{}\n```", lang, code)
                } else {
                    format!("```\n{}\n```", code)
                }
            }
            RichTextNode::Blockquote(children) => {
                let inner: String = children.iter().map(|n| n.to_markdown()).collect();
                inner
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            RichTextNode::Heading { level, children } => {
                let hashes = "#".repeat((*level).min(6) as usize);
                format!(
                    "{} {}",
                    hashes,
                    children.iter().map(|n| n.to_markdown()).collect::<String>()
                )
            }
            RichTextNode::Link { url, text } => {
                format!(
                    "[{}]({})",
                    text.iter().map(|n| n.to_markdown()).collect::<String>(),
                    url
                )
            }
            RichTextNode::Mention { name, .. } => format!("@{}", name),
            RichTextNode::Emoji(e) => e.clone(),
            RichTextNode::Paragraph(children) | RichTextNode::ListItem(children) => {
                children.iter().map(|n| n.to_markdown()).collect()
            }
        }
    }

    fn to_matrix_html(&self) -> String {
        match self {
            RichTextNode::Plain(s) => html_escape(s),
            RichTextNode::Bold(children) => {
                format!(
                    "<b>{}</b>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Italic(children) => {
                format!(
                    "<i>{}</i>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Underline(children) => {
                format!(
                    "<u>{}</u>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Strikethrough(children) => {
                format!(
                    "<del>{}</del>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Spoiler(children) => {
                format!(
                    "<span data-mx-spoiler>{}</span>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Code(s) => format!("<code>{}</code>", html_escape(s)),
            RichTextNode::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    format!(
                        "<pre><code class=\"language-{}\">{}</code></pre>",
                        lang,
                        html_escape(code)
                    )
                } else {
                    format!("<pre>{}</pre>", html_escape(code))
                }
            }
            RichTextNode::Blockquote(children) => {
                format!(
                    "<blockquote>{}</blockquote>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Heading { level, children } => {
                let tag = format!("h{}", (*level).min(6));
                format!(
                    "<{tag}>{}</{tag}>",
                    children
                        .iter()
                        .map(|n| n.to_matrix_html())
                        .collect::<String>()
                )
            }
            RichTextNode::Link { url, text } => {
                format!(
                    "<a href=\"{}\">{}</a>",
                    html_escape(url),
                    text.iter().map(|n| n.to_matrix_html()).collect::<String>()
                )
            }
            RichTextNode::Mention { name, .. } => format!("@{}", html_escape(name)),
            RichTextNode::Emoji(e) => html_escape(e),
            RichTextNode::Paragraph(children) | RichTextNode::ListItem(children) => {
                children.iter().map(|n| n.to_matrix_html()).collect()
            }
        }
    }

    fn to_irc_formatted(&self) -> String {
        match self {
            RichTextNode::Plain(s) => s.clone(),
            RichTextNode::Bold(children) => {
                format!(
                    "\x02{}\x02",
                    children
                        .iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Italic(children) => {
                format!(
                    "\x1D{}\x1D",
                    children
                        .iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Underline(children) => {
                // IRC underline control code: \x1F
                format!(
                    "\x1F{}\x1F",
                    children
                        .iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Strikethrough(children) => {
                // IRC has no strikethrough; render plain
                children.iter().map(|n| n.to_irc_formatted()).collect()
            }
            RichTextNode::Spoiler(children) => {
                // IRC has no native spoiler; render as [spoiler] placeholder
                format!(
                    "[spoiler: {}]",
                    children
                        .iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Code(s) => format!("`{}`", s),
            RichTextNode::CodeBlock { code, .. } => code.clone(),
            RichTextNode::Blockquote(children) => {
                let inner: String = children.iter().map(|n| n.to_irc_formatted()).collect();
                inner
                    .lines()
                    .map(|line| format!("| {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            RichTextNode::Heading { children, .. } => {
                // Render headings as bold on IRC
                format!(
                    "\x02{}\x02",
                    children
                        .iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Link { url, text } => {
                format!(
                    "{} ({})",
                    text.iter()
                        .map(|n| n.to_irc_formatted())
                        .collect::<String>(),
                    url
                )
            }
            RichTextNode::Mention { name, .. } => format!("@{}", name),
            RichTextNode::Emoji(e) => e.clone(),
            RichTextNode::Paragraph(children) | RichTextNode::ListItem(children) => {
                children.iter().map(|n| n.to_irc_formatted()).collect()
            }
        }
    }

    fn to_whatsapp_formatted(&self) -> String {
        match self {
            RichTextNode::Plain(s) => s.clone(),
            RichTextNode::Bold(children) => {
                format!(
                    "*{}*",
                    children
                        .iter()
                        .map(|n| n.to_whatsapp_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Italic(children) => {
                format!(
                    "_{}_",
                    children
                        .iter()
                        .map(|n| n.to_whatsapp_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Underline(children) => {
                // WhatsApp has no underline; render plain
                children.iter().map(|n| n.to_whatsapp_formatted()).collect()
            }
            RichTextNode::Strikethrough(children) => {
                format!(
                    "~{}~",
                    children
                        .iter()
                        .map(|n| n.to_whatsapp_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Spoiler(children) => {
                // WhatsApp has no spoiler; render plain
                children.iter().map(|n| n.to_whatsapp_formatted()).collect()
            }
            RichTextNode::Code(s) => format!("`{}`", s),
            RichTextNode::CodeBlock { code, .. } => format!("```{}```", code),
            RichTextNode::Blockquote(children) => {
                let inner: String = children.iter().map(|n| n.to_whatsapp_formatted()).collect();
                inner
                    .lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            RichTextNode::Heading { children, .. } => {
                // WhatsApp has no heading; render as bold
                format!(
                    "*{}*",
                    children
                        .iter()
                        .map(|n| n.to_whatsapp_formatted())
                        .collect::<String>()
                )
            }
            RichTextNode::Link { url, text } => {
                format!(
                    "{} ({})",
                    text.iter()
                        .map(|n| n.to_whatsapp_formatted())
                        .collect::<String>(),
                    url
                )
            }
            RichTextNode::Mention { name, .. } => format!("@{}", name),
            RichTextNode::Emoji(e) => e.clone(),
            RichTextNode::Paragraph(children) | RichTextNode::ListItem(children) => {
                children.iter().map(|n| n.to_whatsapp_formatted()).collect()
            }
        }
    }
}

fn html_escape(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

impl RichText {
    /// Convert to plain text (strips all formatting).
    pub fn to_plain_text(&self) -> String {
        self.0.iter().map(|n| n.to_plain_text()).collect()
    }

    /// Convert to standard CommonMark markdown.
    pub fn to_markdown(&self) -> String {
        self.0.iter().map(|n| n.to_markdown()).collect()
    }

    /// Convert to Discord markdown.
    ///
    /// Discord extends CommonMark with `||spoiler||` and `__underline__`.
    pub fn to_discord_markdown(&self) -> String {
        fn discord_node(node: &RichTextNode) -> String {
            match node {
                RichTextNode::Underline(children) => {
                    format!(
                        "__{}__",
                        children.iter().map(discord_node).collect::<String>()
                    )
                }
                RichTextNode::Spoiler(children) => {
                    format!(
                        "||{}||",
                        children.iter().map(discord_node).collect::<String>()
                    )
                }
                // For all other nodes, the standard markdown is correct for Discord.
                other => other.to_markdown(),
            }
        }
        self.0.iter().map(discord_node).collect()
    }

    /// Convert to Telegram HTML.
    pub fn to_telegram_html(&self) -> String {
        crate::markdown::markdown_to_telegram_html(self.to_markdown())
    }

    /// Convert to Slack mrkdwn.
    pub fn to_slack_mrkdwn(&self) -> String {
        crate::markdown::markdown_to_slack(self.to_markdown())
    }

    /// Convert to Matrix HTML.
    pub fn to_matrix_html(&self) -> String {
        self.0.iter().map(|n| n.to_matrix_html()).collect()
    }

    /// Convert to IRC formatted text (bold=`\x02`, italic=`\x1D`).
    pub fn to_irc_formatted(&self) -> String {
        self.0.iter().map(|n| n.to_irc_formatted()).collect()
    }

    /// Convert to WhatsApp formatted text.
    pub fn to_whatsapp_formatted(&self) -> String {
        self.0.iter().map(|n| n.to_whatsapp_formatted()).collect()
    }

    /// Create from plain text.
    pub fn from_plain(text: impl AsRef<str>) -> Self {
        Self(vec![RichTextNode::Plain(text.as_ref().to_string())])
    }

    /// Parse from Markdown using pulldown-cmark.
    pub fn from_markdown(text: impl AsRef<str>) -> Self {
        let text = text.as_ref();
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(text, opts);

        let mut stack: Vec<Vec<RichTextNode>> = vec![vec![]];

        for event in parser {
            match event {
                Event::Start(Tag::Strong)
                | Event::Start(Tag::Emphasis)
                | Event::Start(Tag::Strikethrough) => {
                    stack.push(vec![]);
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    stack.push(vec![]);
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    // Push a sentinel with the heading level, then a child accumulator
                    let level_u8 = match level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                    stack.push(vec![RichTextNode::Plain(level_u8.to_string())]);
                    stack.push(vec![]);
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    stack.push(vec![RichTextNode::Plain(dest_url.to_string())]);
                    stack.push(vec![]);
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                            Some(lang.to_string())
                        }
                        _ => None,
                    };
                    stack.push(vec![RichTextNode::Plain(lang.unwrap_or_default())]);
                    stack.push(vec![]);
                }
                Event::End(TagEnd::Strong) => {
                    let children = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Bold(children));
                    }
                }
                Event::End(TagEnd::Emphasis) => {
                    let children = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Italic(children));
                    }
                }
                Event::End(TagEnd::Strikethrough) => {
                    let children = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Strikethrough(children));
                    }
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    let children = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Blockquote(children));
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    let children = stack.pop().unwrap_or_default();
                    let level_node = stack.pop().unwrap_or_default();
                    let level: u8 =
                        if let Some(RichTextNode::Plain(l)) = level_node.into_iter().next() {
                            l.parse().unwrap_or(1)
                        } else {
                            1
                        };
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Heading { level, children });
                    }
                }
                Event::End(TagEnd::Link) => {
                    let link_text = stack.pop().unwrap_or_default();
                    let url_node = stack.pop().unwrap_or_default();
                    let url = if let Some(RichTextNode::Plain(u)) = url_node.into_iter().next() {
                        u
                    } else {
                        String::new()
                    };
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Link {
                            url,
                            text: link_text,
                        });
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    let code_nodes = stack.pop().unwrap_or_default();
                    let lang_node = stack.pop().unwrap_or_default();
                    let lang = if let Some(RichTextNode::Plain(l)) = lang_node.into_iter().next() {
                        if l.is_empty() { None } else { Some(l) }
                    } else {
                        None
                    };
                    let code: String = code_nodes.iter().map(|n| n.to_plain_text()).collect();
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::CodeBlock {
                            language: lang,
                            code,
                        });
                    }
                }
                Event::Code(text) => {
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Code(text.to_string()));
                    }
                }
                Event::Text(text) => {
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Plain(text.to_string()));
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if let Some(top) = stack.last_mut() {
                        top.push(RichTextNode::Plain("\n".into()));
                    }
                }
                _ => {}
            }
        }

        Self(stack.into_iter().next().unwrap_or_default())
    }
}

impl std::fmt::Display for RichText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_plain_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_strips_formatting() {
        let rt = RichText(vec![
            RichTextNode::Bold(vec![RichTextNode::Plain("hello".into())]),
            RichTextNode::Plain(" world".into()),
        ]);
        assert_eq!(rt.to_plain_text(), "hello world");
    }

    #[test]
    fn discord_bold_renders_stars() {
        let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        assert!(rt.to_discord_markdown().contains("**hi**"));
    }

    #[test]
    fn matrix_bold_renders_b_tag() {
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
    fn whatsapp_bold_uses_stars() {
        let rt = RichText(vec![RichTextNode::Bold(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        assert!(rt.to_whatsapp_formatted().contains("*hi*"));
    }

    #[test]
    fn from_markdown_parses_bold() {
        let rt = RichText::from_markdown("**bold text**");
        assert!(rt.0.iter().any(|n| matches!(n, RichTextNode::Bold(_))));
    }

    #[test]
    fn display_gives_plain_text() {
        let rt = RichText(vec![RichTextNode::Plain("hello".into())]);
        assert_eq!(rt.to_string(), "hello");
    }

    #[test]
    fn code_block_roundtrip() {
        let rt = RichText(vec![RichTextNode::CodeBlock {
            language: Some("rust".into()),
            code: "let x = 1;".into(),
        }]);
        let md = rt.to_markdown();
        assert!(md.contains("```rust"));
        assert!(md.contains("let x = 1;"));
    }

    #[test]
    fn underline_renders_html_in_markdown() {
        let rt = RichText(vec![RichTextNode::Underline(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        assert!(rt.to_markdown().contains("<u>hi</u>"));
    }

    #[test]
    fn underline_renders_discord_double_underscore() {
        let rt = RichText(vec![RichTextNode::Underline(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        assert!(rt.to_discord_markdown().contains("__hi__"));
    }

    #[test]
    fn underline_renders_irc_control_char() {
        let rt = RichText(vec![RichTextNode::Underline(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        let s = rt.to_irc_formatted();
        assert!(s.contains('\x1F'));
    }

    #[test]
    fn underline_renders_matrix_u_tag() {
        let rt = RichText(vec![RichTextNode::Underline(vec![RichTextNode::Plain(
            "hi".into(),
        )])]);
        assert!(rt.to_matrix_html().contains("<u>hi</u>"));
    }

    #[test]
    fn spoiler_renders_discord_pipes() {
        let rt = RichText(vec![RichTextNode::Spoiler(vec![RichTextNode::Plain(
            "secret".into(),
        )])]);
        assert!(rt.to_discord_markdown().contains("||secret||"));
    }

    #[test]
    fn spoiler_renders_matrix_span() {
        let rt = RichText(vec![RichTextNode::Spoiler(vec![RichTextNode::Plain(
            "secret".into(),
        )])]);
        assert!(
            rt.to_matrix_html()
                .contains("<span data-mx-spoiler>secret</span>")
        );
    }

    #[test]
    fn blockquote_renders_markdown() {
        let rt = RichText(vec![RichTextNode::Blockquote(vec![RichTextNode::Plain(
            "quoted text".into(),
        )])]);
        assert_eq!(rt.to_markdown(), "> quoted text");
    }

    #[test]
    fn blockquote_renders_matrix_tag() {
        let rt = RichText(vec![RichTextNode::Blockquote(vec![RichTextNode::Plain(
            "quoted".into(),
        )])]);
        assert!(
            rt.to_matrix_html()
                .contains("<blockquote>quoted</blockquote>")
        );
    }

    #[test]
    fn heading_renders_markdown() {
        let rt = RichText(vec![RichTextNode::Heading {
            level: 2,
            children: vec![RichTextNode::Plain("Title".into())],
        }]);
        assert_eq!(rt.to_markdown(), "## Title");
    }

    #[test]
    fn heading_renders_matrix_html() {
        let rt = RichText(vec![RichTextNode::Heading {
            level: 3,
            children: vec![RichTextNode::Plain("Title".into())],
        }]);
        assert!(rt.to_matrix_html().contains("<h3>Title</h3>"));
    }

    #[test]
    fn heading_renders_irc_as_bold() {
        let rt = RichText(vec![RichTextNode::Heading {
            level: 1,
            children: vec![RichTextNode::Plain("Title".into())],
        }]);
        let s = rt.to_irc_formatted();
        assert!(s.contains('\x02'));
        assert!(s.contains("Title"));
    }

    #[test]
    fn from_markdown_parses_blockquote() {
        let rt = RichText::from_markdown("> quoted text");
        assert!(
            rt.0.iter()
                .any(|n| matches!(n, RichTextNode::Blockquote(_)))
        );
    }

    #[test]
    fn from_markdown_parses_heading() {
        let rt = RichText::from_markdown("## Hello");
        assert!(
            rt.0.iter()
                .any(|n| matches!(n, RichTextNode::Heading { level: 2, .. }))
        );
    }
}
