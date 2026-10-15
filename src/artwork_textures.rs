//! Render-thread ownership of live artwork, separate from packaged fallbacks.
//! Construct on the render thread; the store is intentionally !Send and !Sync.
//! This module calls a supplied backend, never a GL API, decoder, or reader.
//! Byte limits estimate stored RGBA texture payload, not actual driver/GPU/RSS
//! use. Atomic replacement may temporarily allocate one extra <=1 MiB image.
use crate::artwork_cache::{
    ResidentKey, ResidentView, MAX_AXIS, MAX_ENTRIES, MAX_IMAGE_BYTES, MAX_RESIDENT_BYTES,
};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait TextureBackend {
    type Handle: Copy + Eq;
    type Error;
    /// Ok transfers one fresh, globally unique live handle to this store.
    /// Err must clean up any partial backend allocation before returning.
    /// The backend must reject its own invalid/sentinel handles (e.g. GL zero).
    fn upload(&mut self, width: u32, height: u32, rgba: &[u8])
        -> Result<Self::Handle, Self::Error>;
    /// Relinquish exactly this owned handle. Implementations must not panic.
    fn delete(&mut self, handle: Self::Handle);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub max_entries: usize,
    pub max_rgba_bytes: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_entries: MAX_ENTRIES,
            max_rgba_bytes: MAX_RESIDENT_BYTES,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidLimit,
}
#[derive(Debug, PartialEq, Eq)]
pub enum InstallError<E> {
    InvalidKey,
    InvalidImage,
    StaleRevision,
    ConflictingRevision,
    Capacity,
    Backend(E),
    /// A backend violated its fresh-handle contract. An alias is not deleted,
    /// since doing so would destroy a still-owned existing texture.
    AliasedHandle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Uploaded { replaced: bool },
    AlreadyPresent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub rgba_bytes_estimate: usize,
}
pub struct TextureView<'a, H> {
    /// Borrowed drawing reference, never a transfer of deletion ownership.
    pub key: &'a ResidentKey,
    pub handle: H,
    pub width: u32,
    pub height: u32,
}
struct Owned<H> {
    key: ResidentKey,
    handle: H,
    width: u32,
    height: u32,
    bytes: usize,
}
pub struct ArtworkTextures<B: TextureBackend> {
    backend: B,
    config: Config,
    entries: BTreeMap<String, Owned<B::Handle>>,
    bytes: usize,
    _render_thread: PhantomData<Rc<()>>,
}

fn key_valid(key: &ResidentKey) -> bool {
    let source = &key.source;
    if key.revision == 0
        || !crate::model::valid_app_id(&source.app_id)
        || matches!(source.app_id.as_str(), "." | "..")
        || source.version.is_empty()
        || source.version.len() > 128
        || source.version.chars().any(char::is_control)
        || source.folder != format!("{}/{}", source.class.prefix(), source.app_id)
        || source.relative.is_empty()
        || source.relative.len() > 8
    {
        return false;
    }
    let mut bytes = source.relative.len() - 1;
    for part in &source.relative {
        if part.is_empty()
            || part.len() > 128
            || matches!(part.as_str(), "." | "..")
            || part.contains(['/', '\\', ':'])
            || part.chars().any(char::is_control)
        {
            return false;
        }
        bytes += part.len();
    }
    bytes <= 512
}
fn image_bytes(width: u32, height: u32, rgba: &[u8]) -> Option<usize> {
    if width == 0 || height == 0 || width > MAX_AXIS || height > MAX_AXIS {
        return None;
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)?;
    (bytes <= MAX_IMAGE_BYTES as u64 && bytes == rgba.len() as u64).then_some(rgba.len())
}

impl<B: TextureBackend> ArtworkTextures<B> {
    pub fn new(backend: B, config: Config) -> Result<Self, ConfigError> {
        if config.max_entries == 0
            || config.max_entries > MAX_ENTRIES
            || config.max_rgba_bytes == 0
            || config.max_rgba_bytes > MAX_RESIDENT_BYTES
        {
            return Err(ConfigError::InvalidLimit);
        }
        Ok(Self {
            backend,
            config,
            entries: BTreeMap::new(),
            bytes: 0,
            _render_thread: PhantomData,
        })
    }
    /// The caller must first process cache eviction handles, then install only
    /// the current cache resident, at most once per frame. This store does not
    /// schedule frames. A revision denotes immutable pixel content and shape.
    pub fn install(
        &mut self,
        view: &ResidentView<'_>,
    ) -> Result<InstallOutcome, InstallError<B::Error>> {
        if !key_valid(view.key) {
            return Err(InstallError::InvalidKey);
        }
        let bytes =
            image_bytes(view.width, view.height, view.rgba).ok_or(InstallError::InvalidImage)?;
        let id = &view.key.source.app_id;
        let previous = self.entries.get(id);
        if let Some(old) = previous {
            if view.key.revision < old.key.revision {
                return Err(InstallError::StaleRevision);
            }
            if view.key.revision == old.key.revision {
                if view.key != &old.key
                    || view.width != old.width
                    || view.height != old.height
                    || bytes != old.bytes
                {
                    return Err(InstallError::ConflictingRevision);
                }
                return Ok(InstallOutcome::AlreadyPresent);
            }
        }
        let old_bytes = previous.map_or(0, |old| old.bytes);
        let next_bytes = self.bytes - old_bytes + bytes;
        if next_bytes > self.config.max_rgba_bytes
            || (previous.is_none() && self.entries.len() >= self.config.max_entries)
        {
            return Err(InstallError::Capacity);
        }
        let handle = self
            .backend
            .upload(view.width, view.height, view.rgba)
            .map_err(InstallError::Backend)?;
        if self.entries.values().any(|old| old.handle == handle) {
            return Err(InstallError::AliasedHandle);
        }
        let replacement = Owned {
            key: view.key.clone(),
            handle,
            width: view.width,
            height: view.height,
            bytes,
        };
        let old = self.entries.insert(id.clone(), replacement);
        self.bytes = next_bytes;
        if let Some(old) = old.as_ref() {
            self.backend.delete(old.handle);
        }
        Ok(InstallOutcome::Uploaded {
            replaced: old.is_some(),
        })
    }
    /// An old source/revision must not delete a newer image of the same app.
    pub fn evict(&mut self, key: &ResidentKey) -> bool {
        if self
            .entries
            .get(&key.source.app_id)
            .is_none_or(|old| old.key != *key)
        {
            return false;
        }
        let old = self
            .entries
            .remove(&key.source.app_id)
            .expect("matching owned texture");
        self.bytes -= old.bytes;
        self.backend.delete(old.handle);
        true
    }
    pub fn texture(&self, app_id: &str) -> Option<TextureView<'_, B::Handle>> {
        let owned = self.entries.get(app_id)?;
        Some(TextureView {
            key: &owned.key,
            handle: owned.handle,
            width: owned.width,
            height: owned.height,
        })
    }
    pub fn clear(&mut self) {
        for owned in std::mem::take(&mut self.entries).into_values() {
            self.backend.delete(owned.handle);
        }
        self.bytes = 0;
    }
    pub fn stats(&self) -> Stats {
        Stats {
            entries: self.entries.len(),
            rgba_bytes_estimate: self.bytes,
        }
    }
}
impl<B: TextureBackend> Drop for ArtworkTextures<B> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_artwork::{FileIdentity, IconKind, RootClass, SourceKey};
    use crate::artwork_cache::Fingerprint;
    use std::cell::RefCell;
    use std::collections::BTreeSet;
    #[derive(Default)]
    struct State {
        next: u32,
        live: BTreeSet<u32>,
        deleted: Vec<u32>,
        attempts: usize,
        fail: bool,
        alias: Option<u32>,
    }
    #[derive(Clone)]
    struct Fake(Rc<RefCell<State>>);
    impl TextureBackend for Fake {
        type Handle = u32;
        type Error = &'static str;
        fn upload(&mut self, w: u32, h: u32, rgba: &[u8]) -> Result<u32, Self::Error> {
            assert_eq!(rgba.len(), w as usize * h as usize * 4);
            let mut state = self.0.borrow_mut();
            state.attempts += 1;
            if let Some(handle) = state.alias {
                return Ok(handle);
            }
            state.next += 1;
            let handle = state.next;
            assert!(state.live.insert(handle));
            if state.fail {
                // Fake models the real backend's partial-allocation obligation.
                assert!(state.live.remove(&handle));
                state.deleted.push(handle);
                return Err("allocation declined");
            }
            Ok(handle)
        }
        fn delete(&mut self, handle: u32) {
            let mut state = self.0.borrow_mut();
            assert!(state.live.remove(&handle), "foreign or double deletion");
            state.deleted.push(handle);
        }
    }
    struct Frame {
        key: ResidentKey,
        fingerprint: Fingerprint,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    }
    impl Frame {
        fn new(id: &str, revision: u64, width: u32, height: u32) -> Self {
            Self {
                key: ResidentKey {
                    source: SourceKey {
                        app_id: id.into(),
                        version: "1".into(),
                        class: RootClass::LgSystem,
                        folder: format!("/usr/palm/applications/{id}"),
                        relative: vec!["icons".into(), "icon.png".into()],
                        kind: IconKind::Icon,
                    },
                    revision,
                },
                fingerprint: Fingerprint {
                    content_sha256: [1; 32],
                    file_identity: FileIdentity {
                        device: 1,
                        inode: 1,
                        bytes: 100,
                        mode: 0o100644,
                        uid: 0,
                        gid: 0,
                        modified_seconds: 1,
                        modified_nanos: 0,
                        changed_seconds: 1,
                        changed_nanos: 0,
                    },
                },
                width,
                height,
                rgba: vec![1; width as usize * height as usize * 4],
            }
        }
        fn view(&self) -> ResidentView<'_> {
            ResidentView {
                key: &self.key,
                fingerprint: &self.fingerprint,
                width: self.width,
                height: self.height,
                rgba: &self.rgba,
            }
        }
    }
    fn store(config: Config) -> (ArtworkTextures<Fake>, Rc<RefCell<State>>) {
        let state = Rc::new(RefCell::new(State::default()));
        (
            ArtworkTextures::new(Fake(state.clone()), config).unwrap(),
            state,
        )
    }
    #[test]
    fn uploaded_handle_shape_and_exact_key_are_owned_until_eviction() {
        let (mut textures, state) = store(Config::default());
        let frame = Frame::new("app.a", 1, 3, 2);
        assert_eq!(
            textures.install(&frame.view()),
            Ok(InstallOutcome::Uploaded { replaced: false })
        );
        let view = textures.texture("app.a").unwrap();
        assert_eq!((view.width, view.height, view.handle), (3, 2, 1));
        assert_eq!(view.key, &frame.key);
        assert_eq!(
            textures.stats(),
            Stats {
                entries: 1,
                rgba_bytes_estimate: 24
            }
        );
        assert!(textures.evict(&frame.key));
        assert!(!textures.evict(&frame.key));
        assert_eq!(state.borrow().deleted, vec![1]);
        assert_eq!(textures.stats().entries, 0);
    }
    #[test]
    fn replacement_failure_cleans_partial_upload_and_preserves_old_texture() {
        let (mut textures, state) = store(Config::default());
        let old = Frame::new("app.a", 1, 2, 2);
        textures.install(&old.view()).unwrap();
        state.borrow_mut().fail = true;
        let new = Frame::new("app.a", 2, 4, 2);
        assert_eq!(
            textures.install(&new.view()),
            Err(InstallError::Backend("allocation declined"))
        );
        assert_eq!(textures.texture("app.a").unwrap().key, &old.key);
        assert_eq!(textures.stats().rgba_bytes_estimate, 16);
        assert_eq!(state.borrow().live, BTreeSet::from([1]));
        assert_eq!(state.borrow().deleted, vec![2]);
        state.borrow_mut().fail = false;
        textures.install(&new.view()).unwrap();
        assert_eq!(textures.texture("app.a").unwrap().handle, 3);
        assert_eq!(textures.stats().rgba_bytes_estimate, 32);
        assert_eq!(state.borrow().deleted, vec![2, 1]);
        drop(textures);
        assert_eq!(state.borrow().deleted, vec![2, 1, 3]);
    }
    #[test]
    fn stale_eviction_or_install_never_removes_fresh_revision() {
        let (mut textures, state) = store(Config::default());
        let old = Frame::new("app.a", 10, 1, 1);
        textures.install(&old.view()).unwrap();
        let mut new = Frame::new("app.a", 11, 1, 1);
        new.key.source.version = "2".into();
        textures.install(&new.view()).unwrap();
        assert!(!textures.evict(&old.key));
        assert_eq!(
            textures.install(&old.view()),
            Err(InstallError::StaleRevision)
        );
        let mut wrong = new.key.clone();
        wrong.source.relative = vec!["different.png".into()];
        assert!(!textures.evict(&wrong));
        assert_eq!(textures.texture("app.a").unwrap().key, &new.key);
        assert_eq!(state.borrow().attempts, 2);
        assert_eq!(state.borrow().deleted, vec![1]);
    }
    #[test]
    fn exact_revision_is_idempotent_but_shape_or_source_conflicts_are_rejected() {
        let (mut textures, state) = store(Config::default());
        let frame = Frame::new("app.a", 1, 2, 2);
        textures.install(&frame.view()).unwrap();
        assert_eq!(
            textures.install(&frame.view()),
            Ok(InstallOutcome::AlreadyPresent)
        );
        let changed = Frame::new("app.a", 1, 4, 1);
        assert_eq!(
            textures.install(&changed.view()),
            Err(InstallError::ConflictingRevision)
        );
        let mut changed = Frame::new("app.a", 1, 2, 2);
        changed.key.source.version = "2".into();
        assert_eq!(
            textures.install(&changed.view()),
            Err(InstallError::ConflictingRevision)
        );
        assert_eq!(state.borrow().attempts, 1);
        assert!(state.borrow().deleted.is_empty());
    }
    #[test]
    fn byte_and_entry_caps_decline_before_backend_allocation() {
        let (mut textures, state) = store(Config {
            max_entries: 2,
            max_rgba_bytes: 8,
        });
        textures
            .install(&Frame::new("app.a", 1, 1, 1).view())
            .unwrap();
        textures
            .install(&Frame::new("app.b", 2, 1, 1).view())
            .unwrap();
        assert_eq!(
            textures.install(&Frame::new("app.c", 3, 1, 1).view()),
            Err(InstallError::Capacity)
        );
        assert_eq!(
            textures.install(&Frame::new("app.a", 4, 2, 1).view()),
            Err(InstallError::Capacity)
        );
        assert_eq!(state.borrow().attempts, 2);
        assert_eq!(
            textures.stats(),
            Stats {
                entries: 2,
                rgba_bytes_estimate: 8
            }
        );
        let (mut textures, state) = store(Config {
            max_entries: 1,
            max_rgba_bytes: 64,
        });
        textures
            .install(&Frame::new("app.a", 1, 1, 1).view())
            .unwrap();
        assert_eq!(
            textures.install(&Frame::new("app.b", 2, 1, 1).view()),
            Err(InstallError::Capacity)
        );
        assert_eq!(state.borrow().attempts, 1);
    }
    #[test]
    fn production_pixel_budget_and_maximum_axes_are_checked() {
        let (mut textures, state) = store(Config::default());
        for n in 0..8 {
            textures
                .install(&Frame::new(&format!("app.{n}"), n + 1, 512, 512).view())
                .unwrap();
        }
        assert_eq!(textures.stats().rgba_bytes_estimate, MAX_RESIDENT_BYTES);
        assert_eq!(
            textures.install(&Frame::new("app.extra", 9, 512, 512).view()),
            Err(InstallError::Capacity)
        );
        assert_eq!(state.borrow().attempts, 8);
        textures.clear();
        assert_eq!(textures.stats().rgba_bytes_estimate, 0);
        assert!(state.borrow().live.is_empty());
    }
    #[test]
    fn default_count_limit_accepts_512_small_textures_and_rejects_513th() {
        let (mut textures, state) = store(Config::default());
        for n in 0..512 {
            textures
                .install(&Frame::new(&format!("app.{n}"), n + 1, 1, 1).view())
                .unwrap();
        }
        assert_eq!(textures.stats().entries, MAX_ENTRIES);
        assert_eq!(
            textures.install(&Frame::new("app.extra", 513, 1, 1).view()),
            Err(InstallError::Capacity)
        );
        assert_eq!(state.borrow().attempts, 512);
    }
    #[test]
    fn malformed_buffers_and_dimensions_never_reach_backend() {
        let (mut textures, state) = store(Config::default());
        for (w, h, len) in [
            (0, 1, 0),
            (1, 0, 0),
            (513, 1, 2052),
            (1, 513, 2052),
            (u32::MAX, u32::MAX, 4),
            (2, 2, 15),
            (1, 1, 5),
        ] {
            let mut frame = Frame::new("app.a", 1, 1, 1);
            frame.width = w;
            frame.height = h;
            frame.rgba = vec![0; len];
            assert_eq!(
                textures.install(&frame.view()),
                Err(InstallError::InvalidImage)
            );
        }
        assert_eq!(state.borrow().attempts, 0);
    }
    #[test]
    fn malformed_keys_cannot_introduce_parent_paths_or_unbounded_metadata() {
        let (mut textures, state) = store(Config::default());
        let mut cases = Vec::new();
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.revision = 0;
        cases.push(frame);
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.source.app_id = "../app.a".into();
        cases.push(frame);
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.source.folder = "/etc/app.a".into();
        cases.push(frame);
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.source.class = RootClass::DeveloperApps;
        cases.push(frame);
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.source.version = "v".repeat(129);
        cases.push(frame);
        let mut frame = Frame::new("app.a", 1, 1, 1);
        frame.key.source.version = "1\n".into();
        cases.push(frame);
        for parts in [
            vec![],
            vec![".."],
            vec!["/absolute"],
            vec!["a/b"],
            vec!["a\\b"],
            vec!["file:uri"],
            vec![""],
            vec!["a"; 9],
        ] {
            let mut frame = Frame::new("app.a", 1, 1, 1);
            frame.key.source.relative = parts.into_iter().map(str::to_owned).collect();
            cases.push(frame);
        }
        for frame in cases {
            assert_eq!(
                textures.install(&frame.view()),
                Err(InstallError::InvalidKey)
            );
        }
        assert_eq!(state.borrow().attempts, 0);
    }
    #[test]
    fn backend_alias_error_does_not_delete_a_still_owned_texture() {
        let (mut textures, state) = store(Config::default());
        let old = Frame::new("app.a", 1, 1, 1);
        textures.install(&old.view()).unwrap();
        state.borrow_mut().alias = Some(1);
        assert_eq!(
            textures.install(&Frame::new("app.a", 2, 1, 1).view()),
            Err(InstallError::AliasedHandle)
        );
        assert_eq!(textures.texture("app.a").unwrap().key, &old.key);
        assert!(state.borrow().deleted.is_empty());
        assert_eq!(textures.stats().entries, 1);
        state.borrow_mut().alias = None;
    }
    #[test]
    fn explicit_clear_and_drop_delete_only_owned_live_handles_once() {
        let (mut textures, state) = store(Config::default());
        state.borrow_mut().live.insert(9000); // Separate packaged fallback.
        textures
            .install(&Frame::new("app.a", 1, 1, 1).view())
            .unwrap();
        textures
            .install(&Frame::new("app.b", 2, 1, 1).view())
            .unwrap();
        textures.clear();
        textures.clear();
        assert_eq!(state.borrow().deleted, vec![1, 2]);
        assert_eq!(state.borrow().live, BTreeSet::from([9000]));
        textures
            .install(&Frame::new("app.c", 3, 1, 1).view())
            .unwrap();
        drop(textures);
        assert_eq!(state.borrow().deleted, vec![1, 2, 3]);
        assert_eq!(state.borrow().live, BTreeSet::from([9000]));
    }
    #[test]
    fn cache_eviction_before_failed_upload_leaves_packaged_fallback_ownership_alone() {
        let (mut textures, state) = store(Config::default());
        state.borrow_mut().live.insert(9000);
        let old = Frame::new("app.a", 1, 1, 1);
        textures.install(&old.view()).unwrap();
        assert!(textures.evict(&old.key));
        state.borrow_mut().fail = true;
        assert!(textures
            .install(&Frame::new("app.a", 2, 1, 1).view())
            .is_err());
        assert!(textures.texture("app.a").is_none());
        assert_eq!(state.borrow().live, BTreeSet::from([9000]));
        assert_eq!(textures.stats().rgba_bytes_estimate, 0);
    }
    #[test]
    fn limits_cannot_be_disabled_or_increased_past_configured_hard_caps() {
        for config in [
            Config {
                max_entries: 0,
                max_rgba_bytes: 4,
            },
            Config {
                max_entries: 513,
                max_rgba_bytes: 4,
            },
            Config {
                max_entries: 1,
                max_rgba_bytes: 0,
            },
            Config {
                max_entries: 1,
                max_rgba_bytes: MAX_RESIDENT_BYTES + 1,
            },
        ] {
            assert!(
                ArtworkTextures::new(Fake(Rc::new(RefCell::new(State::default()))), config)
                    .is_err()
            );
        }
    }
}
