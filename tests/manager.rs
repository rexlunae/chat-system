use chat_system::messengers::ConsoleMessenger;
use chat_system::{Message, MessengerManager};

fn make_console(name: &str) -> ConsoleMessenger {
    ConsoleMessenger::new(name.to_string())
}

fn make_message(id: &str, sender: &str, content: &str) -> Message {
    Message {
        id: id.to_string(),
        sender: sender.to_string(),
        content: content.to_string(),
        timestamp: 1000,
        channel: None,
        reply_to: None,
        media: None,
        is_direct: false,
        reactions: None,
    }
}

#[tokio::test]
async fn manager_new_is_empty() {
    let mgr = MessengerManager::new();
    assert!(mgr.messengers().is_empty());
}

#[tokio::test]
async fn manager_default_is_empty() {
    let mgr = MessengerManager::default();
    assert!(mgr.messengers().is_empty());
}

#[tokio::test]
async fn manager_add_increases_count() {
    let mut mgr = MessengerManager::new();
    mgr = mgr.add(make_console("a"));
    assert_eq!(mgr.messengers().len(), 1);
    mgr = mgr.add(make_console("b"));
    assert_eq!(mgr.messengers().len(), 2);
}

#[tokio::test]
async fn manager_add_three_messengers() {
    let mut mgr = MessengerManager::new();
    for name in ["a", "b", "c"] {
        mgr = mgr.add(make_console(name));
    }
    assert_eq!(mgr.messengers().len(), 3);
}

#[tokio::test]
async fn manager_get_by_name_found() {
    let mut mgr = MessengerManager::new();
    mgr = mgr.add(make_console("alpha"));
    mgr = mgr.add(make_console("beta"));
    assert!(mgr.get("alpha").is_some());
    assert!(mgr.get("beta").is_some());
}

#[tokio::test]
async fn manager_get_by_name_not_found() {
    let mut mgr = MessengerManager::new();
    mgr = mgr.add(make_console("alpha"));
    assert!(mgr.get("gamma").is_none());
}

#[tokio::test]
async fn manager_get_returns_correct_messenger() {
    let mut mgr = MessengerManager::new();
    mgr = mgr.add(make_console("my-console"));
    let m = mgr.get("my-console").unwrap();
    assert_eq!(m.name(), "my-console");
    assert_eq!(m.messenger_type(), "console");
}

#[tokio::test]
async fn manager_initialize_all() {
    let mut mgr = MessengerManager::new()
        .add(make_console("a"))
        .add(make_console("b"));
    mgr.initialize_all().await.unwrap();
    for m in mgr.messengers() {
        assert!(m.is_connected());
    }
}

#[tokio::test]
async fn manager_disconnect_all() {
    let mut mgr = MessengerManager::new()
        .add(make_console("a"))
        .add(make_console("b"));
    mgr.initialize_all().await.unwrap();
    mgr.disconnect_all().await.unwrap();
    for m in mgr.messengers() {
        assert!(!m.is_connected());
    }
}

#[tokio::test]
async fn manager_receive_all_empty() {
    let mut mgr = MessengerManager::new().add(make_console("a"));
    mgr.initialize_all().await.unwrap();
    let msgs = mgr.receive_all().await.unwrap();
    assert!(msgs.is_empty());
}

#[tokio::test]
async fn manager_receive_all_collects_from_all_messengers() {
    let mut mgr = MessengerManager::new();

    let mut m1 = make_console("a");
    m1.enqueue(make_message("1", "alice", "msg1"));
    let mut m2 = make_console("b");
    m2.enqueue(make_message("2", "bob", "msg2"));
    m2.enqueue(make_message("3", "bob", "msg3"));

    mgr = mgr.add(m1);
    mgr = mgr.add(m2);
    mgr.initialize_all().await.unwrap();

    let msgs = mgr.receive_all().await.unwrap();
    assert_eq!(msgs.len(), 3);
}

#[tokio::test]
async fn manager_broadcast_sends_to_all() {
    let mut mgr = MessengerManager::new()
        .add(make_console("a"))
        .add(make_console("b"))
        .add(make_console("c"));
    mgr.initialize_all().await.unwrap();
    let results = mgr.broadcast("#general", "hello everyone").await;
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn manager_broadcast_empty_returns_no_results() {
    let mgr = MessengerManager::new();
    let results = mgr.broadcast("#channel", "hello").await;
    assert!(results.is_empty());
}

#[tokio::test]
async fn manager_messengers_slice_is_in_insertion_order() {
    let mgr = MessengerManager::new()
        .add(make_console("first"))
        .add(make_console("second"))
        .add(make_console("third"));
    let names: Vec<&str> = mgr.messengers().iter().map(|m| m.name()).collect();
    assert_eq!(names, vec!["first", "second", "third"]);
}

#[tokio::test]
async fn manager_receive_all_tolerates_messenger_with_error() {
    // ConsoleMessenger never errors, but the manager should collect
    // successfully from all that don't error.
    let mut m = make_console("ok");
    m.enqueue(make_message("1", "alice", "hello"));
    let mut mgr = MessengerManager::new().add(m);
    mgr.initialize_all().await.unwrap();
    let msgs = mgr.receive_all().await.unwrap();
    assert_eq!(msgs.len(), 1);
}
