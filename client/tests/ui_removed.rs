use std::fs;

#[test]
fn ui_dependencies_removed() {
    let src = fs::read_to_string("src/main.rs").expect("read main.rs");
    assert!(!src.contains("bevy_egui"));
    assert!(!src.contains("reqwest"));
}
