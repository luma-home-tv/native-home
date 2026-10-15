//! Temporary, isolated artwork/Identify server. Started by the test operator over SSH,
//! never by the app. Fixed context GETs use the pinned installed system client.
//! Artwork requests select an app and icon variant from our own fixed catalogue.
//! No caller-selected command, filesystem path or service URI exists.
#[cfg(any(target_arch = "arm", test))]
mod proof {
    use crate::broker_protocol as protocol;
    #[cfg(target_arch = "arm")]
    use crate::broker_protocol::{Enrollment, PeerEvidence};
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
    pub(super) fn peer(pid: u32) -> io::Result<PeerEvidence> {
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
    fn check_reader(
        value: &serde_json::Value,
        expected_sha256: &str,
    ) -> Result<(), protocol::Error> {
        let reader = &value["_reader"];
        if reader["kind"].as_str() != Some("fixed_core_luna_send")
            || reader["program"].as_str() != Some("/usr/bin/luna-send")
            || reader["approved"].as_bool() != Some(true)
            || reader["qualified"].as_bool() != Some(true)
            || reader["cleanup_known"].as_bool() != Some(true)
            || reader["executable"]["sha256"].as_str() != Some(expected_sha256)
        {
            return Err(protocol::Error::Context);
        }
        Ok(())
    }
    fn check_context(
        value: &serde_json::Value,
        expected_sha256: &str,
    ) -> Result<(), protocol::Error> {
        check_reader(value, expected_sha256)?;
        protocol::validate_context(
            &value["system_info"],
            &value["foreground"],
            &value["windows"],
        )
    }
    #[cfg(target_arch = "arm")]
    fn response_identity(enrolled: &Enrollment) -> protocol::Response {
        protocol::Response {
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
        }
    }
    #[cfg(target_arch = "arm")]
    fn resource_catalogue(
        enrolled: &Enrollment,
        deadline: std::time::Instant,
        summary: &mut serde_json::Value,
    ) -> Result<crate::app_artwork::SourceMap, Box<dyn std::error::Error>> {
        use serde_json::json;
        let value = crate::broker_context::catalogue(deadline, &enrolled.context_reader_sha256);
        // Preserve child ownership evidence even when admission fails. Never
        // log the full catalogue or release a caller-selected filesystem path.
        *summary = json!({"_reader":value["_reader"],"app_count":value["apps"]["apps"].as_array().map(Vec::len),
            "launch_point_count":value["launch_points"]["launchPoints"].as_array().map(Vec::len)});
        check_reader(&value, &enrolled.context_reader_sha256)?;
        let raw = json!({"replies":{"apps":value["apps"],"launch_points":value["launch_points"]}});
        let map = crate::app_artwork::sources_from_raw(&raw, crate::identity::PROOF)
            .map_err(|_| invalid("Server-owned artwork catalogue admission failed"))?;
        Ok(map)
    }
    #[cfg(target_arch = "arm")]
    struct PreparedArtwork {
        source: crate::app_artwork::Source,
        file: crate::app_artwork::EncodedFile,
    }
    #[cfg(target_arch = "arm")]
    #[derive(Clone, Copy, Debug)]
    enum ResourceFailure {
        CatalogueUnqualified,
        NoSource,
        Denied,
        Missing,
        Invalid,
        TooLarge,
        Changed,
    }
    #[cfg(target_arch = "arm")]
    fn resource_failure(error: &crate::app_artwork::ReadError) -> ResourceFailure {
        use crate::app_artwork::ReadReason;
        match error.reason {
            ReadReason::TooLarge => ResourceFailure::TooLarge,
            ReadReason::Changed => ResourceFailure::Changed,
            ReadReason::Io if error.errno == Some(libc::ENOENT) => ResourceFailure::Missing,
            ReadReason::Io
                if error.errno == Some(libc::EACCES) || error.errno == Some(libc::EPERM) =>
            {
                ResourceFailure::Denied
            }
            _ => ResourceFailure::Invalid,
        }
    }
    #[cfg(target_arch = "arm")]
    fn prepare_artwork(
        enrolled: &Enrollment,
        app_id: &str,
        icon_only: bool,
        deadline: std::time::Instant,
        record: &mut serde_json::Value,
    ) -> Result<PreparedArtwork, ResourceFailure> {
        use serde_json::json;
        let map = resource_catalogue(enrolled, deadline, &mut record["catalogue_before"])
            .map_err(|_| ResourceFailure::CatalogueUnqualified)?;
        let entry = map.get(app_id).ok_or(ResourceFailure::NoSource)?;
        let source = (if icon_only {
            entry.icon.as_ref()
        } else {
            entry.preferred()
        })
        .cloned()
        .ok_or(ResourceFailure::NoSource)?;
        record["source_key"] = json!(source.key());
        let file = crate::app_artwork::read_root_encoded(&source).map_err(|error| {
            record["read_error"] = json!(error);
            resource_failure(&error)
        })?;
        record["read_identity"] = json!(file.identity);
        record["encoded_sha256"] = json!(file
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>());
        record["encoded_bytes"] = json!(file.bytes.len());
        if !image_signature(&file.bytes) {
            return Err(ResourceFailure::Invalid);
        }
        Ok(PreparedArtwork { source, file })
    }
    #[cfg(target_arch = "arm")]
    fn verify_artwork_after(
        enrolled: &Enrollment,
        prepared: &PreparedArtwork,
        icon_only: bool,
        deadline: std::time::Instant,
        record: &mut serde_json::Value,
    ) -> Result<(), ResourceFailure> {
        use serde_json::json;
        let map = resource_catalogue(enrolled, deadline, &mut record["catalogue_after"])
            .map_err(|_| ResourceFailure::CatalogueUnqualified)?;
        let entry = map
            .get(prepared.source.app_id())
            .ok_or(ResourceFailure::Changed)?;
        let source = (if icon_only {
            entry.icon.as_ref()
        } else {
            entry.preferred()
        })
        .ok_or(ResourceFailure::Changed)?;
        record["source_key_after"] = json!(source.key());
        if source.key() != prepared.source.key() {
            return Err(ResourceFailure::Changed);
        }
        // The second bounded no-follow read covers same-path replacement or
        // content change while bytes were transferred, even without a new app version.
        let after = crate::app_artwork::read_root_encoded(source).map_err(|error| {
            record["readback_error"] = json!(error);
            resource_failure(&error)
        })?;
        record["readback_identity"] = json!(after.identity);
        record["readback_sha256"] = json!(after
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>());
        if after.identity != prepared.file.identity || after.digest != prepared.file.digest {
            return Err(ResourceFailure::Changed);
        }
        Ok(())
    }
    fn image_signature(bytes: &[u8]) -> bool {
        (bytes.len() >= 33 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") && &bytes[12..16] == b"IHDR")
            || (bytes.len() >= 4 && bytes.starts_with(&[0xff, 0xd8, 0xff]))
    }
    #[cfg(target_arch = "arm")]
    fn resource_peer(
        enrolled: &Enrollment,
        connect: &crate::broker_transport::Credentials,
        message: &crate::broker_transport::Credentials,
    ) -> Result<PeerEvidence, Box<dyn std::error::Error>> {
        enrolled.validate(now()?)?;
        if enrollment(&enrolled.nonce)? != *enrolled {
            return Err(invalid("Artwork enrollment changed or was revoked").into());
        }
        let evidence = peer(enrolled.client_pid)?;
        protocol::authorize(enrolled, connect, message, &evidence)?;
        if own_hash(&enrolled.nonce)? != enrolled.broker_sha256 {
            return Err(invalid("Helper identity changed during artwork operation").into());
        }
        Ok(evidence)
    }
    #[cfg(target_arch = "arm")]
    fn refusal_reason(reason: ResourceFailure) -> crate::artwork_transfer::RefusalReason {
        use crate::artwork_transfer::RefusalReason as R;
        match reason {
            ResourceFailure::CatalogueUnqualified => R::Unavailable,
            ResourceFailure::NoSource => R::NoSource,
            ResourceFailure::Denied => R::Denied,
            ResourceFailure::Missing => R::Missing,
            ResourceFailure::Invalid => R::Invalid,
            ResourceFailure::TooLarge => R::TooLarge,
            ResourceFailure::Changed => R::Changed,
        }
    }
    #[cfg(target_arch = "arm")]
    fn serve_artwork(
        connection: &crate::broker_transport::Connection,
        enrolled: &Enrollment,
        connect: crate::broker_transport::Credentials,
        mut packet: crate::broker_transport::Packet,
        deadline: std::time::Instant,
        report: &mut serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::artwork_transfer as wire;
        use serde_json::json;
        let identity = response_identity(enrolled);
        identity.validate()?;
        report["handshake_only"] = json!(false);
        report["artwork_reads_only"] = json!(true);
        report["icon_reads"] = json!(true);
        report["artwork_reads"] = json!([]);
        let mut expected_id = 1_u32;
        loop {
            if let Ok(finish) = wire::parse_finish_request(&packet.bytes) {
                if finish.nonce != enrolled.nonce || finish.request_id != expected_id {
                    return Err(invalid("Artwork finish sequence/session mismatch").into());
                }
                report["artwork_finish_connect_credentials"] =
                    json!({"pid":connect.pid,"uid":connect.uid,"gid":connect.gid});
                report["artwork_finish_message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
                let before = resource_peer(enrolled, &connect, &packet.credentials)?;
                report["artwork_finish_peer_before"] = json!(before);
                let context =
                    crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
                report["artwork_finish_context"] = context.clone();
                check_context(&context, &enrolled.context_reader_sha256)?;
                let after = resource_peer(enrolled, &connect, &packet.credentials)?;
                if after != before {
                    return Err(invalid("Artwork caller changed before finish").into());
                }
                report["artwork_finish_peer_after"] = json!(after);
                let finished = wire::Finished::new(&finish, &identity)?;
                wire::send_finished(connection, &finished, deadline)?;
                report["artwork_finished"] = json!(finished);
                report["artwork_session_completed"] = json!(true);
                return Ok(());
            }
            let request = wire::parse_request(&packet.bytes)?;
            if request.nonce != enrolled.nonce
                || request.request_id != expected_id
                || expected_id > 5
            {
                return Err(invalid("Artwork request limit/sequence/session mismatch").into());
            }
            let mut record = json!({"request":request,"request_bytes":packet.bytes,"encoded_data_released":false,"completed":false});
            record["connect_credentials"] =
                json!({"pid":connect.pid,"uid":connect.uid,"gid":connect.gid});
            record["message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
            let operation = (|| -> Result<(), Box<dyn std::error::Error>> {
                let before = resource_peer(enrolled, &connect, &packet.credentials)?;
                record["peer_before"] = json!(before);
                let context =
                    crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
                record["context_before"] = context.clone();
                check_context(&context, &enrolled.context_reader_sha256)?;
                let icon_only = matches!(request.variant, wire::Variant::Icon);
                let prepared =
                    prepare_artwork(enrolled, &request.app_id, icon_only, deadline, &mut record);
                if matches!(prepared, Err(ResourceFailure::CatalogueUnqualified)) {
                    return Err(invalid(
                        "Catalogue reader unqualified; no further operation permitted",
                    )
                    .into());
                }
                let mut header = None;
                if let Ok(value) = &prepared {
                    let value_header = wire::Header::from_file(
                        &request,
                        &identity,
                        value.source.key(),
                        &value.file,
                    )?;
                    wire::send_header_chunks(
                        connection,
                        &value_header,
                        &value.file.bytes,
                        deadline,
                    )?;
                    record["encoded_data_released"] = json!(true);
                    record["header"] = json!(value_header);
                    header = Some(value_header);
                }
                let readback = match &prepared {
                    Ok(value) => {
                        verify_artwork_after(enrolled, value, icon_only, deadline, &mut record)
                    }
                    Err(reason) => Err(*reason),
                };
                if matches!(readback, Err(ResourceFailure::CatalogueUnqualified)) {
                    return Err(invalid(
                        "Catalogue readback unqualified; no further operation permitted",
                    )
                    .into());
                }
                let after_context =
                    crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
                record["context_after"] = after_context.clone();
                check_context(&after_context, &enrolled.context_reader_sha256)?;
                let after = resource_peer(enrolled, &connect, &packet.credentials)?;
                record["peer_after"] = json!(after);
                if after != before {
                    return Err(invalid("Artwork caller changed during operation").into());
                }
                match readback {
                    Ok(()) => {
                        let complete = wire::Complete::from_header(
                            header
                                .as_ref()
                                .ok_or_else(|| invalid("Missing transferred artwork header"))?,
                        )?;
                        wire::send_complete(connection, &complete, deadline)?;
                        record["complete"] = json!(complete);
                        record["completed"] = json!(true);
                    }
                    Err(reason) => {
                        let refusal =
                            wire::Refusal::new(&request, &identity, refusal_reason(reason))?;
                        wire::send_refusal(connection, &refusal, deadline)?;
                        record["refusal"] = json!(refusal);
                    }
                }
                Ok(())
            })();
            if let Err(ref error) = operation {
                record["error"] = json!(error.to_string());
            }
            report["artwork_reads"]
                .as_array_mut()
                .ok_or_else(|| invalid("Missing artwork receipt container"))?
                .push(record);
            operation?;
            expected_id += 1;
            packet = connection.recv_until(protocol::REQUEST_LIMIT, deadline)?;
        }
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
        // The helper schedules termination of its OWN process. The operator
        // never signals a root PID after a racy procfs identity check. A kernel
        // uninterruptible wait can still defer signal delivery; unknown exit
        // therefore requires retained recovery evidence, never a guessed kill.
        if unsafe { libc::signal(libc::SIGALRM, libc::SIG_DFL) } == libc::SIG_ERR {
            return Err(io::Error::last_os_error().into());
        }
        let mut alarm_set: libc::sigset_t = unsafe { std::mem::zeroed() };
        let mut actual_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigemptyset(&mut alarm_set) } != 0
            || unsafe { libc::sigaddset(&mut alarm_set, libc::SIGALRM) } != 0
            || unsafe { libc::sigprocmask(libc::SIG_UNBLOCK, &alarm_set, std::ptr::null_mut()) }
                != 0
            || unsafe { libc::sigprocmask(libc::SIG_SETMASK, std::ptr::null(), &mut actual_mask) }
                != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        if unsafe { libc::sigismember(&actual_mask, libc::SIGALRM) } != 0 {
            return Err(invalid("Helper alarm signal mask was not cleared").into());
        }
        unsafe { libc::alarm(30) };
        report["own_alarm"] = json!({"requested_seconds":30,"sigalrm_unblocked_readback":true,"default_disposition_requested":true});
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
        let first_context =
            crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
        report["context_at_start"] = first_context.clone();
        check_context(&first_context, &enrolled.context_reader_sha256)?;
        enrolled.validate(now()?)?;
        let listener = Listener::bind(session)?;
        println!(
            "{}",
            json!({"schema":1,"stage":"ready","server_pid":std::process::id(),"server_uid":0,"nonce":session,"broker_sha256":enrolled.broker_sha256})
        );
        // One accepted connection: either one Identify or up to five icon reads
        // and Finish. Any failure terminates; no retry or broad fallback.
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
        if crate::artwork_transfer::parse_request(&packet.bytes).is_ok()
            || crate::artwork_transfer::parse_finish_request(&packet.bytes).is_ok()
        {
            return serve_artwork(&connection, &enrolled, connect, packet, deadline, report);
        }
        let request = protocol::parse_request(&packet.bytes)?;
        enrolled.validate(now()?)?;
        if request.nonce != enrolled.nonce {
            return Err(invalid("Request nonce mismatch").into());
        }
        let before = peer(enrolled.client_pid)?;
        protocol::authorize(&enrolled, &connect, &packet.credentials, &before)?;
        let before_context =
            crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
        report["context_before"] = before_context.clone();
        check_context(&before_context, &enrolled.context_reader_sha256)?;
        let after = peer(enrolled.client_pid)?;
        protocol::authorize(&enrolled, &connect, &packet.credentials, &after)?;
        if before != after
            || enrollment(session)? != enrolled
            || own_hash(session)? != enrolled.broker_sha256
        {
            return Err(invalid("Identity or enrollment changed during Identify").into());
        }
        let after_context =
            crate::broker_context::collect(deadline, &enrolled.context_reader_sha256);
        report["context_after"] = after_context.clone();
        check_context(&after_context, &enrolled.context_reader_sha256)?;
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
        fn resource_bytes_need_an_image_signature_before_release() {
            assert!(!image_signature(b"{\"account\":\"not an image\"}"));
            assert!(!image_signature(b"/etc/anything"));
            assert!(!image_signature(b"\x89PNG\r\n\x1a\n"));
            let mut png = vec![0_u8; 33];
            png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
            png[12..16].copy_from_slice(b"IHDR");
            assert!(image_signature(&png));
            assert!(image_signature(&[0xff, 0xd8, 0xff, 0x00]));
            // These are header admissions only, never validity or decoder proof.
        }
        #[test]
        fn context_requires_the_pinned_reader_and_the_original_proof_policy() {
            use serde_json::json;
            let expected = "d".repeat(64);
            let value = json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},
                "foreground":{"returnValue":true,"appId":crate::identity::PROOF},
                "windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]},
                "_reader":{"kind":"fixed_core_luna_send","program":"/usr/bin/luna-send","approved":true,"qualified":true,"cleanup_known":true,"executable":{"sha256":expected}}});
            assert!(check_context(&value, &expected).is_ok());
            for key in ["approved", "qualified", "cleanup_known"] {
                let mut bad = value.clone();
                bad["_reader"][key] = json!(false);
                assert!(check_context(&bad, &expected).is_err());
                bad["_reader"].as_object_mut().unwrap().remove(key);
                assert!(check_context(&bad, &expected).is_err());
            }
            for (key, replacement) in [
                ("kind", json!("anonymous_bus")),
                ("program", json!("/tmp/luna-send")),
                ("executable", json!({"sha256":"e".repeat(64)})),
            ] {
                let mut bad = value.clone();
                bad["_reader"][key] = replacement;
                assert!(check_context(&bad, &expected).is_err());
            }
            let mut home = value.clone();
            home["foreground"]["appId"] = json!("com.webos.app.home");
            assert!(check_context(&home, &expected).is_err());
            let mut covered = value.clone();
            covered["windows"]["windows"].as_array_mut().unwrap().push(
                json!({"appId":"com.webos.app.hdmi1","windowType":"_WEBOS_WINDOW_TYPE_CARD"}),
            );
            assert!(check_context(&covered, &expected).is_err());
        }
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

/// Kernel-bound fixed Proof process evidence reused by the installed bootstrap.
/// The caller supplies a PID obtained from socket credentials, never a wire PID.
#[cfg(target_arch = "arm")]
pub(crate) fn proof_peer(pid: u32) -> std::io::Result<crate::broker_protocol::PeerEvidence> {
    proof::peer(pid)
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
            if report["artwork_session_completed"] == true {
                "artwork_finished"
            } else {
                "identified"
            }
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
