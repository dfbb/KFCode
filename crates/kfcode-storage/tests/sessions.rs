mod common;

use kfcode_storage::SessionRepository;

#[tokio::test]
async fn round_trips_session() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("ses-1", "proj");
    repo.create(&s).await.expect("create");
    let got = repo.get("ses-1").await.expect("get").expect("some");
    assert_eq!(got.id, "ses-1");
    assert_eq!(got.project_id, "proj");
}

#[tokio::test]
async fn get_returns_none_for_missing() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    assert!(repo.get("does-not-exist").await.unwrap().is_none());
}

#[tokio::test]
async fn rejects_duplicate_session_id() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("dup", "proj");
    repo.create(&s).await.expect("first ok");
    let err = repo.create(&s).await.expect_err("expected dup error");
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("unique") || msg.to_lowercase().contains("constraint"),
        "got: {msg}"
    );
}

#[tokio::test]
async fn list_filters_by_project() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session("a", "p1")).await.unwrap();
    repo.create(&common::make_session("b", "p2")).await.unwrap();
    repo.create(&common::make_session("c", "p1")).await.unwrap();

    let all = repo.list(None, 100).await.unwrap();
    assert_eq!(all.len(), 3);

    let p1 = repo.list(Some("p1"), 100).await.unwrap();
    assert_eq!(p1.len(), 2);
    assert!(p1.iter().all(|s| s.project_id == "p1"));
}

#[tokio::test]
async fn list_respects_limit() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    for i in 0..5 {
        repo.create(&common::make_session(&format!("s{i}"), "p"))
            .await
            .unwrap();
    }
    let limited = repo.list(None, 2).await.unwrap();
    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn update_persists_changes() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let mut s = common::make_session("s", "p");
    repo.create(&s).await.unwrap();
    s.title = "Renamed".to_string();
    repo.update(&s).await.expect("update");
    let got = repo.get("s").await.unwrap().unwrap();
    assert_eq!(got.title, "Renamed");
}

#[tokio::test]
async fn delete_removes_session() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("s", "p");
    repo.create(&s).await.unwrap();
    repo.delete("s").await.expect("delete");
    assert!(repo.get("s").await.unwrap().is_none());
}

#[tokio::test]
async fn list_children_finds_forked_sessions() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session("parent", "p")).await.unwrap();

    let mut child = common::make_session("child-1", "p");
    child.parent_id = Some("parent".to_string());
    repo.create(&child).await.unwrap();

    let mut child2 = common::make_session("child-2", "p");
    child2.parent_id = Some("parent".to_string());
    repo.create(&child2).await.unwrap();

    repo.create(&common::make_session("unrelated", "p")).await.unwrap();

    let kids = repo.list_children("parent").await.unwrap();
    assert_eq!(kids.len(), 2);
    assert!(kids.iter().all(|s| s.parent_id.as_deref() == Some("parent")));
}

#[tokio::test]
async fn unicode_session_fields_round_trip() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let mut s = common::make_session("uni-😀", "项目");
    s.title = "中文标题 🚀".to_string();
    s.directory = "/tmp/路径".to_string();
    repo.create(&s).await.unwrap();
    let got = repo.get("uni-😀").await.unwrap().unwrap();
    assert_eq!(got.title, "中文标题 🚀");
    assert_eq!(got.directory, "/tmp/路径");
    assert_eq!(got.project_id, "项目");
}
