//! `rbxtsc-fw --build-info`: rbxts-rs's `key: value` lines, the Flamework plugin's, then this release's version.

use std::process::Command;

#[test]
fn build_info_names_this_release() {
    let out = Command::new(env!("CARGO_BIN_EXE_rbxtsc-fw")).arg("--build-info").output().expect("rbxtsc-fw runs");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("utf-8");
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("rbxts-rs-version: "), "{text}");
    assert!(lines.contains(&format!("plugin: flamework {}", flamework_rs::VERSION).as_str()), "{text}");
    assert_eq!(lines.last(), Some(&format!("flamework-rs-version: {}", env!("CARGO_PKG_VERSION")).as_str()), "{text}");
}
