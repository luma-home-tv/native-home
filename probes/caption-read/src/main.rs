//! One ordinary anonymous LS2 GET. No Window, setters, subprocesses or fallback.
#[cfg(target_arch = "arm")]
mod native {
    use serde_json::{json, Value};
    use std::ffi::CString;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::time::{Duration, Instant};

    const APP: &str = "com.ash.nativehome.captionreadproof";
    const PARENT: &str = "/tmp/luma-caption-read-proof";
    const URI: &str = "luna://com.webos.settingsservice/getSystemSettings";
    const PAYLOAD: &str =
        "{\"category\":\"caption\",\"keys\":[\"captionEnable\"],\"subscribe\":false}";
    const REPLY_LIMIT: usize = 2 * 1024 * 1024;
    fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
    fn c(value: &str) -> io::Result<CString> {
        CString::new(value).map_err(|_| invalid("NUL"))
    }
    fn bounded(path: &str, cap: usize) -> io::Result<String> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take((cap + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > cap {
            return Err(invalid("self proc bound"));
        }
        String::from_utf8(bytes).map_err(|_| invalid("self proc UTF8"))
    }
    fn fields(status: &str, key: &str) -> io::Result<Vec<u32>> {
        let mut rows = status.lines().filter_map(|line| line.strip_prefix(key));
        let values: Vec<u32> = rows
            .next()
            .ok_or_else(|| invalid("status field"))?
            .split_whitespace()
            .map(|n| n.parse().map_err(|_| invalid("status number")))
            .collect::<io::Result<_>>()?;
        if rows.next().is_some() || values.len() != 4 {
            return Err(invalid("status fields"));
        }
        Ok(values)
    }
    fn own() -> io::Result<Value> {
        let pid = std::process::id();
        let status = bounded("/proc/self/status", 65536)?;
        let status_pid: u32 = status
            .lines()
            .find_map(|line| line.strip_prefix("Pid:"))
            .ok_or_else(|| invalid("status PID"))?
            .trim()
            .parse()
            .map_err(|_| invalid("PID"))?;
        let uid = fields(&status, "Uid:")?;
        let gid = fields(&status, "Gid:")?;
        if pid <= 1
            || status_pid != pid
            || uid[0] < 1000
            || uid.iter().any(|n| *n != uid[0])
            || gid.iter().any(|n| *n != gid[0])
            || uid[1] != unsafe { libc::geteuid() }
            || gid[1] != unsafe { libc::getegid() }
        {
            return Err(invalid("ordinary self identity"));
        }
        let stat = bounded("/proc/self/stat", 8192)?;
        let (head, rest) = stat.rsplit_once(") ").ok_or_else(|| invalid("stat comm"))?;
        let stat_pid: u32 = head
            .split_once(" (")
            .ok_or_else(|| invalid("stat PID"))?
            .0
            .parse()
            .map_err(|_| invalid("stat PID"))?;
        let start: u64 = rest
            .split_whitespace()
            .nth(19)
            .ok_or_else(|| invalid("start ticks"))?
            .parse()
            .map_err(|_| invalid("start ticks"))?;
        if stat_pid != pid || start == 0 {
            return Err(invalid("stat identity"));
        }
        Ok(
            json!({"pid":pid,"uid_fields":uid,"gid_fields":gid,"start_ticks":start,
            "exe_path":std::fs::read_link("/proc/self/exe")?.to_str().ok_or_else(|| invalid("exe UTF8"))?,
            "root_path":std::fs::read_link("/proc/self/root")?.to_str().ok_or_else(|| invalid("root UTF8"))?}),
        )
    }
    fn stamp(file: &File) -> io::Result<Value> {
        let m = file.metadata()?;
        Ok(
            json!({"device":m.dev(),"inode":m.ino(),"uid":m.uid(),"gid":m.gid(),"mode":m.mode(),"links":m.nlink()}),
        )
    }
    fn tmp_identity(file: &File) -> io::Result<Value> {
        let mut identity = stamp(file)?;
        identity
            .as_object_mut()
            .ok_or_else(|| invalid("directory stamp"))?
            .remove("links");
        Ok(identity)
    }
    fn open_dir(parent: &File, name: &str) -> io::Result<File> {
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                c(name)?.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    }
    fn mkdir_new(parent: &File, name: &str) -> io::Result<File> {
        if unsafe { libc::mkdirat(parent.as_raw_fd(), c(name)?.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let file = open_dir(parent, name)?;
        let m = file.metadata()?;
        if m.mode() != (libc::S_IFDIR | 0o700)
            || m.uid() != unsafe { libc::geteuid() }
            || m.gid() != unsafe { libc::getegid() }
        {
            return Err(invalid("new directory identity"));
        }
        parent.sync_all()?;
        Ok(file)
    }
    struct Output {
        tmp: File,
        parent: File,
        directory: File,
        leaf: String,
        tmp_stamp: Value,
        parent_stamp: Value,
        dir_stamp: Value,
    }
    impl Output {
        fn create() -> io::Result<Self> {
            let tmp = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/tmp")?;
            let m = tmp.metadata()?;
            if m.uid() != 0 || m.gid() != 0 || m.mode() != (libc::S_IFDIR | 0o1777) {
                return Err(invalid("tmp root sticky directory"));
            }
            let parent = mkdir_new(&tmp, "luma-caption-read-proof")?;
            let leaf = format!("caption-{}", std::process::id());
            let directory = mkdir_new(&parent, &leaf)?;
            Ok(Self {
                tmp_stamp: tmp_identity(&tmp)?,
                parent_stamp: stamp(&parent)?,
                dir_stamp: stamp(&directory)?,
                tmp,
                parent,
                directory,
                leaf,
            })
        }
        fn check(&self) -> io::Result<()> {
            if tmp_identity(&self.tmp)? != self.tmp_stamp
                || stamp(&self.parent)? != self.parent_stamp
                || stamp(&self.directory)? != self.dir_stamp
                || stamp(&open_dir(&self.tmp, "luma-caption-read-proof")?)? != self.parent_stamp
                || stamp(&open_dir(&self.parent, &self.leaf)?)? != self.dir_stamp
            {
                return Err(invalid("output directory changed"));
            }
            Ok(())
        }
        fn describe(&self) -> Value {
            json!({"parent_path":PARENT,"path":format!("{PARENT}/{}",self.leaf),"parent_identity":self.parent_stamp,"directory_identity":self.dir_stamp,"parent_created_new":true,"directory_created_new":true})
        }
        fn save(&self, name: &str, value: &Value) -> io::Result<()> {
            self.check()?;
            let raw = serde_json::to_vec(value)?;
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    c(name)?.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut file = unsafe { File::from_raw_fd(fd) };
            let before = file.metadata()?;
            if before.mode() != (libc::S_IFREG | 0o600)
                || before.nlink() != 1
                || before.uid() != unsafe { libc::geteuid() }
                || before.gid() != unsafe { libc::getegid() }
            {
                return Err(invalid("output file identity"));
            }
            file.write_all(&raw)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            let after = file.metadata()?;
            let namedfd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    c(name)?.as_ptr(),
                    libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if namedfd < 0 {
                return Err(io::Error::last_os_error());
            }
            let named = unsafe { File::from_raw_fd(namedfd) };
            if stamp(&file)? != stamp(&named)?
                || after.dev() != before.dev()
                || after.ino() != before.ino()
                || after.len() != (raw.len() + 1) as u64
            {
                return Err(invalid("output publication identity"));
            }
            self.directory.sync_all()?;
            self.check()
        }
    }
    fn schema(raw: &str) -> Value {
        match serde_json::from_str::<Value>(raw) {
            Err(error) => json!({"kind":"invalid_json","detail":error.to_string()}),
            Ok(value) => {
                let success = value.get("returnValue").and_then(Value::as_bool);
                let caption = value
                    .get("settings")
                    .and_then(|v| v.get("captionEnable"))
                    .and_then(Value::as_str);
                let kind = if !value.is_object() {
                    "non_object"
                } else if success == Some(false) {
                    "service_false"
                } else if success == Some(true)
                    && value.get("category").and_then(Value::as_str) == Some("caption")
                    && value.get("subscribed").and_then(Value::as_bool) == Some(false)
                    && caption.is_some()
                {
                    "caption_string_reply"
                } else {
                    "unrecognized_reply"
                };
                json!({"kind":kind,"return_value":success,"caption_enable":caption})
            }
        }
    }
    pub fn run() -> io::Result<()> {
        // This exact source-derived app identity is checked before output, registration or logging.
        if plx_base::paths::app_id() != APP {
            return Err(invalid("fixed installed app identity"));
        }
        let before = own()?;
        let core_limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &core_limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
        unsafe {
            libc::umask(0o077);
        }
        let output = Output::create()?;
        let mut report = json!({"schema":1,"phase":"ready","app_id":APP,"source_sha256":env!("CAPTION_PROBE_SOURCE_SHA256"),
            "self_before":before,"runtime":output.describe(),"holds":{"before_call_ms":8000,"after_result_requested_ms":20000},
            "request":{"uri":URI,"payload":PAYLOAD,"call_budget_ms":3000},
            "application_threads_spawned":false,"application_child_processes_spawned":0,"argv_interpreted":false,
            "core_dump_limit_zero":true,"process_umask":63,
            "root_operations":false,"settings_writes":false,"preferences_operations":false,"sdk_internal_activity_qualified":false,
            "window_created_by_probe":false,"caller_application_id_attached":false,"unregistration_outcome_measured":false,
            "native_process_exit_measured":false,"caption_rendering_or_regional_semantics_qualified":false,
            "possible_framework_runtime_parent":"/tmp/com.ash.nativehome.captionreadproof"});
        output.save("ready.json", &report)?;
        let hold = Instant::now();
        std::thread::sleep(Duration::from_secs(8));
        report["reported_prehold_ms"] = json!(hold.elapsed().as_millis());
        let pre_call = match own() {
            Ok(value) => value,
            Err(error) => json!({"observation_error":error.to_string()}),
        };
        report["self_before_call"] = pre_call.clone();
        if pre_call != before {
            report["phase"] = json!("result");
            report["registration"] = json!({"registered":false,"setup":null});
            report["call"] = json!({"attempts":0,"outcome":"not_attempted","reason":"self_identity_changed","setup":null,
                "raw_reply":null,"raw_reply_bytes":null,"retained_reply_limit":REPLY_LIMIT,"retained_reply_truncated":false,"reply_schema":null});
            report["self_after"] = pre_call;
            report["self_unchanged"] = json!(false);
            output.save("result.json", &report)?;
            std::thread::sleep(Duration::from_secs(20));
            return Err(invalid("self changed before call; observation saved"));
        }
        let mut call = json!({"attempts":0,"outcome":"not_attempted","setup":null,"raw_reply":null,"raw_reply_bytes":null,
            "retained_reply_limit":REPLY_LIMIT,"retained_reply_truncated":false,"reply_schema":null});
        let begin = Instant::now();
        match plx_platform::webos::ls2::register() {
            Err(plx_platform::webos::ls2::RegisterFail::Setup {
                stage,
                detail,
                code,
            }) => {
                report["registration"] =
                    json!({"registered":false,"setup":{"stage":stage,"detail":detail,"code":code}});
            }
            Ok(bus) => {
                report["registration"] = json!({"registered":true,"setup":null});
                call["attempts"] = json!(1);
                let call_begin = Instant::now();
                match bus.call(URI, PAYLOAD, Duration::from_secs(3)) {
                    Ok(raw) => {
                        call["outcome"] = json!("reply");
                        call["raw_reply_bytes"] = json!(raw.len());
                        if raw.len() <= REPLY_LIMIT {
                            call["reply_schema"] = schema(&raw);
                            call["raw_reply"] = json!(raw);
                        } else {
                            call["retained_reply_truncated"] = json!(true);
                            call["reply_schema"] = json!({"kind":"oversized_reply_not_retained"});
                        }
                    }
                    Err(plx_platform::webos::ls2::Fail::Timeout) => {
                        call["outcome"] = json!("timeout")
                    }
                    Err(plx_platform::webos::ls2::Fail::Setup {
                        stage,
                        detail,
                        code,
                    }) => {
                        call["outcome"] = json!("setup_error");
                        call["setup"] = json!({"stage":stage,"detail":detail,"code":code});
                    }
                }
                report["reported_call_ms"] = json!(call_begin.elapsed().as_millis());
                drop(bus); // Outcome is not exposed by the existing client. May log in its own app runtime.
            }
        }
        report["reported_registration_call_drop_ms"] = json!(begin.elapsed().as_millis());
        report["call"] = call;
        report["phase"] = json!("result");
        report["self_after"] = match own() {
            Ok(value) => value,
            Err(error) => json!({"observation_error":error.to_string()}),
        };
        report["self_unchanged"] = json!(report["self_after"] == before);
        output.save("result.json", &report)?;
        std::thread::sleep(Duration::from_secs(20));
        output.check()?;
        Ok(())
    }
}
fn main() -> Result<(), String> {
    #[cfg(target_arch = "arm")]
    {
        native::run().map_err(|error| error.to_string())
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Err("Unsupported host: no native registration or request performed".into())
    }
}
