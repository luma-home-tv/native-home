//! Installed Proof helper: Identify plus Finish, or one sealed icon read then Finish.
#[cfg(any(target_arch = "arm", test))]
use crate::{broker_protocol as protocol, broker_transport::Credentials};
#[cfg(any(target_arch = "arm", test))]
use serde_json::Value;
use std::io;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

fn no_arguments(arguments: &[String]) -> io::Result<()> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(invalid("Installed startup proof accepts no arguments"))
    }
}

#[cfg(any(target_arch = "arm", test))]
fn checked_context(value: &Value, expected_sha256: &str) -> io::Result<()> {
    let reader = &value["_reader"];
    if reader["schema"].as_u64() != Some(1)
        || reader["kind"].as_str() != Some("fixed_core_luna_send")
        || reader["program"].as_str() != Some("/usr/bin/luna-send")
        || reader["approved"].as_bool() != Some(true)
        || reader["qualified"].as_bool() != Some(true)
        || reader["cleanup_known"].as_bool() != Some(true)
        || reader["executable"]["sha256"].as_str() != Some(expected_sha256)
        || reader["executable_after"]["sha256"].as_str() != Some(expected_sha256)
    {
        return Err(invalid("Unqualified fixed Core context reader"));
    }
    protocol::validate_context(
        &value["system_info"],
        &value["foreground"],
        &value["windows"],
    )
    .map_err(|_| invalid("Startup proof requires exact G2 and sole Proof CARD"))
}

#[cfg(any(target_arch = "arm", test))]
fn enrolled_peer(
    marker: &protocol::Marker,
    context_reader_sha256: &str,
    connect: &Credentials,
    message: &Credentials,
    peer: &protocol::PeerEvidence,
    now: u64,
) -> io::Result<protocol::Enrollment> {
    if connect != message || peer.start_ticks == 0 {
        return Err(invalid(
            "Kernel message credentials differ from accepted connection",
        ));
    }
    let enrollment = protocol::Enrollment {
        schema: 1,
        nonce: marker.nonce.clone(),
        client_pid: u32::try_from(connect.pid).map_err(|_| invalid("Invalid kernel client PID"))?,
        client_uid: connect.uid,
        client_start_ticks: peer.start_ticks,
        ui_sha256: marker.ui_sha256.clone(),
        broker_sha256: marker.broker_sha256.clone(),
        expires_unix: now
            .checked_add(25)
            .ok_or_else(|| invalid("Clock overflow"))?,
        context_reader_sha256: context_reader_sha256.into(),
    };
    enrollment
        .validate(now)
        .map_err(|_| invalid("Invalid kernel-derived enrollment"))?;
    protocol::authorize(&enrollment, connect, message, peer)
        .map_err(|_| invalid("Peer does not match installed Proof and kernel identity"))?;
    Ok(enrollment)
}

#[cfg(target_arch = "arm")]
mod native {
    use super::*;
    use crate::{
        artwork_transfer,
        broker_installation::{Installation, Profile},
        broker_transport::Listener,
    };
    use serde_json::json;
    use std::{
        io::Write,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    fn unix_now() -> io::Result<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .map_err(|_| invalid("Invalid wall clock"))
    }
    fn remaining(deadline: Instant) -> io::Result<()> {
        if Instant::now() >= deadline {
            Err(invalid("Startup proof operation deadline expired"))
        } else {
            Ok(())
        }
    }
    fn alarm(report: &mut Value) -> io::Result<()> {
        // Own-process fail-stop only; no external PID signaling. D-state can
        // defer delivery, so this is not a hard process-lifetime guarantee.
        if unsafe { libc::signal(libc::SIGALRM, libc::SIG_DFL) } == libc::SIG_ERR {
            return Err(io::Error::last_os_error());
        }
        let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
        let mut actual: libc::sigset_t = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigemptyset(&mut set) } != 0
            || unsafe { libc::sigaddset(&mut set, libc::SIGALRM) } != 0
            || unsafe { libc::sigprocmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut()) } != 0
            || unsafe { libc::sigprocmask(libc::SIG_SETMASK, std::ptr::null(), &mut actual) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::sigismember(&actual, libc::SIGALRM) } != 0 {
            return Err(invalid("Own alarm remains blocked"));
        }
        unsafe { libc::alarm(30) };
        report["own_alarm"] = json!({"requested_seconds":30,"default_disposition_requested":true,"sigalrm_unblocked_readback":true,"delivery_observed":false});
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0
            || unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn admit(
        installation: &Installation,
        enrollment: &protocol::Enrollment,
        connect: &Credentials,
        message: &Credentials,
        deadline: Instant,
    ) -> Result<protocol::PeerEvidence, Box<dyn std::error::Error>> {
        remaining(deadline)?;
        installation.revalidate()?;
        enrollment.validate(unix_now()?)?;
        let peer = crate::broker_server::proof_peer(enrollment.client_pid)?;
        installation.admit_peer(connect, message, &peer)?;
        protocol::authorize(enrollment, connect, message, &peer)?;
        remaining(deadline)?;
        Ok(peer)
    }
    pub(super) fn run(report: &mut Value) -> Result<(), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(20);
        alarm(report)?; // Before synchronous installation/proc/LS2 work.
        let installation = Installation::load(Profile::Proof)?;
        let pins = installation.pins();
        report["installation"] = installation.evidence();
        report["root_process"] = report["installation"]["root_process"].clone();
        report["nonce"] = json!(pins.rendezvous_nonce);
        report["broker_sha256"] = json!(pins.broker_sha256);
        installation.revalidate()?;
        remaining(deadline)?;
        let listener = Listener::bind(&pins.rendezvous_nonce)?;
        report["listener_bound"] = json!(true);
        let connection = listener.accept_until(deadline)?;
        let connect = connection.peer_credentials()?;
        report["connect_credentials"] =
            json!({"pid":connect.pid,"uid":connect.uid,"gid":connect.gid});
        let packet = connection.recv_until(protocol::REQUEST_LIMIT, deadline)?;
        let request = protocol::parse_request(&packet.bytes)?;
        if request.nonce != pins.rendezvous_nonce {
            return Err(invalid("Identify nonce mismatch").into());
        }
        report["identify_message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
        report["identify_request"] = json!(request);
        let peer = crate::broker_server::proof_peer(
            u32::try_from(connect.pid).map_err(|_| invalid("Invalid client PID"))?,
        )?;
        installation.admit_peer(&connect, &packet.credentials, &peer)?;
        let marker = protocol::Marker {
            schema: 1,
            nonce: pins.rendezvous_nonce.clone(),
            ui_sha256: pins.ui_sha256.clone(),
            broker_sha256: pins.broker_sha256.clone(),
        };
        let enrollment = enrolled_peer(
            &marker,
            &pins.context_reader_sha256,
            &connect,
            &packet.credentials,
            &peer,
            unix_now()?,
        )?;
        report["enrollment"] = json!(enrollment);
        report["peer_before"] = json!(peer);
        let context = crate::broker_context::collect(deadline, &pins.context_reader_sha256);
        report["context_before"] = context;
        checked_context(&report["context_before"], &pins.context_reader_sha256)?;
        report["peer_at_identify_send"] = json!(admit(
            &installation,
            &enrollment,
            &connect,
            &packet.credentials,
            deadline
        )?);
        let response = protocol::Response {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: enrollment.nonce.clone(),
            server_pid: std::process::id(),
            server_uid: 0,
            client_pid: enrollment.client_pid,
            client_uid: enrollment.client_uid,
            client_start_ticks: enrollment.client_start_ticks,
            ui_sha256: enrollment.ui_sha256.clone(),
            broker_sha256: enrollment.broker_sha256.clone(),
        };
        response.validate()?;
        connection.send_until(&serde_json::to_vec(&response)?, deadline)?;
        report["response"] = json!(response);
        report["response_sent"] = json!(true);
        let packet = connection.recv_until(4096, deadline)?;
        if artwork_transfer::parse_request(&packet.bytes).is_ok() {
            report["resource_mode"] = json!(true);
            report["context_cli_gets_only"] = json!(false);
            crate::broker_server::serve_installed_artwork(
                &connection,
                &enrollment,
                connect,
                packet,
                deadline,
                report,
                &installation,
            )?;
            report["finished"] = report["artwork_finished"].clone();
            report["finished_sent"] = json!(true);
            // The shared operation sends Finished only after its final context
            // and peer readback. The finite installed session now closes.
            drop(connection);
            drop(listener);
            report["connection_closed"] = json!(true);
            report["qualified"] = json!(true);
            return Ok(());
        }
        let finish = artwork_transfer::parse_finish_request(&packet.bytes)?;
        if finish.request_id != 1 || finish.nonce != pins.rendezvous_nonce {
            return Err(invalid("Only first Finish request is admitted after Identify").into());
        }
        report["finish_request"] = json!(finish);
        report["finish_message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
        report["peer_at_finish"] = json!(admit(
            &installation,
            &enrollment,
            &connect,
            &packet.credentials,
            deadline
        )?);
        let context = crate::broker_context::collect(deadline, &pins.context_reader_sha256);
        report["context_after"] = context;
        checked_context(&report["context_after"], &pins.context_reader_sha256)?;
        report["peer_after"] = json!(admit(
            &installation,
            &enrollment,
            &connect,
            &packet.credentials,
            deadline
        )?);
        let finished = artwork_transfer::Finished::new(&finish, &response)?;
        artwork_transfer::send_finished(&connection, &finished, deadline)?;
        report["finished"] = json!(finished);
        report["finished_sent"] = json!(true);
        // Closing this accepted connection and listener is the only endpoint
        // cleanup. No PID file, daemon, filesystem socket or Root write exists.
        drop(connection);
        drop(listener);
        report["connection_closed"] = json!(true);
        report["qualified"] = json!(true);
        Ok(())
    }
    pub(super) fn entry() -> Result<(), String> {
        let began = Instant::now();
        let mut report = json!({"schema":1,"stage":"startup_proof_complete","profile":"proof","server_pid":std::process::id(),"operation_budget_ms":20000,"listener_bound":false,"response_sent":false,"finished_sent":false,"connection_closed":false,"qualified":false,"icon_reads":false,"artwork_reads":false,"commands":false,"setting_writes":false,"root_file_writes":false,"daemonized":false,"normal_loader_qualified":false,"production_startup_qualified":false,"context_cli_gets_only":true});
        let result = run(&mut report);
        report["outcome"] = json!(if result.is_ok() {
            if report["artwork_session_completed"] == true {
                "identified_artwork_finished"
            } else {
                "identified_finished"
            }
        } else {
            "refused"
        });
        if let Err(ref error) = result {
            report["error"] = json!(error.to_string());
        }
        report["reported_elapsed_ms"] = json!(began.elapsed().as_millis());
        println!("{report}");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        result.map_err(|e| e.to_string())
    }
}

/// No arguments, host fallback, command selection or standalone service mode.
pub fn main_entry(arguments: &[String]) -> Result<(), String> {
    no_arguments(arguments).map_err(|error| error.to_string())?;
    #[cfg(target_arch = "arm")]
    {
        native::entry()
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Err("Installed startup proof requires the admitted ARM Proof package".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn context() -> Value {
        json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},"foreground":{"returnValue":true,"appId":crate::identity::PROOF},"windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD","extra":"ignored"}]},"_reader":{"schema":1,"kind":"fixed_core_luna_send","program":"/usr/bin/luna-send","approved":true,"qualified":true,"cleanup_known":true,"executable":{"sha256":"d".repeat(64)},"executable_after":{"sha256":"d".repeat(64)}}})
    }
    #[test]
    fn command_line_never_accepts_dynamic_fields() {
        assert!(no_arguments(&[]).is_ok());
        for arg in ["--session", "--daemon", "--profile", "x", ""] {
            assert!(no_arguments(&[arg.into()]).is_err());
        }
    }
    #[test]
    fn context_requires_original_policy_and_qualified_pinned_reader() {
        let good = context();
        assert!(checked_context(&good, &"d".repeat(64)).is_ok());
        for (key, value) in [
            ("approved", json!(false)),
            ("qualified", json!(false)),
            ("cleanup_known", json!(false)),
            ("program", json!("/tmp/luna-send")),
        ] {
            let mut bad = good.clone();
            bad["_reader"][key] = value;
            assert!(checked_context(&bad, &"d".repeat(64)).is_err());
        }
        let mut bad = good.clone();
        bad["foreground"]["appId"] = json!("com.webos.app.home");
        assert!(checked_context(&bad, &"d".repeat(64)).is_err());
        let mut bad = good;
        bad["_reader"]["executable_after"]["sha256"] = json!("e".repeat(64));
        assert!(checked_context(&bad, &"d".repeat(64)).is_err());
    }
    #[test]
    fn enrollment_comes_from_kernel_and_measured_peer_not_ui_json() {
        let connect = Credentials {
            pid: 123,
            uid: 6583,
            gid: 6583,
        };
        let peer = protocol::PeerEvidence {
            pid: 123,
            uid_fields: [6583; 4],
            start_ticks: 456,
            exe_sha256: "b".repeat(64),
            exe_path: protocol::PROOF_EXE_PATH.into(),
            root_path: protocol::PROOF_JAIL_ROOT.into(),
        };
        let marker = protocol::Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        let enrolled =
            enrolled_peer(&marker, &"d".repeat(64), &connect, &connect, &peer, 100).unwrap();
        assert_eq!(enrolled.client_pid, 123);
        assert_eq!(enrolled.client_start_ticks, 456);
        assert_eq!(enrolled.expires_unix, 125);
        let changed = Credentials { gid: 0, ..connect };
        assert!(enrolled_peer(&marker, &"d".repeat(64), &connect, &changed, &peer, 100).is_err());
        let wrong = protocol::PeerEvidence {
            start_ticks: 0,
            ..peer
        };
        assert!(enrolled_peer(&marker, &"d".repeat(64), &connect, &connect, &wrong, 100).is_err());
    }
}
