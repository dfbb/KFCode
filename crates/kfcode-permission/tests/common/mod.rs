#![allow(dead_code)]

use kfcode_permission::{PermissionAction, PermissionRule, PermissionRuleset};

pub fn rule(perm: &str, pattern: &str, action: PermissionAction) -> PermissionRule {
    PermissionRule {
        permission: perm.to_string(),
        pattern: pattern.to_string(),
        action,
    }
}

pub fn ruleset(rules: Vec<PermissionRule>) -> PermissionRuleset {
    rules
}
