mod common;

use kfcode_mcp::auth::{AuthEntry, AuthStore, OAuthTokens, is_token_expired};

#[tokio::test]
async fn token_expiry_detection() {
    // expires_at 在过去 → expired
    let mut e = AuthEntry::default();
    e.tokens = Some(OAuthTokens {
        access_token: "old".into(),
        refresh_token: None,
        expires_at: Some(0.0),
        scope: None,
    });
    assert_eq!(is_token_expired(&e), Some(true));

    // expires_at 在未来 → not expired
    let mut e2 = AuthEntry::default();
    let future = (chrono::Utc::now().timestamp() + 3600) as f64;
    e2.tokens = Some(OAuthTokens {
        access_token: "fresh".into(),
        refresh_token: None,
        expires_at: Some(future),
        scope: None,
    });
    assert_eq!(is_token_expired(&e2), Some(false));

    // 缺少 expires_at → Some(false)（实现：无过期时间视为未过期）
    let mut e3 = AuthEntry::default();
    e3.tokens = Some(OAuthTokens {
        access_token: "x".into(),
        refresh_token: None,
        expires_at: None,
        scope: None,
    });
    assert_eq!(is_token_expired(&e3), Some(false));
}

#[tokio::test]
async fn store_persists_across_reload() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("auth.json");

    {
        let store = AuthStore::new(path.clone());
        let mut e = AuthEntry::default();
        e.tokens = Some(OAuthTokens {
            access_token: "tok".into(),
            refresh_token: Some("ref".into()),
            expires_at: None,
            scope: None,
        });
        e.server_url = Some("https://api.example.com".into());
        store.set("server-1", e).await.unwrap();
    }

    let store = AuthStore::new(path);
    let got = store.get("server-1").await.expect("must persist");
    assert_eq!(got.tokens.unwrap().access_token, "tok");
    assert_eq!(got.server_url.as_deref(), Some("https://api.example.com"));
}

#[tokio::test]
async fn get_for_url_invalidates_on_url_change() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = AuthStore::new(dir.path().join("auth.json"));
    let mut e = AuthEntry::default();
    e.server_url = Some("https://old".into());
    store.set("name", e).await.unwrap();

    assert!(store.get_for_url("name", "https://old").await.is_some());
    assert!(
        store.get_for_url("name", "https://new").await.is_none(),
        "URL change must invalidate"
    );
}
