fn main() {
    println!("cargo:rerun-if-env-changed=SURFACE_PROBE_GLUE_SHA256");
    println!("cargo:rerun-if-changed=src/main.rs");
    let value = std::env::var("SURFACE_PROBE_GLUE_SHA256").unwrap_or_else(|_| {
        if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm") {
            panic!("Native probe requires source-bound build recipe");
        }
        "host-not-native".into()
    });
    println!("cargo:rustc-env=SURFACE_PROBE_GLUE_SHA256={value}");
}
