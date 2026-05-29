mod common;

use kfcode_permission::PermissionAction;

#[test]
fn rule_helper_constructs() {
    let r = common::rule("read", "*.env", PermissionAction::Ask);
    assert_eq!(r.permission, "read");
}
