use crate::init::Init;
use rstest::rstest;
use std::collections::HashSet;

#[rstest]
pub fn test() {
    let path = std::path::Path::new("./tests/test");
    let init = Init::new("", "", path);
    let set: HashSet<String> = HashSet::from_iter(
        init.get_subdecks_path()
            .unwrap()
            .iter()
            .map(|f| f.display().to_string()),
    );
    assert!(set.contains("a/b"));
    assert!(set.contains("a"));
    assert!(set.contains("a/b/c"));
}

#[rstest]
pub fn test_init() {
    let path = std::path::Path::new("./tests/test");
    let init = Init::new("", "", path);
    let generated = init.generate().unwrap();

    assert_eq!(generated.get("a::b").unwrap().added.len(), 1);
    assert_eq!(
        generated.get("a::b").unwrap().added[0].hash,
        "891d1444a24e1470faf362e757b6ffaec6d5c12a99b8bbe00d3636566040e57e"
    );

    assert_eq!(generated.get("a::b::c").unwrap().added.len(), 1);
    assert_eq!(
        generated.get("a::b::c").unwrap().added[0].hash,
        "d9a617e1bab933800c1df24cbe1a6cc543b1e0dcfd414346f3c2d7a07647d0c2"
    );

    assert_eq!(generated.get("a").unwrap().added.len(), 1);
    assert_eq!(
        generated.get("a").unwrap().added[0].hash,
        "d346aae91becee16016fe6d97d5d34f9e50f1261230577302f467fc4398cb90a"
    );
}
