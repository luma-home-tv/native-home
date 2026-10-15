//! Fixed core context GETs for the standalone Root proof helper. There is no
//! caller-selected executable, argument, URI, shell, fallback, or setting write.
//! Byte/deadline limits do not bound an uninterruptible kernel wait or spawn.
use serde_json::{json, Value};
use std::time::Instant;

const PROGRAM: &str = "/usr/bin/luna-send";
const OPERATIONS: [(&str, &str, &str); 3] = [
    (
        "system_info",
        "luna://com.webos.service.tv.systemproperty/getSystemInfo",
        "{\"keys\":[\"modelName\",\"firmwareVersion\",\"sdkVersion\"]}",
    ),
    (
        "foreground",
        "luna://com.webos.applicationManager/getForegroundAppInfo",
        "{}",
    ),
    (
        "windows",
        "luna://com.webos.surfacemanager/getForegroundWindowInfo",
        "{}",
    ),
];
const CATALOGUE_OPERATIONS: [(&str, &str, &str); 2] = [
    ("apps", "luna://com.webos.applicationManager/listApps", "{}"),
    (
        "launch_points",
        "luna://com.webos.applicationManager/listLaunchPoints",
        "{}",
    ),
];
#[derive(Clone, Copy)]
enum ReadSet {
    Context,
    Catalogue,
}
impl ReadSet {
    fn operations(self) -> &'static [(&'static str, &'static str, &'static str)] {
        match self {
            Self::Context => &OPERATIONS,
            Self::Catalogue => &CATALOGUE_OPERATIONS,
        }
    }
    #[cfg(any(all(target_os = "linux", target_arch = "arm"), test))]
    fn stdout_cap(self) -> usize {
        match self {
            Self::Context => 65536,
            Self::Catalogue => 2 * 1024 * 1024,
        }
    }
    #[cfg(any(all(target_os = "linux", target_arch = "arm"), test))]
    fn operation_budget(self) -> std::time::Duration {
        std::time::Duration::from_millis(match self {
            Self::Context => 700,
            Self::Catalogue => 1000,
        })
    }
    #[cfg(any(all(target_os = "linux", target_arch = "arm"), test))]
    fn child_stage(self) -> &'static str {
        match self {
            Self::Context => "context_child_started",
            Self::Catalogue => "catalogue_child_started",
        }
    }
}

fn unavailable() -> Value {
    json!({"unavailable":true})
}
fn base_report(set: ReadSet) -> Value {
    let mut result = serde_json::Map::new();
    for (key, _, _) in set.operations() {
        result.insert((*key).to_owned(), unavailable());
    }
    result.insert("_reader".into(),json!({"schema":1,"kind":"fixed_core_luna_send","program":PROGRAM,"approved":false,"qualified":false,
        "executable":null,"executable_after":null,"operations":[],"cleanup_known":true,"error":null}));
    Value::Object(result)
}
fn collect_set(set: ReadSet, deadline: Instant, expected_sha256: &str) -> Value {
    let mut report = base_report(set);
    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    core::collect_into(&mut report, deadline, expected_sha256, set);
    #[cfg(not(all(target_os = "linux", target_arch = "arm")))]
    {
        let _ = (deadline, expected_sha256);
        report["_reader"]["error"] = json!("unsupported_platform");
    }
    report
}
pub fn collect(deadline: Instant, expected_sha256: &str) -> Value {
    collect_set(ReadSet::Context, deadline, expected_sha256)
}
/// Only these two catalogue GETs; no caller-selected route. The returned full
/// catalogue is ephemeral helper input, not a payload to print to helper stdout.
pub fn catalogue(deadline: Instant, expected_sha256: &str) -> Value {
    collect_set(ReadSet::Catalogue, deadline, expected_sha256)
}

#[cfg(any(all(target_os = "linux", target_arch = "arm"), test))]
mod core {
    use super::*;
    use serde::Serialize;
    use std::{
        ffi::CString,
        fs::{File, OpenOptions},
        io::{self, Read},
        os::{
            fd::{AsRawFd, FromRawFd, RawFd},
            unix::{
                fs::{FileExt, MetadataExt, OpenOptionsExt},
                process::{CommandExt, ExitStatusExt},
            },
        },
        process::{Child, Command, Stdio},
        time::Duration,
    };
    const MAX_CORE: u64 = 8 * 1024 * 1024;
    const OUT_CAP: usize = 65536;
    const ERR_CAP: usize = 8192;
    const CLEANUP: Duration = Duration::from_millis(100);
    fn invalid() -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "core executable admission declined",
        )
    }
    fn digest(bytes: &[u8]) -> String {
        plx_base::sha256::sha256(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    struct Identity {
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
    impl Identity {
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
    struct Measurement {
        sha256: String,
        identity: Identity,
    }
    struct Executable {
        file: File,
        parent: File,
        name: CString,
        initial: Measurement,
    }
    fn directory(meta: &std::fs::Metadata) -> bool {
        meta.is_dir() && meta.uid() == 0 && meta.gid() == 0 && meta.mode() & 0o022 == 0
    }
    fn regular(meta: &std::fs::Metadata) -> bool {
        meta.is_file()
            && meta.uid() == 0
            && meta.gid() == 0
            && meta.nlink() == 1
            && matches!(meta.mode() & 0o7777, 0o700 | 0o755 | 0o555)
            && meta.len() >= 52
            && meta.len() <= MAX_CORE
    }
    fn arm_elf(bytes: &[u8]) -> bool {
        bytes.len() >= 52
            && bytes[..7] == [0x7f, b'E', b'L', b'F', 1, 1, 1]
            && matches!(u16::from_le_bytes([bytes[16], bytes[17]]), 2 | 3)
            && u16::from_le_bytes([bytes[18], bytes[19]]) == 40
            && u32::from_le_bytes(bytes[20..24].try_into().expect("ELF version")) == 1
    }
    fn open_component(parent: RawFd, name: &CString, dir: bool) -> io::Result<File> {
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if dir {
                libc::O_DIRECTORY
            } else {
                libc::O_NONBLOCK
            };
        let fd = unsafe { libc::openat(parent, name.as_ptr(), flags) };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    }
    impl Executable {
        fn parent() -> io::Result<File> {
            let root = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/")?;
            if !directory(&root.metadata()?) {
                return Err(invalid());
            }
            let mut parent = root;
            for part in ["usr", "bin"] {
                parent = open_component(
                    parent.as_raw_fd(),
                    &CString::new(part).expect("literal component"),
                    true,
                )?;
                if !directory(&parent.metadata()?) {
                    return Err(invalid());
                }
            }
            Ok(parent)
        }
        fn admit() -> io::Result<Self> {
            let parent = Self::parent()?;
            let name = CString::new("luna-send").expect("literal name");
            let file = open_component(parent.as_raw_fd(), &name, false)?;
            if file.as_raw_fd() <= 2 {
                return Err(invalid());
            }
            let initial = measure(&file)?;
            let value = Self {
                file,
                parent,
                name,
                initial,
            };
            value.verify()?;
            Ok(value)
        }
        fn verify(&self) -> io::Result<Measurement> {
            // Rewalk the fixed full name as well as retaining the admitted FD.
            let current_parent = Self::parent()?;
            let old = self.parent.metadata()?;
            let new = current_parent.metadata()?;
            if (old.dev(), old.ino()) != (new.dev(), new.ino()) {
                return Err(invalid());
            }
            let named = open_component(current_parent.as_raw_fd(), &self.name, false)?;
            if !regular(&named.metadata()?)
                || Identity::of(&named.metadata()?) != self.initial.identity
            {
                return Err(invalid());
            }
            let current = measure(&self.file)?;
            if current != self.initial || measure(&named)? != self.initial {
                return Err(invalid());
            }
            Ok(current)
        }
    }
    fn measure(file: &File) -> io::Result<Measurement> {
        let before = file.metadata()?;
        if !regular(&before) {
            return Err(invalid());
        }
        let identity = Identity::of(&before);
        let mut hash = plx_base::sha256::Sha256::new();
        let mut buffer = [0_u8; 32768];
        let mut offset = 0_u64;
        let mut header = [0_u8; 52];
        loop {
            let permitted = ((MAX_CORE - offset + 1) as usize).min(buffer.len());
            let n = match file.read_at(&mut buffer[..permitted], offset) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                value => value?,
            };
            if n == 0 {
                break;
            }
            if offset == 0 {
                if n < 52 {
                    return Err(invalid());
                }
                header.copy_from_slice(&buffer[..52]);
            }
            offset += n as u64;
            if offset > MAX_CORE {
                return Err(invalid());
            }
            hash.update(&buffer[..n]);
        }
        if offset != identity.bytes
            || !arm_elf(&header)
            || Identity::of(&file.metadata()?) != identity
        {
            return Err(invalid());
        }
        Ok(Measurement {
            sha256: hash.finish().iter().map(|b| format!("{b:02x}")).collect(),
            identity,
        })
    }
    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    fn prepare_parent() -> io::Result<()> {
        // Only the standalone Root helper reaches this process-global setup.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = libc::SIG_DFL;
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0
            || unsafe { libc::sigaction(libc::SIGCHLD, &action, std::ptr::null_mut()) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut mask: libc::sigset_t = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigemptyset(&mut mask) } != 0
            || unsafe { libc::sigaddset(&mut mask, libc::SIGCHLD) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let result =
            unsafe { libc::pthread_sigmask(libc::SIG_UNBLOCK, &mask, std::ptr::null_mut()) };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result));
        }
        // Include any inherited SSH/runtime descriptors, not only our own sockets.
        // Command's later-created error pipe is itself CLOEXEC.
        for entry in std::fs::read_dir("/proc/self/fd")? {
            let entry = entry?;
            let fd = entry
                .file_name()
                .to_string_lossy()
                .parse::<RawFd>()
                .map_err(|_| invalid())?;
            if fd <= 2 {
                continue;
            }
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    fn spawn_fixed(executable: &Executable, uri: &str, payload: &str) -> io::Result<Child> {
        let arguments = [PROGRAM, "-i", "-n", "1", "-w", "5000", "-f", uri, payload]
            .map(|v| CString::new(v).expect("static argument"));
        let environment = [
            CString::new("PATH=/usr/bin:/bin").expect("static env"),
            CString::new("LANG=C").expect("static env"),
        ];
        let fd = executable.file.as_raw_fd();
        let parent = unsafe { libc::getpid() };
        let mut command = Command::new(PROGRAM);
        command
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            command.pre_exec(move || {
                if libc::setpgid(0, 0) != 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::prctl(
                    libc::PR_SET_PDEATHSIG,
                    libc::SIGKILL as libc::c_ulong,
                    0,
                    0,
                    0,
                ) != 0
                {
                    return Err(io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    libc::_exit(126);
                }
                let argv = [
                    arguments[0].as_ptr(),
                    arguments[1].as_ptr(),
                    arguments[2].as_ptr(),
                    arguments[3].as_ptr(),
                    arguments[4].as_ptr(),
                    arguments[5].as_ptr(),
                    arguments[6].as_ptr(),
                    arguments[7].as_ptr(),
                    arguments[8].as_ptr(),
                    std::ptr::null(),
                ];
                let env = [
                    environment[0].as_ptr(),
                    environment[1].as_ptr(),
                    std::ptr::null(),
                ];
                libc::fexecve(fd, argv.as_ptr(), env.as_ptr());
                // Never return Ok: Command must not perform a pathname exec fallback.
                Err(io::Error::last_os_error())
            });
        }
        command.spawn()
    }
    struct Pipe {
        file: File,
        bytes: Vec<u8>,
        cap: usize,
        eof: bool,
        overflow: bool,
    }
    impl Pipe {
        fn new(fd: RawFd, cap: usize) -> io::Result<Self> {
            let file = unsafe { File::from_raw_fd(fd) };
            let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
            if flags < 0
                || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                    < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                file,
                bytes: Vec::new(),
                cap,
                eof: false,
                overflow: false,
            })
        }
        fn drain(&mut self) -> io::Result<()> {
            if self.eof {
                return Ok(());
            }
            let mut buffer = [0_u8; 4096];
            // At most bounded data per poll turn, even during overflow cleanup.
            for _ in 0..32 {
                let n = match self.file.read(&mut buffer) {
                    Ok(0) => {
                        self.eof = true;
                        return Ok(());
                    }
                    Ok(n) => n,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                    Err(e) => return Err(e),
                };
                let remaining = self.cap.saturating_add(1).saturating_sub(self.bytes.len());
                self.bytes.extend_from_slice(&buffer[..n.min(remaining)]);
                if self.bytes.len() > self.cap {
                    self.overflow = true;
                }
            }
            Ok(())
        }
    }
    fn observed_exit(pid: u32) -> io::Result<bool> {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        #[cfg(target_os = "linux")]
        let observed = unsafe { info.si_pid() };
        #[cfg(not(target_os = "linux"))]
        let observed = info.si_pid;
        Ok(observed == pid as i32)
    }
    fn reap_nonblocking(pid: u32) -> io::Result<Option<std::process::ExitStatus>> {
        let mut status = 0;
        let rc = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((rc > 0).then(|| std::process::ExitStatus::from_raw(status)))
    }
    fn group_kill(pid: u32) -> io::Result<()> {
        if pid <= 1 {
            return Err(invalid());
        }
        if unsafe { libc::kill(-(pid as i32), libc::SIGKILL) } != 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::ESRCH) {
                return Err(e);
            }
        }
        Ok(())
    }
    fn poll_pipes(out: &Pipe, err: &Pipe, deadline: Instant) -> io::Result<()> {
        let mut fds = [
            libc::pollfd {
                fd: if out.eof { -1 } else { out.file.as_raw_fd() },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if err.eof { -1 } else { err.file.as_raw_fd() },
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let milliseconds = remaining.as_millis().clamp(1, 10) as i32;
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, milliseconds) } < 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
        Ok(())
    }
    #[derive(Debug)]
    struct Outcome {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit_code: Option<i32>,
        signal: Option<i32>,
        timed_out: bool,
        reaped: bool,
        stdout_eof: bool,
        stderr_eof: bool,
        cleanup_known: bool,
        group_cleanup_attempted: bool,
        group_kill_errno: Option<i32>,
        error: Option<&'static str>,
        pid: u32,
        held_empty_stdin: bool,
    }
    fn run_child(child: Child, deadline: Instant) -> Outcome {
        run_child_bounded(child, deadline, OUT_CAP)
    }
    fn run_child_bounded(mut child: Child, deadline: Instant, stdout_cap: usize) -> Outcome {
        use std::os::fd::IntoRawFd;
        let pid = child.id();
        // A private empty pipe avoids luna-send interactive EOF before callback.
        // Its writer is held throughout monitoring; no branch writes any bytes.
        let held_stdin = child.stdin.take();
        let mut result = Outcome {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: None,
            signal: None,
            timed_out: false,
            reaped: false,
            stdout_eof: false,
            stderr_eof: false,
            cleanup_known: false,
            group_cleanup_attempted: false,
            group_kill_errno: None,
            error: None,
            pid,
            held_empty_stdin: held_stdin.is_some(),
        };
        // Stdio ownership is extracted before polling; no wait/try_wait can reap
        // the leader until group cleanup has run while its PID is still pinned.
        let out = child
            .stdout
            .take()
            .ok_or_else(invalid)
            .and_then(|f| Pipe::new(f.into_raw_fd(), stdout_cap));
        let err = child
            .stderr
            .take()
            .ok_or_else(invalid)
            .and_then(|f| Pipe::new(f.into_raw_fd(), ERR_CAP));
        let run_deadline = deadline.checked_sub(CLEANUP).unwrap_or(deadline);
        if let (Ok(mut out), Ok(mut err)) = (out, err) {
            loop {
                if out.drain().is_err() || err.drain().is_err() {
                    result.error = Some("pipe_read_failed");
                    break;
                }
                if out.overflow || err.overflow {
                    result.error = Some("output_limit");
                    break;
                }
                match observed_exit(pid) {
                    Ok(true) if out.eof && err.eof => break,
                    Ok(_) => (),
                    Err(_) => {
                        result.error = Some("exit_observation_failed");
                        break;
                    }
                }
                if Instant::now() >= run_deadline {
                    result.timed_out = true;
                    result.error = Some("deadline");
                    break;
                }
                if poll_pipes(&out, &err, run_deadline).is_err() {
                    result.error = Some("pipe_poll_failed");
                    break;
                }
            }
            // EVERY outcome (including successful leader exit/EOF) cleans only
            // the owned group before leader reap. No post-reap group signal.
            result.group_cleanup_attempted = true;
            if let Err(e) = group_kill(pid) {
                result.group_kill_errno = e.raw_os_error();
                result.error = Some("group_cleanup_failed");
            }
            loop {
                if out.drain().is_err() || err.drain().is_err() {
                    result.error = Some("cleanup_pipe_failed");
                    break;
                }
                match observed_exit(pid) {
                    Ok(true) if out.eof && err.eof => match reap_nonblocking(pid) {
                        Ok(Some(status)) => {
                            result.exit_code = status.code();
                            result.signal = status.signal();
                            result.reaped = true;
                            break;
                        }
                        Ok(None) => (),
                        Err(_) => {
                            result.error = Some("reap_failed");
                            break;
                        }
                    },
                    Ok(_) => (),
                    Err(_) => {
                        result.error = Some("cleanup_observation_failed");
                        break;
                    }
                }
                if Instant::now() >= deadline {
                    result.error = Some("cleanup_unknown");
                    break;
                }
                if poll_pipes(&out, &err, deadline).is_err() {
                    result.error = Some("cleanup_poll_failed");
                    break;
                }
            }
            result.stdout_eof = out.eof;
            result.stderr_eof = err.eof;
            result.stdout = out.bytes;
            result.stderr = err.bytes;
            result.cleanup_known = result.reaped
                && result.stdout_eof
                && result.stderr_eof
                && result.group_kill_errno.is_none();
        } else {
            result.error = Some("pipe_setup_failed");
            result.group_cleanup_attempted = true;
            if let Err(e) = group_kill(pid) {
                result.group_kill_errno = e.raw_os_error();
            }
            while Instant::now() < deadline {
                if let Ok(Some(status)) = reap_nonblocking(pid) {
                    result.exit_code = status.code();
                    result.signal = status.signal();
                    result.reaped = true;
                    break;
                }
                unsafe {
                    libc::poll(std::ptr::null_mut(), 0, 1);
                }
            }
            // Missing pipe observation cannot qualify cleanup.
        }
        drop(held_stdin);
        result
    }
    struct Distinct(Value);
    impl<'de> serde::Deserialize<'de> for Distinct {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Distinct;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("JSON without duplicate object keys")
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Distinct, E> {
                    Ok(Distinct(Value::Bool(v)))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Distinct, E> {
                    Ok(Distinct(Value::Number(v.into())))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Distinct, E> {
                    Ok(Distinct(Value::Number(v.into())))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Distinct, E> {
                    serde_json::Number::from_f64(v)
                        .map(|n| Distinct(Value::Number(n)))
                        .ok_or_else(|| E::custom("nonfinite JSON number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Distinct, E> {
                    Ok(Distinct(Value::String(v.to_owned())))
                }
                fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Distinct, E> {
                    Ok(Distinct(Value::String(v)))
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Distinct, E> {
                    Ok(Distinct(Value::Null))
                }
                fn visit_none<E: serde::de::Error>(self) -> Result<Distinct, E> {
                    Ok(Distinct(Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut sequence: A,
                ) -> Result<Distinct, A::Error> {
                    let mut values = Vec::new();
                    while let Some(Distinct(value)) = sequence.next_element::<Distinct>()? {
                        values.push(value);
                    }
                    Ok(Distinct(Value::Array(values)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Distinct, A::Error> {
                    let mut values = serde_json::Map::new();
                    while let Some(key) = map.next_key::<String>()? {
                        if values.contains_key(&key) {
                            return Err(serde::de::Error::custom("duplicate JSON object key"));
                        }
                        values.insert(key, map.next_value::<Distinct>()?.0);
                    }
                    Ok(Distinct(Value::Object(values)))
                }
            }
            deserializer.deserialize_any(Visitor)
        }
    }
    fn object(bytes: &[u8]) -> io::Result<Value> {
        let Distinct(value) = serde_json::from_slice::<Distinct>(bytes).map_err(|_| invalid())?;
        if !value.is_object() {
            return Err(invalid());
        }
        Ok(value)
    }
    fn operation_succeeded(
        outcome: &Outcome,
        parsed: &io::Result<Value>,
        core_unchanged: bool,
    ) -> bool {
        outcome.error.is_none()
            && outcome.held_empty_stdin
            && outcome.cleanup_known
            && outcome.exit_code == Some(0)
            && outcome.stderr.is_empty()
            && parsed
                .as_ref()
                .is_ok_and(|value| value["returnValue"].as_bool() == Some(true))
            && core_unchanged
    }
    fn reader_qualified(rows: &[Value], cleanup_known: bool, core_unchanged: bool) -> bool {
        reader_qualified_for(rows, cleanup_known, core_unchanged, ReadSet::Context)
    }
    fn reader_qualified_for(
        rows: &[Value],
        cleanup_known: bool,
        core_unchanged: bool,
        set: ReadSet,
    ) -> bool {
        rows.len() == set.operations().len()
            && cleanup_known
            && core_unchanged
            && rows
                .iter()
                .all(|row| row["success"].as_bool() == Some(true))
    }

    fn receipt(key: &str, uri: &str, payload: &str) -> Value {
        json!({"key":key,"uri":uri,"argv":[PROGRAM,"-i","-n","1","-w","5000","-f",uri,payload],"attempted":false,"success":false,
            "child_pid":null,"process_group":null,"exit_code":null,"signal":null,"timed_out":false,"stdout_bytes":0,"stderr_bytes":0,
            "stdout_sha256":digest(&[]),"stderr_sha256":digest(&[]),"stdout_eof":false,"stderr_eof":false,"child_reaped":false,
            "cleanup_known":true,"held_empty_stdin":false,"stdin_bytes_sent":0,"group_cleanup_attempted":false,"group_kill_errno":null,"executable_before":null,"executable_after":null,"error":null})
    }
    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    pub(super) fn collect_into(
        report: &mut Value,
        deadline: Instant,
        expected_sha256: &str,
        set: ReadSet,
    ) {
        if Instant::now() >= deadline {
            report["_reader"]["error"] = json!("deadline");
            return;
        }
        if expected_sha256.len() != 64
            || !expected_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            report["_reader"]["error"] = json!("invalid_expected_core_hash");
            return;
        }
        if unsafe { libc::getuid() } != 0
            || unsafe { libc::geteuid() } != 0
            || std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|s| s == "artwork-broker-proof"))
                != Some(true)
        {
            report["_reader"]["error"] = json!("standalone_root_helper_required");
            return;
        }
        let executable = match Executable::admit() {
            Ok(v) => v,
            Err(_) => {
                report["_reader"]["error"] = json!("core_admission_failed");
                return;
            }
        };
        report["_reader"]["executable"] = json!(executable.initial);
        if executable.initial.sha256 != expected_sha256 {
            report["_reader"]["error"] = json!("core_hash_mismatch");
            return;
        }
        report["_reader"]["approved"] = json!(true);
        if prepare_parent().is_err() {
            report["_reader"]["error"] = json!("parent_setup_failed");
            return;
        }
        let mut operations = Vec::new();
        let mut all = true;
        let mut clean = true;
        for &(key, uri, payload) in set.operations() {
            let mut row = receipt(key, uri, payload);
            let before = match executable.verify() {
                Ok(v) => v,
                Err(_) => {
                    row["error"] = json!("core_changed_before");
                    operations.push(row);
                    all = false;
                    break;
                }
            };
            row["executable_before"] = json!(before);
            if deadline.saturating_duration_since(Instant::now()) <= CLEANUP {
                row["error"] = json!("insufficient_deadline");
                operations.push(row);
                all = false;
                break;
            }
            let operation_deadline = deadline.min(Instant::now() + set.operation_budget());
            let child = match spawn_fixed(&executable, uri, payload) {
                Ok(v) => v,
                Err(_) => {
                    row["error"] = json!("spawn_failed");
                    operations.push(row);
                    all = false;
                    break;
                }
            };
            println!(
                "{}",
                json!({"schema":1,"stage":set.child_stage(),"child_pid":child.id(),"process_group":child.id(),"program_sha256":executable.initial.sha256})
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let outcome = match set {
                ReadSet::Context => run_child(child, operation_deadline),
                ReadSet::Catalogue => {
                    run_child_bounded(child, operation_deadline, set.stdout_cap())
                }
            };
            row["attempted"] = json!(true);
            row["child_pid"] = json!(outcome.pid);
            row["process_group"] = json!(outcome.pid);
            row["exit_code"] = json!(outcome.exit_code);
            row["signal"] = json!(outcome.signal);
            row["timed_out"] = json!(outcome.timed_out);
            row["stdout_bytes"] = json!(outcome.stdout.len());
            row["stderr_bytes"] = json!(outcome.stderr.len());
            row["stdout_sha256"] = json!(digest(&outcome.stdout));
            row["stderr_sha256"] = json!(digest(&outcome.stderr));
            row["held_empty_stdin"] = json!(outcome.held_empty_stdin);
            row["stdin_bytes_sent"] = json!(0);
            row["stdout_eof"] = json!(outcome.stdout_eof);
            row["stderr_eof"] = json!(outcome.stderr_eof);
            row["child_reaped"] = json!(outcome.reaped);
            row["cleanup_known"] = json!(outcome.cleanup_known);
            row["group_cleanup_attempted"] = json!(outcome.group_cleanup_attempted);
            row["group_kill_errno"] = json!(outcome.group_kill_errno);
            row["error"] = json!(outcome.error);
            clean &= outcome.cleanup_known;
            let after = executable.verify();
            if let Ok(value) = &after {
                row["executable_after"] = json!(value);
            }
            let parsed = object(&outcome.stdout);
            let success = operation_succeeded(&outcome, &parsed, after.is_ok());
            if success {
                report[key] = parsed.expect("checked object");
                row["success"] = json!(true);
            } else {
                all = false;
                if outcome.error.is_none() {
                    row["error"] = json!(if after.is_err() {
                        "core_changed_after"
                    } else if outcome.exit_code != Some(0) {
                        "nonzero_exit"
                    } else if !outcome.stderr.is_empty() {
                        "stderr_present"
                    } else if parsed.is_err() {
                        "invalid_json_object"
                    } else {
                        "service_refused"
                    });
                }
            }
            operations.push(row);
            if !clean || after.is_err() {
                break;
            }
        }
        let after = executable.verify();
        if let Ok(value) = &after {
            report["_reader"]["executable_after"] = json!(value);
        }
        report["_reader"]["qualified"] = json!(
            all && match set {
                ReadSet::Context => reader_qualified(&operations, clean, after.is_ok()),
                ReadSet::Catalogue => reader_qualified_for(&operations, clean, after.is_ok(), set),
            }
        );
        report["_reader"]["cleanup_known"] = json!(clean);
        report["_reader"]["operations"] = json!(operations);
        if report["_reader"]["qualified"] != true {
            report["_reader"]["error"] = json!("context_unqualified");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn spawn(script: &str) -> Child {
            let mut c = Command::new("/bin/sh");
            c.args(["-c", script])
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            unsafe {
                c.pre_exec(|| {
                    if libc::setpgid(0, 0) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            c.spawn().unwrap()
        }
        fn run(script: &str) -> Outcome {
            run_child(spawn(script), Instant::now() + Duration::from_secs(2))
        }
        fn assert_native_exit_cleanup(outcome: &Outcome) {
            assert!(
                outcome.reaped
                    && outcome.stdout_eof
                    && outcome.stderr_eof
                    && outcome.held_empty_stdin,
                "{outcome:?}"
            );
            #[cfg(target_os = "linux")]
            {
                assert_eq!(outcome.group_kill_errno, None, "{outcome:?}");
                assert!(outcome.cleanup_known, "{outcome:?}");
            }
            #[cfg(target_os = "macos")]
            {
                // Darwin rejects signaling an already zombie-only group with
                // EPERM. Preserve the production refusal; this is not Linux
                // group-cleanup qualification or a reason to waive the check.
                assert_eq!(outcome.group_kill_errno, Some(libc::EPERM), "{outcome:?}");
                assert_eq!(outcome.error, Some("group_cleanup_failed"));
                assert!(!outcome.cleanup_known);
            }
        }
        fn synthetic_known_cleanup(stdout: &[u8]) -> Outcome {
            // Classification fixture only. Captured command bytes are real;
            // these cleanup fields are deliberately synthetic policy inputs.
            Outcome {
                stdout: stdout.to_vec(),
                stderr: Vec::new(),
                exit_code: Some(0),
                signal: None,
                timed_out: false,
                reaped: true,
                stdout_eof: true,
                stderr_eof: true,
                cleanup_known: true,
                group_cleanup_attempted: true,
                group_kill_errno: None,
                error: None,
                pid: 42,
                held_empty_stdin: true,
            }
        }
        #[test]
        fn strict_one_object_and_trailing_whitespace_only() {
            assert_eq!(
                object(b"{\"returnValue\":true}\n \t").unwrap()["returnValue"],
                true
            );
            for b in [
                b"{}{}".as_slice(),
                b"[]",
                b"null",
                b"{bad",
                b"{} garbage",
                b"{\"returnValue\":false,\"returnValue\":true}",
                b"{\"windows\":[{\"appId\":\"other\",\"appId\":\"proof\"}]}",
            ] {
                assert!(object(b).is_err());
            }
        }
        #[test]
        fn non_core_hosts_decline_the_fixed_production_path() {
            // Invokes only the fixed read-only admission path, never a command.
            #[cfg(not(all(target_os = "linux", target_arch = "arm")))]
            assert!(Executable::admit().is_err());
        }
        #[test]
        fn catalogue_has_only_two_fixed_gets_and_separate_count_budget() {
            assert_eq!(
                ReadSet::Catalogue.operations(),
                &[
                    ("apps", "luna://com.webos.applicationManager/listApps", "{}"),
                    (
                        "launch_points",
                        "luna://com.webos.applicationManager/listLaunchPoints",
                        "{}"
                    ),
                ]
            );
            assert_eq!(ReadSet::Catalogue.stdout_cap(), 2 * 1024 * 1024);
            assert_eq!(
                ReadSet::Catalogue.operation_budget(),
                Duration::from_millis(1000)
            );
            assert_eq!(ReadSet::Catalogue.child_stage(), "catalogue_child_started");
            assert_eq!(ReadSet::Context.stdout_cap(), 65536);
            assert_eq!(
                ReadSet::Context.operation_budget(),
                Duration::from_millis(700)
            );
            assert_eq!(ReadSet::Context.child_stage(), "context_child_started");
            let rows = [json!({"success":true}), json!({"success":true})];
            assert!(reader_qualified_for(&rows, true, true, ReadSet::Catalogue));
            assert!(!reader_qualified(&rows, true, true));
            let three = [
                json!({"success":true}),
                json!({"success":true}),
                json!({"success":true}),
            ];
            assert!(!reader_qualified_for(
                &three,
                true,
                true,
                ReadSet::Catalogue
            ));
            assert!(!reader_qualified_for(
                &[json!({"success":true}), json!({"success":false})],
                true,
                true,
                ReadSet::Catalogue
            ));
            let report = base_report(ReadSet::Catalogue);
            assert_eq!(
                report
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
                ["apps", "launch_points", "_reader"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            );
        }
        #[test]
        fn catalogue_stdout_can_exceed_context_cap_without_changing_context_limit() {
            let output = run_child_bounded(
                spawn("printf '%070000d' 0"),
                Instant::now() + Duration::from_secs(2),
                ReadSet::Catalogue.stdout_cap(),
            );
            assert_eq!(output.stdout.len(), 70000);
            assert_ne!(output.error, Some("output_limit"));
            assert_native_exit_cleanup(&output);
            let capped = run("printf '%070000d' 0");
            assert_eq!(capped.stdout.len(), OUT_CAP + 1);
            assert!(capped.error.is_some());
            let oversized = run_child_bounded(
                spawn("printf '%02097153d' 0"),
                Instant::now() + Duration::from_secs(2),
                ReadSet::Catalogue.stdout_cap(),
            );
            assert_eq!(oversized.stdout.len(), ReadSet::Catalogue.stdout_cap() + 1);
            assert!(oversized.error.is_some());
        }
        #[test]
        fn root_cli_argv_is_fixed_in_receipt() {
            let (key, uri, payload) = OPERATIONS[0];
            let r = receipt(key, uri, payload);
            assert_eq!(
                r["argv"],
                json!([
                    PROGRAM,
                    "-i",
                    "-n",
                    "1",
                    "-w",
                    "5000",
                    "-f",
                    uri,
                    "{\"keys\":[\"modelName\",\"firmwareVersion\",\"sdkVersion\"]}"
                ])
            );
            assert_eq!(OPERATIONS[1].2, "{}");
            assert_eq!(OPERATIONS[2].2, "{}");
        }
        #[test]
        fn elf_header_requires_arm32_little_endian_executable() {
            let mut h = [0_u8; 52];
            h[..7].copy_from_slice(&[0x7f, b'E', b'L', b'F', 1, 1, 1]);
            h[16..18].copy_from_slice(&2_u16.to_le_bytes());
            h[18..20].copy_from_slice(&40_u16.to_le_bytes());
            h[20..24].copy_from_slice(&1_u32.to_le_bytes());
            assert!(arm_elf(&h));
            for index in [0, 4, 5, 6, 16, 18, 20] {
                let mut bad = h;
                bad[index] = 0;
                assert!(!arm_elf(&bad));
            }
        }
        #[test]
        fn synthetic_known_cleanup_get_classification_uses_actual_command_bytes() {
            let good = run("printf '%s' '{\"returnValue\":true}'");
            assert_native_exit_cleanup(&good);
            let mut synthetic = synthetic_known_cleanup(&good.stdout);
            assert!(operation_succeeded(
                &synthetic,
                &object(&synthetic.stdout),
                true
            ));
            assert!(reader_qualified(
                &[
                    json!({"success":true}),
                    json!({"success":true}),
                    json!({"success":true})
                ],
                true,
                true
            ));
            synthetic.cleanup_known = false;
            assert!(!operation_succeeded(
                &synthetic,
                &object(&synthetic.stdout),
                true
            ));
            assert!(!reader_qualified(
                &[
                    json!({"success":true}),
                    json!({"success":true}),
                    json!({"success":true})
                ],
                false,
                true
            ));
            for body in [
                r#"{"returnValue":false}"#,
                r#"{"returnValue":1}"#,
                r#"{"returnValue":"true"}"#,
            ] {
                let actual = run(&format!("printf '%s' '{body}'"));
                assert_native_exit_cleanup(&actual);
                let classified = synthetic_known_cleanup(&actual.stdout);
                let success = operation_succeeded(&classified, &object(&classified.stdout), true);
                assert!(!success);
                assert!(!reader_qualified(
                    &[
                        json!({"success":true}),
                        json!({"success":success}),
                        json!({"success":true})
                    ],
                    true,
                    true
                ));
            }
        }
        #[test]
        fn eof_and_nonzero_are_reaped_with_group_cleanup() {
            let good = run("printf '{}'");
            assert_eq!(good.exit_code, Some(0));
            assert!(good.pid > 1 && good.stdout_eof && good.stderr_eof && good.held_empty_stdin);
            assert_eq!(good.signal, None);
            assert_native_exit_cleanup(&good);
            assert!(good.group_cleanup_attempted);
            assert_eq!(good.stdout, b"{}");
            let bad = run("printf '{}';exit 7");
            assert_eq!(bad.exit_code, Some(7));
            assert_native_exit_cleanup(&bad);
        }
        #[test]
        fn stderr_and_stdout_limits_are_independent_and_bounded() {
            let out = run("while :; do printf 'abcdefghijklmnopqrstuvwxyz0123456789';done");
            assert_eq!(out.error, Some("output_limit"));
            assert_eq!(out.stdout.len(), OUT_CAP + 1);
            assert!(out.reaped);
            let err = run("while :; do printf 'abcdefghijklmnopqrstuvwxyz0123456789' >&2;done");
            assert_eq!(err.error, Some("output_limit"));
            assert_eq!(err.stderr.len(), ERR_CAP + 1);
            assert!(err.reaped);
        }
        #[test]
        fn deadline_kills_only_unreaped_owned_group() {
            let out = run_child(
                spawn("sleep 5"),
                Instant::now() + Duration::from_millis(200),
            );
            assert!(out.timed_out && out.group_cleanup_attempted && out.reaped);
            assert!(out.cleanup_known, "{out:?}");
        }
        #[test]
        fn held_private_stdin_does_not_deliver_eof_or_bytes() {
            let out = run_child(
                spawn("if read line;then printf received;else printf eof;fi"),
                Instant::now() + Duration::from_millis(200),
            );
            assert!(out.timed_out && out.reaped && out.held_empty_stdin);
            assert!(out.stdout.is_empty());
        }
        #[test]
        fn child_stdin_open_empty_probe() {
            if std::env::var("LUMA_CONTEXT_STDIN_FIXTURE").as_deref() != Ok("1") {
                return;
            }
            let flags = unsafe { libc::fcntl(0, libc::F_GETFL) };
            assert!(flags >= 0);
            assert_eq!(
                unsafe { libc::fcntl(0, libc::F_SETFL, flags | libc::O_NONBLOCK) },
                0
            );
            let mut byte = 0_u8;
            assert_eq!(
                unsafe { libc::read(0, (&mut byte as *mut u8).cast(), 1) },
                -1
            );
            assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
            println!("{}", json!({"stdin_empty_open":true}));
        }
        #[test]
        fn child_can_reply_without_reading_or_receiving_stdin() {
            let mut c = Command::new(std::env::current_exe().unwrap());
            c.args([
                "--exact",
                "broker_context::core::tests::child_stdin_open_empty_probe",
                "--nocapture",
            ])
            .env_clear()
            .env("LUMA_CONTEXT_STDIN_FIXTURE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            unsafe {
                c.pre_exec(|| {
                    if libc::setpgid(0, 0) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let out = run_child(c.spawn().unwrap(), Instant::now() + Duration::from_secs(2));
            assert_eq!(out.exit_code, Some(0));
            assert_native_exit_cleanup(&out);
            assert!(String::from_utf8(out.stdout)
                .unwrap()
                .contains(r#""stdin_empty_open":true"#));
        }
        #[test]
        fn exited_leader_with_descendant_pipes_is_not_reaped_early() {
            let out = run_child(
                spawn("sleep 5 & printf '{}';exit 0"),
                Instant::now() + Duration::from_millis(200),
            );
            assert!(
                out.timed_out && out.group_cleanup_attempted && out.reaped && out.cleanup_known
            );
            assert_eq!(out.exit_code, Some(0));
        }
        #[test]
        fn stderr_is_retained_for_decline_not_ignored() {
            let out = run("printf '{}';printf warning >&2");
            assert_eq!(out.exit_code, Some(0));
            assert_eq!(out.stderr, b"warning");
            assert_native_exit_cleanup(&out);
        }
    }
}
