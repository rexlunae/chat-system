#![cfg(feature = "signal-cli")]

use chat_system::messengers::SignalCliMessenger;
use chat_system::Messenger;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn unique_test_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("chat-system-signal-cli-{nanos}"))
}

fn write_mock_signal_cli() -> (PathBuf, PathBuf) {
    let dir = unique_test_dir();
    fs::create_dir_all(&dir).unwrap();

    let script_path = dir.join("signal-cli-mock.sh");
    let log_path = dir.join("signal-cli.log");
    let script = r#"#!/bin/sh
set -eu
LOG_FILE="__LOG_PATH__"
if [ "${1:-}" = "--version" ]; then
    echo "signal-cli 0.13.0"
    exit 0
fi
if [ "${3:-}" = "send" ]; then
    echo "$*" >> "$LOG_FILE"
    exit 0
fi
if [ "${3:-}" = "receive" ]; then
    cat <<'EOF'
{"envelope":{"timestamp":1710000000000,"sourceNumber":"+15551234567","sourceName":"Alice","dataMessage":{"message":"hello from signal"}}}
{"envelope":{"timestamp":1710000001000,"sourceNumber":"+15557654321","dataMessage":{"message":"group hello","groupInfo":{"groupId":"group-123"},"quote":{"id":1709999999000}}}}
EOF
    exit 0
fi
echo "unexpected args: $*" >&2
exit 1
"#
        .replace("__LOG_PATH__", &log_path.display().to_string());

    fs::write(&script_path, script).unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();
    }

    (script_path, log_path)
}

#[tokio::test]
async fn signal_initialize_sets_connected() {
    let (script_path, _) = write_mock_signal_cli();
    let mut messenger = SignalCliMessenger::new("signal", "+15550000000")
        .with_cli_path(script_path.to_string_lossy().into_owned());

    messenger.initialize().await.unwrap();

    assert!(messenger.is_connected());
}

#[tokio::test]
async fn signal_send_message_invokes_cli() {
    let (script_path, log_path) = write_mock_signal_cli();
    let mut messenger = SignalCliMessenger::new("signal", "+15550000000")
        .with_cli_path(script_path.to_string_lossy().into_owned());
    messenger.initialize().await.unwrap();

    let message_id = messenger
        .send_message("+15551112222", "test signal send")
        .await
        .unwrap();

    assert!(message_id.starts_with("signal:"));
    let log = fs::read_to_string(log_path).unwrap();
    assert!(log.contains("-u +15550000000 send -m test signal send +15551112222"));
}

#[tokio::test]
async fn signal_receive_messages_parses_json_output() {
    let (script_path, _) = write_mock_signal_cli();
    let mut messenger = SignalCliMessenger::new("signal", "+15550000000")
        .with_cli_path(script_path.to_string_lossy().into_owned());
    messenger.initialize().await.unwrap();

    let messages = messenger.receive_messages().await.unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].sender, "Alice");
    assert_eq!(messages[0].content, "hello from signal");
    assert_eq!(messages[0].timestamp, 1_710_000_000);
    assert!(messages[0].is_direct);
    assert_eq!(messages[1].sender, "+15557654321");
    assert_eq!(messages[1].channel.as_deref(), Some("group-123"));
    assert_eq!(messages[1].reply_to.as_deref(), Some("1709999999000"));
    assert!(!messages[1].is_direct);
}

#[tokio::test]
async fn signal_disconnect_clears_connected() {
    let (script_path, _) = write_mock_signal_cli();
    let mut messenger = SignalCliMessenger::new("signal", "+15550000000")
        .with_cli_path(script_path.to_string_lossy().into_owned());
    messenger.initialize().await.unwrap();

    messenger.disconnect().await.unwrap();

    assert!(!messenger.is_connected());
}
