//! Ordinary held Window only. The host measures scene policy; this probe never
//! starts a Target, Root operation, keyboard, preference or artwork transaction.
// Exact frozen copies include unused APIs; the probe does not call them.
// Gate-only host tests do not import or pretend to retest the frozen runtime.
#[cfg(not(test))]
#[allow(dead_code)]
mod identity;
#[cfg(not(test))]
#[allow(dead_code)]
mod runtime;
#[cfg(any(target_arch = "arm", test))]
use serde::Serialize;
use std::error::Error;
#[cfg(any(target_arch = "arm", test))]
trait Surface {
    fn poll_kind(&mut self) -> Option<u32>;
    fn clear(&mut self);
    fn swap(&mut self);
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Default, Serialize)]
struct RenderGate {
    stopped: bool,
    abort_requested: bool,
    departure_observed: bool,
    input_observed: bool,
    unknown_event: bool,
    clears: u64,
    swaps: u64,
    stop_counts: Option<[u64; 2]>,
    events: u64,
}
#[cfg(any(target_arch = "arm", test))]
impl RenderGate {
    fn drain(&mut self, surface: &mut impl Surface) {
        for _ in 0..256 {
            let Some(kind) = surface.poll_kind() else {
                return;
            };
            self.events += 1;
            if matches!(kind, 7 | 8) {
                self.departure_observed = true;
                self.stop();
            } else if kind == 4 {
                self.input_observed = true; // Motion suppresses rendering, not lifetime.
                self.stop();
            } else if matches!(kind, 1..=3 | 5 | 6 | 10 | 11) {
                self.input_observed = true;
                self.abort_requested = true;
                self.stop();
            } else if kind != 9 {
                self.unknown_event = true;
                self.abort_requested = true;
                self.stop();
            }
        }
        self.unknown_event = true;
        self.abort_requested = true;
        self.stop(); // Incomplete batch never permits GL; ordinary courtesy abort.
    }
    fn stop(&mut self) {
        self.stopped = true;
        self.stop_counts.get_or_insert([self.clears, self.swaps]);
    }
    fn frame(&mut self, surface: &mut impl Surface) {
        self.drain(surface);
        if self.stopped {
            return;
        }
        surface.clear();
        self.clears += 1;
        self.drain(surface);
        if self.stopped {
            return;
        }
        surface.swap();
        self.swaps += 1;
    }
}

#[cfg(all(target_arch = "arm", not(test)))]
mod native {
    use super::{runtime, RenderGate, Surface};
    use serde::Serialize;
    use serde_json::{json, Value};
    use std::{
        error::Error,
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    const APP: &str = "com.ash.nativehome.proof";
    const MAIN: &str = "surface-scene-proof";
    const LIFETIME: Duration = Duration::from_secs(60);
    const RUNTIME_SHA: &str = "04ca72f63b83eb2c233cf7b79ed46be682ca374db4631170e06798ec35ebe855";
    const IDENTITY_SHA: &str = "77993442e98479633419cd443ad62198a293f239e19f0622ee9211e036a8958b";
    const EVENTS_SHA: &str = "67fb9bbc76546d1efdc0e6b8f4b3f54b0f40de80dbdcaf0aedb464b4261989e2";
    const PRODUCTION: &str = "324d016a248d8c5d84b0b4fe89766d79bf3b54c5";
    fn invalid(message: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    struct FileIdentity {
        device: u64,
        inode: u64,
        bytes: u64,
        mode: u32,
        uid: u32,
        gid: u32,
        links: u64,
        modified_seconds: i64,
        modified_nanos: i64,
        changed_seconds: i64,
        changed_nanos: i64,
    }
    impl FileIdentity {
        fn of(m: &std::fs::Metadata) -> Self {
            Self {
                device: m.dev(),
                inode: m.ino(),
                bytes: m.len(),
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
                links: m.nlink(),
                modified_seconds: m.mtime(),
                modified_nanos: m.mtime_nsec(),
                changed_seconds: m.ctime(),
                changed_nanos: m.ctime_nsec(),
            }
        }
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    struct Own {
        pid: u32,
        uid_fields: [u32; 4],
        gid_fields: [u32; 4],
        start_ticks: u64,
        exe_path: String,
        exe_sha256: String,
        exe_identity: FileIdentity,
        root: Value,
    }
    fn bounded(path: &str, limit: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut raw = Vec::new();
        File::open(path)?.take(limit + 1).read_to_end(&mut raw)?;
        if raw.len() as u64 > limit {
            return Err(invalid("Own observation exceeded bound").into());
        }
        Ok(raw)
    }
    fn fields(status: &str, key: &str) -> Result<[u32; 4], Box<dyn Error>> {
        let rows = status
            .lines()
            .filter_map(|line| line.strip_prefix(key))
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            return Err(invalid("Missing or duplicate own status fields").into());
        }
        let values = rows[0]
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        values
            .try_into()
            .map_err(|_| invalid("Own status field count").into())
    }
    fn own() -> Result<Own, Box<dyn Error>> {
        let pid = std::process::id();
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        if pid <= 1 || uid < 1000 || uid == 5724 {
            return Err(invalid("Ordinary isolated Proof required").into());
        }
        let status = String::from_utf8(bounded("/proc/self/status", 16384)?)?;
        let uid_fields = fields(&status, "Uid:")?;
        let gid_fields = fields(&status, "Gid:")?;
        if uid_fields != [uid; 4] || gid_fields != [gid; 4] {
            return Err(invalid("Own kernel UID/GID mismatch").into());
        }
        let status_pid = status
            .lines()
            .filter_map(|line| line.strip_prefix("Pid:"))
            .collect::<Vec<_>>();
        if status_pid.len() != 1 || status_pid[0].trim().parse::<u32>()? != pid {
            return Err(invalid("Own status PID mismatch").into());
        }
        let stat = String::from_utf8(bounded("/proc/self/stat", 8192)?)?;
        let close = stat.rfind(')').ok_or_else(|| invalid("Own stat shape"))?;
        let start_ticks = stat[close + 1..]
            .split_whitespace()
            .nth(19)
            .ok_or_else(|| invalid("Own start absent"))?
            .parse::<u64>()?;
        if start_ticks == 0 {
            return Err(invalid("Own start zero").into());
        }
        let exe_path = std::fs::read_link("/proc/self/exe")?
            .into_os_string()
            .into_string()
            .map_err(|_| invalid("Own exe UTF8"))?;
        if exe_path != format!("/media/developer/apps/usr/palm/applications/{APP}/{MAIN}") {
            return Err(invalid("Unexpected own executable path").into());
        }
        let mut exe = File::open("/proc/self/exe")?;
        let before = FileIdentity::of(&exe.metadata()?);
        if before.mode & 0o170000 != 0o100000 || before.bytes < 52 || before.bytes > 8 * 1024 * 1024
        {
            return Err(invalid("Own ELF type/bound").into());
        }
        let mut bytes = Vec::new();
        (&mut exe)
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if FileIdentity::of(&exe.metadata()?) != before
            || bytes.len() as u64 != before.bytes
            || std::fs::read_link("/proc/self/exe")? != Path::new(&exe_path)
        {
            return Err(invalid("Own ELF changed").into());
        }
        let exe_sha256 = plx_base::sha256::sha256(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let root = match std::fs::read_link("/proc/self/root") {
            Ok(value) => json!({"value":value.to_str(),"error":null}),
            Err(error) => json!({"value":null,"error":format!("{:?}",error.kind())}),
        };
        Ok(Own {
            pid,
            uid_fields,
            gid_fields,
            start_ticks,
            exe_path,
            exe_sha256,
            exe_identity: before,
            root,
        })
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    struct DirectoryIdentity {
        device: u64,
        inode: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    }
    impl DirectoryIdentity {
        fn of(m: &std::fs::Metadata) -> Self {
            Self {
                device: m.dev(),
                inode: m.ino(),
                uid: m.uid(),
                gid: m.gid(),
                mode: m.mode(),
            }
        }
    }
    struct Output {
        _tmp: File,
        parent: File,
        leaf: File,
        parent_identity: DirectoryIdentity,
        leaf_identity: DirectoryIdentity,
        path: PathBuf,
        parent_created: bool,
    }
    #[derive(Clone, Copy)]
    enum Report {
        Ready,
        Complete,
    }
    impl Report {
        fn name(self) -> &'static str {
            match self {
                Self::Ready => "surface-scene-ready.json",
                Self::Complete => "surface-scene-complete.json",
            }
        }
    }
    impl Output {
        fn open(own: &Own) -> Result<Self, Box<dyn Error>> {
            let uid = own.uid_fields[0];
            let gid = own.gid_fields[0];
            // This precedes ALL framework runtime/log calls. Existing private700,
            // foreign, nonsticky and symlink parents are never widened or repaired.
            let tmp = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/tmp")?;
            let m = tmp.metadata()?;
            if !m.is_dir() || m.uid() != 0 || m.gid() != 0 || m.mode() & 0o7777 != 0o1777 {
                return Err(invalid("Unexpected own namespace tmp parent").into());
            }
            let name = std::ffi::CString::new(APP)?;
            let made = unsafe { libc::mkdirat(tmp.as_raw_fd(), name.as_ptr(), 0o1777) };
            let parent_created = made == 0;
            if made < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err(std::io::Error::last_os_error().into());
            }
            let fd = unsafe {
                libc::openat(
                    tmp.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let parent = unsafe { File::from_raw_fd(fd) };
            let m = parent.metadata()?;
            if parent_created {
                if !m.is_dir() || m.uid() != uid || m.gid() != gid {
                    return Err(invalid("Fresh own runtime parent mismatch").into());
                }
                parent.set_permissions(std::fs::Permissions::from_mode(0o1777))?;
            }
            let p = DirectoryIdentity::of(&parent.metadata()?);
            if p.mode & 0o170000 != 0o040000
                || p.mode & 0o7777 != 0o1777
                || !((p.uid == uid && p.gid == gid) || (p.uid == 0 && p.gid == 0))
            {
                return Err(invalid("Existing runtime parent refused").into());
            }
            let path = PathBuf::from(format!("/tmp/{APP}/surface-scene-{}", own.pid));
            if DirectoryIdentity::of(&std::fs::symlink_metadata(path.parent().unwrap())?) != p {
                return Err(invalid("Runtime parent replaced").into());
            }
            let name = std::ffi::CString::new(format!("surface-scene-{}", own.pid))?;
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let leaf = unsafe { File::from_raw_fd(fd) };
            let l = DirectoryIdentity::of(&leaf.metadata()?);
            if l.mode & 0o170000 != 0o040000
                || l.mode & 0o7777 != 0o700
                || l.uid != uid
                || l.gid != gid
                || DirectoryIdentity::of(&std::fs::symlink_metadata(&path)?) != l
            {
                return Err(invalid("Fresh private runtime leaf mismatch").into());
            }
            let output = Self {
                _tmp: tmp,
                parent,
                leaf,
                parent_identity: p,
                leaf_identity: l,
                path,
                parent_created,
            };
            output.verify()?;
            Ok(output)
        }
        fn verify(&self) -> Result<(), Box<dyn Error>> {
            if DirectoryIdentity::of(&self.parent.metadata()?) != self.parent_identity
                || DirectoryIdentity::of(&std::fs::symlink_metadata(self.path.parent().unwrap())?)
                    != self.parent_identity
                || DirectoryIdentity::of(&self.leaf.metadata()?) != self.leaf_identity
                || DirectoryIdentity::of(&std::fs::symlink_metadata(&self.path)?)
                    != self.leaf_identity
            {
                return Err(invalid("Runtime directory identity changed").into());
            }
            Ok(())
        }
        fn write(&self, report: Report, value: &Value) -> Result<(), Box<dyn Error>> {
            self.verify()?;
            let raw = serde_json::to_vec(value)?;
            if raw.len() > 16384 {
                return Err(invalid("Surface metadata exceeds bound").into());
            }
            let name = std::ffi::CString::new(report.name())?;
            let fd = unsafe {
                libc::openat(
                    self.leaf.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut file = unsafe { File::from_raw_fd(fd) };
            let m = file.metadata()?;
            let initial = DirectoryIdentity::of(&m);
            if !m.is_file()
                || m.nlink() != 1
                || initial.mode & 0o7777 != 0o600
                || initial.uid != self.leaf_identity.uid
                || initial.gid != self.leaf_identity.gid
            {
                return Err(invalid("Fresh report identity refused").into());
            }
            file.write_all(&raw)?;
            file.sync_all()?;
            let current = file.metadata()?;
            let named = std::fs::symlink_metadata(self.path.join(report.name()))?;
            if DirectoryIdentity::of(&current) != initial
                || DirectoryIdentity::of(&named) != initial
                || named.nlink() != 1
                || current.nlink() != 1
                || current.len() != raw.len() as u64
            {
                return Err(invalid("Report replaced or changed").into());
            }
            self.leaf.sync_all()?;
            self.verify()
        }
        fn report(&self) -> Value {
            json!({"path":self.path,"parent_created":self.parent_created,"parent_identity":self.parent_identity,"directory_identity":self.leaf_identity,"directory_created_new":true,"report_create_new_mode":0o600})
        }
    }
    #[cfg(target_arch = "arm")]
    struct NativeSurface<'a>(&'a runtime::Window);
    #[cfg(target_arch = "arm")]
    impl Surface for NativeSurface<'_> {
        fn poll_kind(&mut self) -> Option<u32> {
            runtime::poll().map(|event| event.kind)
        }
        fn clear(&mut self) {
            plx_gfx::gfx::frame_clear(0.0, 0.0, 0.0);
        }
        fn swap(&mut self) {
            self.0.swap();
        }
    }
    fn pins() -> Value {
        json!({"production_revision":PRODUCTION,"runtime_sha256":RUNTIME_SHA,"identity_sha256":IDENTITY_SHA,"events_sha256":EVENTS_SHA,"probe_glue_sha256":env!("SURFACE_PROBE_GLUE_SHA256")})
    }
    #[cfg(target_arch = "arm")]
    pub(super) fn run() -> Result<(), Box<dyn Error>> {
        let before = own()?;
        if plx_base::paths::app_id() != APP {
            return Err(invalid("Fixed Proof identity required").into());
        }
        let output = Output::open(&before)?; // Parent guard occurs before Window/framework logging.
        let window = match runtime::Window::new(true) {
            Ok(value) => value,
            Err(error) => {
                output.write(Report::Complete,&json!({"schema":1,"app_id":APP,"source":pins(),"own_before":before,"window_created":false,"exit_reason":"window_initialization_failed","error":error.to_string(),"native_scene_qualified":false}))?;
                return Err(error);
            }
        };
        let began = Instant::now();
        let mut surface = NativeSurface(&window);
        let mut gate = RenderGate::default();
        let mut ready = false;
        while began.elapsed() < LIFETIME && !gate.abort_requested {
            gate.frame(&mut surface);
            if !ready {
                output.write(Report::Ready,&json!({"schema":1,"app_id":APP,"source":pins(),"own":before,"runtime":output.report(),"window_created":true,"renderer":window.renderer,"first_swap_performed":gate.swaps>0,"requested_lifetime_ms":60000,"render":gate,"native_scene_qualified":false,"progress_is_not_authority":true}))?;
                ready = true;
            }
            if !gate.abort_requested {
                std::thread::sleep(Duration::from_millis(16));
            }
        }
        gate.drain(&mut surface);
        let after = own();
        gate.drain(&mut surface);
        let stable = after.as_ref().is_ok_and(|value| value == &before);
        let exit_reason = if gate.abort_requested {
            "courtesy_abort"
        } else {
            "fixed_lifetime_complete"
        };
        let full_lifetime_unaborted = !gate.abort_requested && began.elapsed() >= LIFETIME;
        output.write(Report::Complete,&json!({"schema":1,"app_id":APP,"source":pins(),"own_before":before,"own_after":after.as_ref().ok(),"own_after_error":after.as_ref().err().map(|error|error.to_string()),"runtime":output.report(),"window_created":true,"window_held_until_report":true,"exit_reason":exit_reason,"full_lifetime_unaborted":full_lifetime_unaborted,"held_ms":began.elapsed().as_millis(),"requested_lifetime_ms":60000,"render":gate,"identity_stable":stable,"no_draw_after_observed_stop":gate.stop_counts.is_none_or(|counts|counts==[gate.clears,gate.swaps]),"native_scene_qualified":false,"application_root_operations":false,"application_settings_operations":false,"application_keyboard_requested":false,"application_preferences_operations":false,"application_provider_operations":false,"application_child_processes_spawned":0,"application_threads_spawned":false,"argv_interpreted":false,"sdk_internal_activity_qualified":false,"process_exit_code_measured":false,"scope":"Ordinary application fixed-window/render gate only; SDK internal threads, process lifecycle and activity are unqualified; real scene and external lifecycle require host evidence"}))?;
        // Window/context still owned when completion is written. Normal Drop follows.
        Ok(())
    }
}
#[cfg(all(target_arch = "arm", not(test)))]
fn main() -> Result<(), Box<dyn Error>> {
    native::run()
}
#[cfg(any(not(target_arch = "arm"), test))]
fn main() -> Result<(), Box<dyn Error>> {
    Err(std::io::Error::other("Native ordinary Proof probe only").into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Fake {
        events: VecDeque<Option<u32>>,
        loss_during_clear: bool,
        clears: u64,
        swaps: u64,
    }
    impl Surface for Fake {
        fn poll_kind(&mut self) -> Option<u32> {
            self.events.pop_front().flatten()
        }
        fn clear(&mut self) {
            self.clears += 1;
            if self.loss_during_clear {
                self.events.push_front(Some(7));
                self.loss_during_clear = false;
            }
        }
        fn swap(&mut self) {
            self.swaps += 1;
        }
    }
    #[test]
    fn observed_departure_before_clear_or_between_clear_and_swap_never_resumes() {
        for during in [false, true] {
            let mut surface = Fake {
                events: if during {
                    VecDeque::new()
                } else {
                    VecDeque::from([Some(8), None])
                },
                loss_during_clear: during,
                clears: 0,
                swaps: 0,
            };
            let mut gate = RenderGate::default();
            gate.frame(&mut surface);
            assert!(gate.stopped && gate.departure_observed);
            assert_eq!(surface.clears, u64::from(during));
            assert_eq!(surface.swaps, 0);
            surface.events.extend([Some(9), None]);
            for _ in 0..3 {
                gate.frame(&mut surface);
            }
            assert_eq!(surface.clears, u64::from(during));
            assert_eq!(surface.swaps, 0);
            assert_eq!(gate.stop_counts, Some([gate.clears, gate.swaps]));
        }
    }
    #[test]
    fn courtesy_abort_is_discrete_input_or_unknown_not_expected_departure_or_motion() {
        for kind in [1, 2, 3, 5, 6, 10, 11, 12] {
            let mut surface = Fake {
                events: VecDeque::from([Some(kind), None]),
                loss_during_clear: false,
                clears: 0,
                swaps: 0,
            };
            let mut gate = RenderGate::default();
            gate.frame(&mut surface);
            assert!(gate.stopped && gate.abort_requested);
            assert_eq!((surface.clears, surface.swaps), (0, 0));
        }
        for kind in [4, 7, 8] {
            let mut surface = Fake {
                events: VecDeque::from([Some(kind), None, Some(9), None]),
                loss_during_clear: false,
                clears: 0,
                swaps: 0,
            };
            let mut gate = RenderGate::default();
            gate.frame(&mut surface);
            gate.frame(&mut surface);
            assert!(gate.stopped);
            assert!(!gate.abort_requested);
            assert_eq!((surface.clears, surface.swaps), (0, 0));
        }
        let mut surface = Fake {
            events: (0..256).map(|_| Some(9)).collect(),
            loss_during_clear: false,
            clears: 0,
            swaps: 0,
        };
        let mut gate = RenderGate::default();
        gate.frame(&mut surface);
        assert!(gate.abort_requested && gate.unknown_event);
        assert_eq!((surface.clears, surface.swaps), (0, 0));
    }
}
