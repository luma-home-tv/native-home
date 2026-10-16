use super::*;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "packaged-icon-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("icons")).unwrap();
        Self(path)
    }
    fn reader(&self) -> PackagedIconReader {
        PackagedIconReader::from_directory(File::open(&self.0).unwrap()).unwrap()
    }
    fn icon(&self) -> PathBuf {
        self.0.join("icons/test.png")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn path() -> RelativeIconPath {
    RelativeIconPath::parse("icons/test.png").unwrap()
}

#[test]
fn rejects_escape_urls_dot_and_non_icon_paths() {
    for bad in [
        "",
        "/icons/test.png",
        "icons",
        "icons/",
        "icons/.",
        "icons/..",
        "icons/../test.png",
        "icons//test.png",
        "icons/sub/test.png",
        "icons\\test.png",
        "icons/a:b",
        "https://host/icon",
        "fonts/test.png",
        "icons/a\0b",
        "icons/a\nb",
    ] {
        assert!(RelativeIconPath::parse(bad).is_err(), "{bad:?}");
    }
    assert!(RelativeIconPath::parse(&format!("icons/{}", "x".repeat(256))).is_err());
    assert!(RelativeIconPath::parse("icons/a-b_2.png").is_ok());
}
#[test]
fn nofollow_rejects_directory_and_leaf_symlinks() {
    let f = Fixture::new();
    let outside = f.0.join("outside");
    fs::write(&outside, b"not image").unwrap();
    symlink(&outside, f.icon()).unwrap();
    assert!(f.reader().read(&path()).is_err());
    fs::remove_file(f.icon()).unwrap();
    fs::remove_dir(f.0.join("icons")).unwrap();
    symlink(&f.0, f.0.join("icons")).unwrap();
    assert!(f.reader().read(&path()).is_err());
}
#[test]
fn fifo_directory_and_sparse_oversize_never_reach_decoder() {
    for kind in [0, 1, 2] {
        let f = Fixture::new();
        match kind {
            0 => {
                let name = CString::new(f.icon().as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            1 => fs::create_dir(f.icon()).unwrap(),
            _ => File::create(f.icon())
                .unwrap()
                .set_len(MAX_ENCODED_BYTES as u64 + 1)
                .unwrap(),
        }
        let mut called = false;
        assert!(f
            .reader()
            .decode_using(&path(), |_| {
                called = true;
                None
            })
            .is_err());
        assert!(!called);
    }
}
#[test]
fn fd_named_replacement_same_length_write_and_growth_are_rejected() {
    for mutation in [0, 1, 2, 3] {
        let f = Fixture::new();
        fs::write(f.icon(), b"original").unwrap();
        let r = f.reader();
        let got = r.read_checked(&path(), || match mutation {
            0 => {
                let replacement = f.0.join("replacement");
                fs::write(&replacement, b"original").unwrap();
                fs::rename(replacement, f.icon()).unwrap();
            }
            1 => {
                fs::write(f.icon(), b"modified").unwrap();
                File::options()
                    .write(true)
                    .open(f.icon())
                    .unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(
                        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1),
                    ))
                    .unwrap();
            }
            2 => fs::write(f.icon(), b"larger than original").unwrap(),
            _ => {
                fs::rename(f.0.join("icons"), f.0.join("moved")).unwrap();
                fs::create_dir(f.0.join("icons")).unwrap();
            }
        });
        assert!(got.is_err(), "mutation {mutation}");
    }
}
#[test]
fn readable_world_writable_and_hardlinked_asset_is_not_authority_rejected() {
    let f = Fixture::new();
    fs::write(f.icon(), b"ordinary packaged bytes").unwrap();
    fs::set_permissions(f.icon(), fs::Permissions::from_mode(0o777)).unwrap();
    fs::hard_link(f.icon(), f.0.join("alias")).unwrap();
    let read = f.reader().read(&path()).unwrap();
    assert_eq!(read.bytes, b"ordinary packaged bytes");
    assert_eq!(read.identity.mode & 0o777, 0o777);
    assert_eq!(read.identity.links, 2);
}

fn png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 99,
        208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69, 78, 68, 174,
        66, 96, 130,
    ]
}

#[test]
fn real_png_preserves_native_aspect_and_transparent_rgba() {
    let f = Fixture::new();
    fs::write(f.icon(), png()).unwrap();
    let decoded = f.reader().decode(&path()).unwrap();
    assert_eq!((decoded.width, decoded.height), (2, 1));
    assert_eq!(decoded.rgba, [10, 20, 30, 0, 40, 50, 60, 128]);
    assert_eq!(decoded.encoded_identity.bytes, png().len() as u64);
}
fn crc(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for b in bytes {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}
#[test]
fn malformed_and_valid_crc_oversized_headers_fail_bounded_decode() {
    for variant in [0, 1, 2] {
        let f = Fixture::new();
        let mut bytes = png();
        match variant {
            0 => bytes = b"not a PNG or JPEG".to_vec(),
            1 => {
                bytes[16..20].copy_from_slice(&513_u32.to_be_bytes());
            }
            _ => {
                bytes[16..20].copy_from_slice(&512_u32.to_be_bytes());
                bytes[20..24].copy_from_slice(&512_u32.to_be_bytes());
                bytes[24] = 16;
            }
        }
        if variant != 0 {
            let checksum = crc(&bytes[12..29]);
            bytes[29..33].copy_from_slice(&checksum.to_be_bytes());
        }
        fs::write(f.icon(), bytes).unwrap();
        assert!(f.reader().decode(&path()).is_err(), "variant {variant}");
    }
}
#[test]
fn file_mode_change_rejected_without_chmod_repair() {
    let f = Fixture::new();
    fs::write(f.icon(), b"before").unwrap();
    let r = f.reader();
    assert!(r
        .read_checked(&path(), || fs::set_permissions(
            f.icon(),
            fs::Permissions::from_mode(0o777)
        )
        .unwrap())
        .is_err());
    assert_eq!(fs::metadata(f.icon()).unwrap().mode() & 0o777, 0o777);
    fs::write(f.0.join("new-package-entry"), b"unrelated entry").unwrap();
    assert_eq!(r.read(&path()).unwrap().bytes, b"before");
}
#[test]
fn repeated_real_reads_do_not_refuse_access_time_updates() {
    let f = Fixture::new();
    fs::write(f.icon(), b"before").unwrap();
    let reader = f.reader();
    assert_eq!(reader.read(&path()).unwrap().bytes, b"before");
    assert_eq!(reader.read(&path()).unwrap().bytes, b"before");
}
#[test]
fn exact_encoded_limit_reads_but_oversize_is_refused() {
    let f = Fixture::new();
    File::create(f.icon())
        .unwrap()
        .set_len(MAX_ENCODED_BYTES as u64)
        .unwrap();
    assert_eq!(
        f.reader().read(&path()).unwrap().bytes.len(),
        MAX_ENCODED_BYTES
    );
    File::options()
        .write(true)
        .open(f.icon())
        .unwrap()
        .set_len(MAX_ENCODED_BYTES as u64 + 1)
        .unwrap();
    assert!(f.reader().read(&path()).is_err());
}

#[test]
fn unrelated_bundle_and_icons_entry_churn_does_not_poison_reader() {
    let f = Fixture::new();
    fs::write(f.icon(), b"unchanged icon").unwrap();
    let r = f.reader();
    let got = r
        .read_checked(&path(), || {
            fs::write(f.0.join("marker.json"), b"unrelated marker").unwrap();
            fs::write(f.0.join("icons/another.png"), b"unrelated icon").unwrap();
            fs::create_dir(f.0.join("icons/unrelated-directory")).unwrap();
        })
        .unwrap();
    assert_eq!(got.bytes, b"unchanged icon");
    fs::remove_file(f.0.join("marker.json")).unwrap();
    fs::remove_file(f.0.join("icons/another.png")).unwrap();
    fs::remove_dir(f.0.join("icons/unrelated-directory")).unwrap();
    assert_eq!(r.read(&path()).unwrap().bytes, b"unchanged icon");
}
#[test]
fn icons_name_replaced_by_symlink_during_read_is_rejected() {
    let f = Fixture::new();
    fs::write(f.icon(), b"unchanged icon").unwrap();
    let r = f.reader();
    assert!(r
        .read_checked(&path(), || {
            fs::rename(f.0.join("icons"), f.0.join("moved")).unwrap();
            symlink(f.0.join("moved"), f.0.join("icons")).unwrap();
        })
        .is_err());
}
#[test]
fn wrong_held_anchor_and_directory_mode_change_are_rejected() {
    let f = Fixture::new();
    fs::write(f.icon(), b"unchanged icon").unwrap();
    let other = Fixture::new();
    fs::write(other.icon(), b"unchanged icon").unwrap();
    let mut r = f.reader();
    r.directory = File::open(&other.0).unwrap();
    assert!(r.read(&path()).is_err());
    let r = f.reader();
    let original = fs::metadata(f.0.join("icons")).unwrap().mode() & 0o777;
    let result = r.read_checked(&path(), || {
        fs::set_permissions(
            f.0.join("icons"),
            fs::Permissions::from_mode(original ^ 0o100),
        )
        .unwrap();
    });
    fs::set_permissions(f.0.join("icons"), fs::Permissions::from_mode(original)).unwrap();
    assert!(result.is_err());
}
#[test]
fn held_root_anchor_does_not_claim_external_name_continuity() {
    let f = Fixture::new();
    fs::write(f.icon(), b"held bundle").unwrap();
    let r = f.reader();
    let moved = f.0.with_extension("held");
    fs::rename(&f.0, &moved).unwrap();
    // Root name is not supplied to this API. Its held directory remains valid;
    // controller generation checks decide whether this package is still wanted.
    let got = r.read(&path());
    fs::rename(&moved, &f.0).unwrap();
    assert_eq!(got.unwrap().bytes, b"held bundle");
}
