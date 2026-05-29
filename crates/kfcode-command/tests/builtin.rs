mod common;

#[test]
fn new_registry_has_builtin_commands() {
    let r = common::fresh_registry();
    let list = r.list();
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert!(!names.is_empty(), "registry must have builtin commands; got empty");
    assert!(names.contains(&"init"), "expected builtin 'init'; got: {:?}", names);
    assert!(names.contains(&"review"), "expected builtin 'review'; got: {:?}", names);
    assert!(names.contains(&"commit"), "expected builtin 'commit'; got: {:?}", names);
    assert!(names.contains(&"test"), "expected builtin 'test'; got: {:?}", names);
}

#[test]
fn get_returns_some_for_builtin() {
    let r = common::fresh_registry();
    let list = r.list();
    if let Some(first) = list.first() {
        assert!(r.get(&first.name).is_some());
    }
}

#[test]
fn get_returns_none_for_unknown() {
    let r = common::fresh_registry();
    assert!(r.get("does-not-exist-xyz").is_none());
}
