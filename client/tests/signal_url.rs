use std::fs;

#[test]
fn no_signal_url_default() {
    let src = fs::read_to_string("src/net.rs").expect("read net.rs");
    assert!(!src.contains("ws://localhost:3000/signal"));
}

#[test]
fn no_old_default() {
    let src = fs::read_to_string("src/net.rs").expect("read net.rs");
    assert!(!src.contains("ws://localhost:9001"));
}
