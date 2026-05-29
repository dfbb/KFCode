mod common;

use kfcode_mcp::auth::{AuthEntry, AuthStore, OAuthTokens};
use tempfile::TempDir;

fn fresh_store() -> (AuthStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mcp-auth.json");
    (AuthStore::new(path), dir)
}

#[tokio::test]
async fn round_trips_auth_entry() {
    let (store, _dir) = fresh_store();
    let mut entry = AuthEntry::default();
    entry.tokens = Some(OAuthTokens {
        access_token: "tok".into(),
        refresh_token: Some("ref".into()),
        expires_at: None,
        scope: None,
    });
    store.set("server-1", entry).await.expect("set");
    let got = store.get("server-1").await.expect("io error").expect("must exist");
    assert_eq!(got.tokens.unwrap().access_token, "tok");
}

#[tokio::test]
async fn isolates_independent_stores() {
    let (a, _da) = fresh_store();
    let (b, _db) = fresh_store();
    let mut e = AuthEntry::default();
    e.server_url = Some("https://a".into());
    a.set("name", e).await.unwrap();
    assert!(b.get("name").await.unwrap().is_none(), "stores at distinct paths must not bleed");
}

#[tokio::test]
async fn remove_clears_entry() {
    let (store, _dir) = fresh_store();
    store.set("x", AuthEntry::default()).await.unwrap();
    store.remove("x").await.expect("remove");
    assert!(store.get("x").await.unwrap().is_none());
}

#[tokio::test]
async fn update_tokens_preserves_other_fields() {
    let (store, _dir) = fresh_store();
    let mut e = AuthEntry::default();
    e.server_url = Some("https://example".into());
    store.set("x", e).await.unwrap();

    store
        .update_tokens(
            "x",
            OAuthTokens {
                access_token: "new".into(),
                refresh_token: None,
                expires_at: None,
                scope: None,
            },
        )
        .await
        .expect("update_tokens");

    let got = store.get("x").await.unwrap().unwrap();
    assert_eq!(got.tokens.unwrap().access_token, "new");
    assert_eq!(got.server_url.as_deref(), Some("https://example"));
}
