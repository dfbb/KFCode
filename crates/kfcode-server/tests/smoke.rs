mod common;

#[tokio::test]
async fn server_state_constructs() {
    let _state = common::fresh_state_in_memory();
}
