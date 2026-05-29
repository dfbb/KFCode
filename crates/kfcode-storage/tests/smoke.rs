mod common;

#[tokio::test]
async fn fresh_db_initializes_without_error() {
    let _db = common::fresh_db().await;
}
