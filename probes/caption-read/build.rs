fn main() {
    println!("cargo:rerun-if-env-changed=CAPTION_PROBE_SOURCE_SHA256");
    println!("cargo:rerun-if-changed=src/main.rs");
    let sha = std::env::var("CAPTION_PROBE_SOURCE_SHA256").unwrap_or_else(|_| {
        if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm") {
            panic!("Use source-bound build recipe");
        }
        "host-not-native".into()
    });
    assert!(
        sha == "host-not-native" || (sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit()))
    );
    println!("cargo:rustc-env=CAPTION_PROBE_SOURCE_SHA256={sha}");
}
