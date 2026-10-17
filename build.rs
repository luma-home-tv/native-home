#[path = ".dependencies/plx-native/rust-modules/build_support/host_link.rs"]
mod host_link;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    verify_framework();
    consumer_contract();
    println!("cargo:rerun-if-changed=.dependencies/plx-native/rust-modules/build_support/host_link.rs");
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default()=="arm" {
        println!("cargo:rustc-link-lib=luna-service2");
        println!("cargo:rustc-link-lib=glib-2.0");
    }
    compile_events();
    host_link::emit(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".dependencies/plx-native"));
}

fn verify_framework() {
    // Recursive directory tracking makes cached direct Cargo invocations rerun
    // this guard after dependency edits. The wrapper checks before Cargo starts.
    for path in ["tools/verify-framework.py", "dependency-lock.json",
        "patches/plxnative-framework-integration.patch", ".dependencies/plx-native"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let result = std::process::Command::new("python3")
        .arg(root.join("tools/verify-framework.py"))
        .arg(root)
        .output()
        .expect("Run read-only framework verifier");
    assert!(result.status.success(), "Framework verification failed: {}",
        String::from_utf8_lossy(&result.stderr));
}

fn compile_events() {
    use std::{path::PathBuf, process::Command};
    println!("cargo:rerun-if-changed=sdl-events.c");
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "arm" {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("sdl-events.o");
    let flags = Command::new("pkg-config")
        .args(["--cflags", "sdl2"])
        .output()
        .expect("SDL pkg-config");
    assert!(flags.status.success(), "SDL include flags unavailable");
    let flags = String::from_utf8(flags.stdout).unwrap();
    let status = Command::new("cc")
        .args([
            "-c",
            "sdl-events.c",
            "-O2",
            "-fPIC",
            "-Wall",
            "-Wextra",
            "-Werror",
        ])
        .args(flags.split_whitespace())
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success(), "SDL adapter compile");
    println!("cargo:rustc-link-arg={}", out.display());
}

fn consumer_contract() {
    for path in ["Cargo.toml", "cache_consumer_contract.py"] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=NATIVE_HOME_CONSUMER_BUILD_SELECTION");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::path::Path::new(&std::env::var("OUT_DIR").unwrap())
        .join("cache-consumer-build-contract.json");
    let result = std::process::Command::new("python3")
        .arg(root.join("cache_consumer_contract.py")).arg(root).arg(output)
        .output().expect("Compile actual Cargo selection contract");
    assert!(result.status.success(), "Consumer build contract failed: {}",
        String::from_utf8_lossy(&result.stderr));
    assert!(result.stdout.len() < 4096, "Bounded compile contract output");
    print!("{}", String::from_utf8(result.stdout).unwrap());
}
