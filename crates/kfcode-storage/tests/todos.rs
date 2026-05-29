mod common;

use kfcode_storage::{SessionRepository, TodoItem, TodoRepository};

fn todo(id: &str, content: &str, position: i64) -> TodoItem {
    TodoItem {
        id: id.to_string(),
        content: content.to_string(),
        status: "pending".to_string(),
        priority: "normal".to_string(),
        position,
    }
}

async fn setup_session(db: &kfcode_storage::Database, sid: &str) {
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session(sid, "p")).await.unwrap();
}

#[tokio::test]
async fn list_returns_empty_for_new_session() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    let list = repo.list_for_session("s").await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn upsert_inserts_and_updates() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());

    repo.upsert("s", &todo("t1", "first", 0)).await.expect("insert");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].content, "first");

    let mut updated = todo("t1", "first updated", 0);
    updated.status = "done".to_string();
    repo.upsert("s", &updated).await.expect("update");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].content, "first updated");
    assert_eq!(list[0].status, "done");
}

#[tokio::test]
async fn list_orders_by_position() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("c", "third", 2)).await.unwrap();
    repo.upsert("s", &todo("a", "first", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "second", 1)).await.unwrap();
    let list = repo.list_for_session("s").await.unwrap();
    let positions: Vec<i64> = list.iter().map(|t| t.position).collect();
    assert_eq!(positions, vec![0, 1, 2], "expected ascending position order");
}

#[tokio::test]
async fn delete_removes_single_todo() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "y", 1)).await.unwrap();
    repo.delete("s", "a").await.expect("delete");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "b");
}

#[tokio::test]
async fn delete_for_session_clears_all() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "y", 1)).await.unwrap();
    repo.delete_for_session("s").await.expect("delete_for_session");
    assert!(repo.list_for_session("s").await.unwrap().is_empty());
}

#[tokio::test]
async fn deleting_session_cascades_todos() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let trepo = TodoRepository::new(db.pool().clone());
    trepo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    SessionRepository::new(db.pool().clone()).delete("s").await.unwrap();
    let list = trepo.list_for_session("s").await.unwrap();
    assert!(list.is_empty(), "cascade should remove todos");
}

#[tokio::test]
async fn rejects_todo_for_nonexistent_session() {
    let db = common::fresh_db().await;
    let repo = TodoRepository::new(db.pool().clone());
    let err = repo.upsert("missing", &todo("a", "x", 0)).await.expect_err("FK");
    let msg = format!("{err}").to_uppercase();
    assert!(msg.contains("FOREIGN KEY") || msg.contains("CONSTRAINT"), "got: {msg}");
}
