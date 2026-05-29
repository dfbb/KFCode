mod common;

#[tokio::test]
async fn watcher_constructs_and_subscribes() {
    let w = common::fresh_watcher();
    let _rx = w.subscribe();
}
