mod common;

use kfcode_permission::{evaluate, disabled, build_agent_ruleset, PermissionAction};

#[test]
fn evaluate_returns_last_matching_rule() {
    let rs = vec![common::ruleset(vec![
        common::rule("read", "*", PermissionAction::Ask),
        common::rule("read", "*.env", PermissionAction::Deny),
        common::rule("read", "*.env.local", PermissionAction::Allow),
    ])];

    let action = evaluate("read", "*.env.local", &rs).action;
    assert_eq!(action, PermissionAction::Allow, "last-wins must pick Allow");

    let action = evaluate("read", "*.env", &rs).action;
    assert_eq!(action, PermissionAction::Deny);
}

#[test]
fn evaluate_falls_back_to_ask_when_no_match() {
    let rs = vec![common::ruleset(vec![
        common::rule("write", "*", PermissionAction::Allow),
    ])];
    let action = evaluate("read", "*.env", &rs).action;
    assert_eq!(action, PermissionAction::Ask);
}

#[test]
fn disabled_collects_globally_denied_tools() {
    let rs = common::ruleset(vec![
        common::rule("bash", "*", PermissionAction::Deny),
        common::rule("read", "*.env", PermissionAction::Ask),
    ]);
    let tools = vec!["bash".into(), "read".into(), "write".into()];
    let result = disabled(&tools, &rs);
    assert!(result.contains("bash"));
    assert!(!result.contains("read"), "read is Ask not Deny");
}

#[test]
fn disabled_aliases_edit_family() {
    let rs = common::ruleset(vec![
        common::rule("edit", "*", PermissionAction::Deny),
    ]);
    let tools = vec!["edit".into(), "write".into(), "patch".into(), "multiedit".into()];
    let result = disabled(&tools, &rs);
    for n in &tools {
        assert!(result.contains(n), "edit alias missing: {n}");
    }
}

#[test]
fn build_agent_ruleset_merges_build_specific_rules() {
    let user = vec![common::rule("read", "*", PermissionAction::Allow)];
    let rs = build_agent_ruleset("build", &user);
    assert!(!rs.is_empty());
}

#[test]
fn build_agent_ruleset_distinguishes_build_plan_explore() {
    let user: Vec<_> = vec![];
    let rb = build_agent_ruleset("build", &user);
    let rp = build_agent_ruleset("plan", &user);
    let re = build_agent_ruleset("explore", &user);
    // Different agents produce different rule counts
    assert_ne!(rb.len(), rp.len(), "build and plan should differ");
    assert_ne!(rp.len(), re.len(), "plan and explore should differ");
}

#[test]
fn build_agent_ruleset_unknown_agent_falls_back_to_defaults() {
    let user: Vec<_> = vec![];
    let _r = build_agent_ruleset("totally-unknown-agent", &user);
}
