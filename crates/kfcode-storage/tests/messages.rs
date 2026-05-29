mod common;

use kfcode_storage::{MessageRepository, SessionRepository};
use kfcode_types::message::MessageRole;

async fn setup_session(db: &kfcode_storage::Database, sid: &str) {
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session(sid, "p")).await.unwrap();
}

#[tokio::test]
async fn round_trips_user_message() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    let m = common::make_message("msg-1", "s", MessageRole::User);
    repo.create(&m).await.expect("create");
    let got = repo.get("msg-1").await.unwrap().unwrap();
    assert_eq!(got.id, "msg-1");
    assert_eq!(got.role, MessageRole::User);
}

#[tokio::test]
async fn rejects_message_for_nonexistent_session() {
    let db = common::fresh_db().await;
    let repo = MessageRepository::new(db.pool().clone());
    let m = common::make_message("msg-orphan", "no-such-session", MessageRole::User);
    let err = repo.create(&m).await.expect_err("expected FK error");
    let msg = format!("{err}").to_uppercase();
    assert!(msg.contains("FOREIGN KEY") || msg.contains("CONSTRAINT"), "got: {msg}");
}

#[tokio::test]
async fn upsert_overwrites_existing_row() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    let mut m = common::make_message("msg", "s", MessageRole::User);
    repo.create(&m).await.unwrap();
    m.role = MessageRole::Assistant;
    repo.upsert(&m).await.expect("upsert");
    let got = repo.get("msg").await.unwrap().unwrap();
    assert_eq!(got.role, MessageRole::Assistant);
}

#[tokio::test]
async fn list_for_session_returns_only_session_messages() {
    let db = common::fresh_db().await;
    setup_session(&db, "s1").await;
    setup_session(&db, "s2").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("a", "s1", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("b", "s2", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("c", "s1", MessageRole::Assistant)).await.unwrap();

    let list1 = repo.list_for_session("s1").await.unwrap();
    assert_eq!(list1.len(), 2);
    assert!(list1.iter().all(|m| m.session_id == "s1"));

    let list2 = repo.list_for_session("s2").await.unwrap();
    assert_eq!(list2.len(), 1);
}

#[tokio::test]
async fn delete_removes_single_message() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("m1", "s", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("m2", "s", MessageRole::User)).await.unwrap();
    repo.delete("m1").await.expect("delete");
    assert!(repo.get("m1").await.unwrap().is_none());
    assert!(repo.get("m2").await.unwrap().is_some());
}

#[tokio::test]
async fn delete_for_session_clears_only_target() {
    let db = common::fresh_db().await;
    setup_session(&db, "s1").await;
    setup_session(&db, "s2").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("a", "s1", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("b", "s2", MessageRole::User)).await.unwrap();
    repo.delete_for_session("s1").await.expect("delete_for_session");
    assert!(repo.list_for_session("s1").await.unwrap().is_empty());
    assert_eq!(repo.list_for_session("s2").await.unwrap().len(), 1);
}

#[tokio::test]
async fn deleting_session_cascades_messages() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let mrepo = MessageRepository::new(db.pool().clone());
    mrepo.create(&common::make_message("m", "s", MessageRole::User)).await.unwrap();
    let srepo = SessionRepository::new(db.pool().clone());
    srepo.delete("s").await.expect("session delete");
    assert!(mrepo.get("m").await.unwrap().is_none(), "message should be cascaded");
}
