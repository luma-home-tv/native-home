//! Temporary, Identify-only server. Started by the test operator over SSH,
//! never by the app. It installs no service, reads no icons and executes no commands.
#[cfg(any(target_arch = "arm", test))]
mod proof {
    #[cfg(target_arch = "arm")]
    use crate::broker_protocol::{self as protocol, Enrollment, PeerEvidence};
    #[cfg(target_arch = "arm")]
    use std::fs::File;
    use std::fs::{Metadata, OpenOptions};
    use std::io::{self, Read};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    #[cfg(target_arch = "arm")]
    use std::path::Path;

    const ELF_LIMIT: u64 = 8 * 1024 * 1024;
    fn invalid(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
    pub(super) fn nonce(value: &str) -> io::Result<()> {
        if value.len() != 32
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "Expected 32 lowercase hexadecimal session characters",
            ));
        }
        Ok(())
    }
    pub(super) fn session_path(value: &str) -> io::Result<String> {
        nonce(value)?;
        Ok(format!("/tmp/luma-home-artwork-proof.{value}"))
    }
    fn bounded(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.is_empty() || bytes.len() > limit {
            return Err(invalid("Empty or oversized proof input"));
        }
        Ok(bytes)
    }
    fn hash_reader(reader: &mut impl Read) -> io::Result<String> {
        let mut hash = plx_base::sha256::Sha256::new();
        let mut buffer = [0_u8; 32768];
        let mut total = 0_u64;
        loop {
            let count = reader.read(&mut buffer[..(ELF_LIMIT - total + 1).min(32768) as usize])?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > ELF_LIMIT {
                return Err(invalid("Proof executable exceeds 8 MiB"));
            }
            hash.update(&buffer[..count]);
        }
        if total == 0 {
            return Err(invalid("Empty proof executable"));
        }
        Ok(hash.finish().iter().map(|b| format!("{b:02x}")).collect())
    }
    fn same_file(a: &Metadata, b: &Metadata) -> bool {
        (
            a.dev(),
            a.ino(),
            a.len(),
            a.mode(),
            a.uid(),
            a.gid(),
            a.nlink(),
            a.mtime(),
            a.mtime_nsec(),
            a.ctime(),
            a.ctime_nsec(),
        ) == (
            b.dev(),
            b.ino(),
            b.len(),
            b.mode(),
            b.uid(),
            b.gid(),
            b.nlink(),
            b.mtime(),
            b.mtime_nsec(),
            b.ctime(),
            b.ctime_nsec(),
        )
    }
    fn private_file(meta: &Metadata, mode: u32) -> io::Result<()> {
        if !meta.is_file()
            || meta.uid() != 0
            || meta.gid() != 0
            || meta.mode() & 0o7777 != mode
            || meta.nlink() != 1
        {
            return Err(invalid("Expected a private root-owned regular proof file"));
        }
        Ok(())
    }
    fn checked_directory(meta: &Metadata) -> io::Result<()> {
        if !meta.is_dir() || meta.uid() != 0 || meta.gid() != 0 || meta.mode() & 0o7777 != 0o700 {
            return Err(invalid("Expected a private root-owned proof directory"));
        }
        Ok(())
    }
    pub(super) fn start_ticks(raw: &str, pid: u32) -> io::Result<u64> {
        let (prefix, fields) = raw
            .rsplit_once(") ")
            .ok_or_else(|| invalid("Malformed process stat"))?;
        if !prefix.starts_with(&format!("{pid} (")) {
            return Err(invalid("Process stat PID mismatch"));
        }
        fields
            .split_whitespace()
            .nth(19)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| invalid("Missing process start ticks"))
    }
    fn uid_fields(raw: &str) -> io::Result<[u32; 4]> {
        let mut values = raw.lines().filter_map(|line| line.strip_prefix("Uid:"));
        let fields = values
            .next()
            .ok_or_else(|| invalid("Missing process UIDs"))?;
        if values.next().is_some() {
            return Err(invalid("Duplicate process UIDs"));
        }
        let numbers: Vec<u32> = fields
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| invalid("Invalid process UIDs"))?;
        numbers
            .try_into()
            .map_err(|_| invalid("Expected four process UIDs"))
    }
    #[cfg(target_arch = "arm")]
    pub(super) fn now() -> io::Result<u64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|_| invalid("Invalid system clock"))
    }
    #[cfg(target_arch = "arm")]
    fn open_child(parent: &File, name: &str, flags: i32) -> io::Result<File> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let name =
            std::ffi::CString::new(name).map_err(|_| invalid("Invalid fixed proof file name"))?;
        let fd =
            unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    #[cfg(target_arch = "arm")]
    fn enrollment(session: &str) -> Result<Enrollment, Box<dyn std::error::Error>> {
        let path = session_path(session)?;
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        checked_directory(&dir.metadata()?)?;
        let mut file = open_child(
            &dir,
            "session.json",
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?;
        let before = file.metadata()?;
        private_file(&before, 0o600)?;
        if before.len() == 0 || before.len() > protocol::ENROLLMENT_LIMIT as u64 {
            return Err(invalid("Invalid enrollment size").into());
        }
        let raw = bounded(&mut file, protocol::ENROLLMENT_LIMIT)?;
        if !same_file(&before, &file.metadata()?) || raw.len() as u64 != before.len() {
            return Err(invalid("Enrollment changed during read").into());
        }
        // Check the current name still names that inode; O_NOFOLLOW also rejects
        // a substituted link. The private directory is never writable by the client.
        let current = open_child(
            &dir,
            "session.json",
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?;
        if !same_file(&before, &current.metadata()?)
            || !same_file(&dir.metadata()?, &std::fs::symlink_metadata(&path)?)
        {
            return Err(invalid("Enrollment or session directory was replaced").into());
        }
        let value = protocol::parse_enrollment(&raw, now()?)?;
        if value.nonce != session {
            return Err(invalid("Enrollment session mismatch").into());
        }
        Ok(value)
    }
    #[cfg(target_arch = "arm")]
    fn text(parent: &File, name: &str, limit: usize) -> io::Result<String> {
        let file = open_child(
            parent,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?;
        String::from_utf8(bounded(file, limit)?).map_err(|_| invalid("Non-UTF8 process evidence"))
    }
    #[cfg(target_arch = "arm")]
    fn link(parent: &File, name: &str) -> io::Result<String> {
        use std::os::fd::AsRawFd;
        let name = std::ffi::CString::new(name).map_err(|_| invalid("Invalid fixed proc link"))?;
        let mut bytes = [0_u8; 4097];
        let size = unsafe {
            libc::readlinkat(
                parent.as_raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        if size == 0 || size as usize >= bytes.len() {
            return Err(invalid("Invalid process link length"));
        }
        String::from_utf8(bytes[..size as usize].to_vec())
            .map_err(|_| invalid("Non-UTF8 process link"))
    }
    #[cfg(target_arch = "arm")]
    fn peer(pid: u32) -> io::Result<PeerEvidence> {
        if pid <= 1 {
            return Err(invalid("Invalid enrolled PID"));
        }
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(format!("/proc/{pid}"))?;
        let start = start_ticks(&text(&dir, "stat", 4096)?, pid)?;
        let uids = uid_fields(&text(&dir, "status", 65536)?)?;
        let exe_path = link(&dir, "exe")?;
        let root_path = link(&dir, "root")?;
        if exe_path != protocol::PROOF_EXE_PATH || root_path != protocol::PROOF_JAIL_ROOT {
            return Err(invalid("Enrolled process is outside the exact Proof jail"));
        }
        // Deliberately follow only this kernel procfs executable link. We never
        // open a path supplied by the client or use its JSON as process evidence.
        let mut exe = open_child(&dir, "exe", libc::O_RDONLY | libc::O_NONBLOCK)?;
        let before = exe.metadata()?;
        if !before.is_file() || before.len() == 0 || before.len() > ELF_LIMIT {
            return Err(invalid("Invalid peer executable"));
        }
        let exe_sha256 = hash_reader(&mut exe)?;
        if !same_file(&before, &exe.metadata()?)
            || start_ticks(&text(&dir, "stat", 4096)?, pid)? != start
            || uid_fields(&text(&dir, "status", 65536)?)? != uids
            || link(&dir, "exe")? != exe_path
            || link(&dir, "root")? != root_path
        {
            return Err(invalid("Enrolled process changed during observation"));
        }
        let current = open_child(&dir, "exe", libc::O_RDONLY | libc::O_NONBLOCK)?;
        if !same_file(&before, &current.metadata()?) {
            return Err(invalid("Peer executable replaced"));
        }
        Ok(PeerEvidence {
            pid,
            uid_fields: uids,
            start_ticks: start,
            exe_sha256,
            exe_path,
            root_path,
        })
    }
    #[cfg(target_arch = "arm")]
    fn own_hash(session: &str) -> io::Result<String> {
        let expected = format!("{}/artwork-broker-proof", session_path(session)?);
        if std::fs::read_link("/proc/self/exe")? != Path::new(&expected) {
            return Err(invalid("Unexpected helper executable path"));
        }
        let mut file = File::open("/proc/self/exe")?;
        let before = file.metadata()?;
        private_file(&before, 0o700)?;
        let hash = hash_reader(&mut file)?;
        if !same_file(&before, &file.metadata()?)
            || !same_file(&before, &std::fs::symlink_metadata(expected)?)
        {
            return Err(invalid("Helper executable changed"));
        }
        Ok(hash)
    }
    #[cfg(target_arch = "arm")]
    fn context(
        bus: &plx_platform::webos::ls2::Registration,
        deadline: std::time::Instant,
    ) -> serde_json::Value {
        use serde_json::json;
        use std::time::{Duration, Instant};
        let mut values = serde_json::Map::new();
        for (name, uri, payload) in [
            (
                "system_info",
                "luna://com.webos.service.tv.systemproperty/getSystemInfo",
                json!({"keys":["modelName","firmwareVersion","sdkVersion"]}),
            ),
            (
                "foreground",
                "luna://com.webos.applicationManager/getForegroundAppInfo",
                json!({}),
            ),
            (
                "windows",
                "luna://com.webos.surfacemanager/getForegroundWindowInfo",
                json!({}),
            ),
        ] {
            let left = deadline.saturating_duration_since(Instant::now());
            let value = if left.is_zero() {
                json!({"deadline":true})
            } else {
                match bus.call(
                    uri,
                    &payload.to_string(),
                    left.min(Duration::from_millis(500)),
                ) {
                    Ok(raw) if raw.len() <= 65536 => {
                        serde_json::from_str(&raw).unwrap_or_else(|_| json!({"parse_error":true}))
                    }
                    Ok(_) => json!({"oversize":true}),
                    Err(_) => json!({"unavailable":true}),
                }
            };
            values.insert(name.into(), value);
        }
        serde_json::Value::Object(values)
    }
    #[cfg(target_arch = "arm")]
    fn check_context(value: &serde_json::Value) -> Result<(), protocol::Error> {
        protocol::validate_context(
            &value["system_info"],
            &value["foreground"],
            &value["windows"],
        )
    }
    fn operator_barrier(input: libc::c_int, deadline: std::time::Instant) -> io::Result<()> {
        use std::time::Instant;
        // Root operator's SSH stdin, never the application socket. This fixed
        // one-shot token lets the driver bind our actual proc records before
        // releasing the handshake. It cannot select an operation or argument.
        let flags = unsafe { libc::fcntl(input, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(input, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let expected = b"BEGIN_IDENTIFY\n";
        let mut bytes = [0_u8; 16];
        let mut used = 0;
        loop {
            let left = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| io::Error::from(io::ErrorKind::TimedOut))?;
            let mut fd = libc::pollfd {
                fd: input,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut fd, 1, left.as_millis().clamp(1, 8000) as i32) };
            if ready < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            if ready == 0 {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let count =
                unsafe { libc::read(input, bytes[used..].as_mut_ptr().cast(), bytes.len() - used) };
            if count < 0 {
                let e = io::Error::last_os_error();
                if matches!(
                    e.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) {
                    continue;
                }
                return Err(e);
            }
            if count == 0 {
                return if &bytes[..used] == expected {
                    Ok(())
                } else {
                    Err(invalid("Operator startup token absent or malformed"))
                };
            }
            used += count as usize;
            if used > expected.len() || bytes[..used] != expected[..used] {
                return Err(invalid("Operator startup token absent or malformed"));
            }
        }
    }
    #[cfg(target_arch = "arm")]
    pub(super) fn run(
        session: &str,
        report: &mut serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::broker_transport::Listener;
        use serde_json::json;
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_secs(25);
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(
                invalid("Temporary helper requires the operator's root test connection").into(),
            );
        }
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0
            || unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let enrolled = enrollment(session)?;
        if own_hash(session)? != enrolled.broker_sha256 {
            return Err(invalid("Enrolled helper hash mismatch").into());
        }
        println!(
            "{}",
            json!({"schema":1,"stage":"started","server_pid":std::process::id(),"server_uid":0,"nonce":session,"broker_sha256":enrolled.broker_sha256})
        );
        std::io::Write::flush(&mut std::io::stdout())?;
        operator_barrier(0, Instant::now() + Duration::from_secs(8))?;
        report["enrollment"] = serde_json::to_value(&enrolled)?;
        let before = peer(enrolled.client_pid)?;
        report["peer_at_start"] = serde_json::to_value(&before)?;
        let bus = plx_platform::webos::ls2::register()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let first_context = context(&bus, deadline);
        report["context_at_start"] = first_context.clone();
        check_context(&first_context)?;
        enrolled.validate(now()?)?;
        let listener = Listener::bind(session)?;
        println!(
            "{}",
            json!({"schema":1,"stage":"ready","server_pid":std::process::id(),"server_uid":0,"nonce":session,"broker_sha256":enrolled.broker_sha256})
        );
        // Exactly one accepted connection, one packet and one fixed operation.
        // Any failure drops both FDs and terminates; no retry or broad fallback.
        let connection = listener.accept_until(deadline)?;
        enrolled.validate(now()?)?;
        let connect = connection.peer_credentials()?;
        if connect.pid <= 1
            || connect.pid as u32 != enrolled.client_pid
            || connect.uid != enrolled.client_uid
        {
            return Err(invalid("Unenrolled peer connection").into());
        }
        let packet = connection.recv_until(protocol::REQUEST_LIMIT, deadline)?;
        let request = protocol::parse_request(&packet.bytes)?;
        enrolled.validate(now()?)?;
        if request.nonce != enrolled.nonce {
            return Err(invalid("Request nonce mismatch").into());
        }
        let before = peer(enrolled.client_pid)?;
        protocol::authorize(&enrolled, &connect, &packet.credentials, &before)?;
        let before_context = context(&bus, deadline);
        report["context_before"] = before_context.clone();
        check_context(&before_context)?;
        let after = peer(enrolled.client_pid)?;
        protocol::authorize(&enrolled, &connect, &packet.credentials, &after)?;
        if before != after
            || enrollment(session)? != enrolled
            || own_hash(session)? != enrolled.broker_sha256
        {
            return Err(invalid("Identity or enrollment changed during Identify").into());
        }
        let after_context = context(&bus, deadline);
        report["context_after"] = after_context.clone();
        check_context(&after_context)?;
        let final_peer = peer(enrolled.client_pid)?;
        protocol::authorize(&enrolled, &connect, &packet.credentials, &final_peer)?;
        if after != final_peer {
            return Err(invalid("Process changed before response").into());
        }
        enrolled.validate(now()?)?;
        let response = protocol::Response {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: enrolled.nonce.clone(),
            server_pid: std::process::id(),
            server_uid: 0,
            client_pid: enrolled.client_pid,
            client_uid: enrolled.client_uid,
            client_start_ticks: enrolled.client_start_ticks,
            ui_sha256: enrolled.ui_sha256.clone(),
            broker_sha256: enrolled.broker_sha256.clone(),
        };
        response.validate()?;
        let raw = serde_json::to_vec(&response)?;
        if raw.len() > protocol::RESPONSE_LIMIT {
            return Err(invalid("Response exceeds protocol bound").into());
        }
        connection.send_until(&raw, deadline)?;
        report["peer_before"] = serde_json::to_value(before)?;
        report["peer_after"] = serde_json::to_value(final_peer)?;
        report["connect_credentials"] =
            json!({"pid":connect.pid,"uid":connect.uid,"gid":connect.gid});
        report["message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
        report["response"] = serde_json::to_value(response)?;
        report["response_sent"] = json!(true);
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn operator_barrier_accepts_only_fixed_complete_token_and_eof() {
            use std::io::Write;
            use std::os::fd::AsRawFd;
            use std::os::unix::net::UnixStream;
            use std::time::{Duration, Instant};
            for token in [
                b"BEGIN_IDENTIFY\n".as_slice(),
                b"",
                b"BEGIN_IDENTIFY",
                b"BEGIN_IDENTIFY\nX",
                b"RUN anything\n",
            ] {
                let (reader, mut writer) = UnixStream::pair().unwrap();
                writer.write_all(token).unwrap();
                drop(writer);
                let result =
                    operator_barrier(reader.as_raw_fd(), Instant::now() + Duration::from_secs(1));
                assert_eq!(result.is_ok(), token == b"BEGIN_IDENTIFY\n");
            }
            let (reader, mut writer) = UnixStream::pair().unwrap();
            writer.write_all(b"BEGIN_IDENTIFY\n").unwrap();
            assert_eq!(
                operator_barrier(
                    reader.as_raw_fd(),
                    Instant::now() + Duration::from_millis(10)
                )
                .unwrap_err()
                .kind(),
                io::ErrorKind::TimedOut
            );
            assert!(operator_barrier(-1, Instant::now() + Duration::from_secs(1)).is_err());
        }
        #[test]
        fn session_cannot_choose_a_path_or_command() {
            let n = "0123456789abcdef0123456789abcdef";
            assert_eq!(
                session_path(n).unwrap(),
                format!("/tmp/luma-home-artwork-proof.{n}")
            );
            for bad in [
                "",
                "../root",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "a; reboot",
            ] {
                assert!(session_path(bad).is_err());
            }
        }
        #[test]
        fn bounded_input_and_streaming_hash_enforce_limits() {
            assert!(bounded(&b""[..], 4).is_err());
            assert_eq!(bounded(&b"abcd"[..], 4).unwrap(), b"abcd");
            assert!(bounded(&b"abcde"[..], 4).is_err());
            let digest = plx_base::sha256::sha256(b"abc")
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(hash_reader(&mut &b"abc"[..]).unwrap(), digest);
            assert!(hash_reader(&mut &b""[..]).is_err());
            assert!(hash_reader(&mut io::repeat(0).take(ELF_LIMIT + 1)).is_err());
        }
        #[test]
        fn proc_parser_uses_actual_uid_tuple_and_stat_field22() {
            let mut fields = vec!["0"; 20];
            fields[0] = "S";
            fields[19] = "1234";
            assert_eq!(
                start_ticks(&format!("101 (odd ) name) {}", fields.join(" ")), 101).unwrap(),
                1234
            );
            assert!(start_ticks("101 (x) S", 101).is_err());
            assert!(start_ticks(&format!("102 (x) {}", fields.join(" ")), 101).is_err());
            assert_eq!(
                uid_fields("Name:\ttest\nUid:\t6001 6001 6001 6001\n").unwrap(),
                [6001; 4]
            );
            for bad in [
                "",
                "Uid: 1 2 3",
                "Uid: 1 2 3 4 5",
                "Uid: 1 2 x 4",
                "Uid: 1 2 3 4\nUid: 1 2 3 4",
            ] {
                assert!(uid_fields(bad).is_err());
            }
        }
        #[test]
        fn ownership_modes_and_identity_are_checked_on_real_metadata() {
            let root =
                std::env::temp_dir().join(format!("luma-server-test-{}", std::process::id()));
            std::fs::create_dir(&root).unwrap();
            let _cleanup = Cleanup(root.clone());
            let path = root.join("session");
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .unwrap();
            let a = file.metadata().unwrap();
            assert!(same_file(&a, &a));
            assert_eq!(
                private_file(&a, 0o600).is_ok(),
                a.uid() == 0 && a.gid() == 0
            );
            assert!(private_file(&a, 0o700).is_err());
            assert!(checked_directory(&a).is_err());
            std::fs::hard_link(&path, root.join("linked")).unwrap();
            assert!(private_file(&file.metadata().unwrap(), 0o600).is_err());
            std::fs::write(&path, b"changed").unwrap();
            assert!(!same_file(&a, &file.metadata().unwrap()));
        }
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}

/// Host builds refuse this proof. There is no stream or unprivileged fallback.
pub fn main_entry(arguments: &[String]) -> Result<(), String> {
    #[cfg(target_arch = "arm")]
    {
        if arguments.len() != 2 || arguments[0] != "--session" {
            return Err("Usage: artwork-broker-proof --session <32 lowercase hex>".into());
        }
        proof::nonce(&arguments[1]).map_err(|e| e.to_string())?;
        let mut report = serde_json::json!({"schema":1,"handshake_only":true,"icon_reads":false,"commands":false,"setting_writes":false,"response_sent":false,"server_pid":std::process::id()});
        let result = proof::run(&arguments[1], &mut report);
        report["outcome"] = serde_json::json!(if result.is_ok() {
            "identified"
        } else {
            "unqualified"
        });
        if let Err(ref error) = result {
            report["error"] = serde_json::json!(error.to_string());
        }
        println!("{report}");
        result.map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "arm"))]
    {
        let _ = arguments;
        Err("Identify helper requires the admitted ARM TV test lane".into())
    }
}
