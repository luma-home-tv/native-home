//! Fixed installed helper entries: Identify plus Finish, or one sealed icon read then Finish.
use crate::broker_installation::Profile;
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

#[cfg(test)]
fn checked_context(value: &Value, expected_sha256: &str) -> io::Result<()> {
    checked_context_for_profile(value, expected_sha256, Profile::Proof)
}

#[cfg(any(target_arch = "arm", test))]
fn checked_context_for_profile(
    value: &Value,
    expected_sha256: &str,
    profile: Profile,
) -> io::Result<()> {
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
    protocol::validate_context_for_profile(
        &value["system_info"],
        &value["foreground"],
        &value["windows"],
        profile,
    )
    .map_err(|_| {
        invalid(match profile {
            Profile::Proof => "Startup proof requires exact G2 and sole Proof CARD",
            Profile::Preview => "Installed helper requires exact G2 and sole Preview CARD",
        })
    })
}

#[cfg(test)]
fn enrolled_peer(
    marker: &protocol::Marker,
    context_reader_sha256: &str,
    connect: &Credentials,
    message: &Credentials,
    peer: &protocol::PeerEvidence,
    now: u64,
) -> io::Result<protocol::Enrollment> {
    enrolled_peer_for_profile(
        marker,
        context_reader_sha256,
        connect,
        message,
        peer,
        now,
        Profile::Proof,
    )
}

#[cfg(any(target_arch = "arm", test))]
fn enrolled_peer_for_profile(
    marker: &protocol::Marker,
    context_reader_sha256: &str,
    connect: &Credentials,
    message: &Credentials,
    peer: &protocol::PeerEvidence,
    now: u64,
    profile: Profile,
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
        .validate_for_profile(now, profile)
        .map_err(|_| invalid("Invalid kernel-derived enrollment"))?;
    protocol::authorize_for_profile(&enrollment, connect, message, peer, profile).map_err(
        |_| {
            invalid(match profile {
                Profile::Proof => "Peer does not match installed Proof and kernel identity",
                Profile::Preview => "Peer does not match installed Preview and kernel identity",
            })
        },
    )?;
    Ok(enrollment)
}

// Both values are moved in; the callback runs only after their owned descriptors
// have been dropped. This is source-bound FD closure, not process/namespace proof.
#[cfg(any(target_arch = "arm", test))]
fn after_dropping_endpoints<C, L, R>(connection: C, listener: L, after: impl FnOnce() -> R) -> R {
    drop(connection);
    drop(listener);
    after()
}

#[cfg(any(target_arch = "arm", test))]
fn record_closed_withdrawal(
    report: &mut Value,
    withdrawal: &crate::broker_server::ContextWithdrawal,
    peer_after: &protocol::PeerEvidence,
    installation_after: Value,
) -> io::Result<()> {
    use serde_json::json;
    if report["qualified"].as_bool() != Some(false)
        || report["finished_sent"].as_bool() != Some(false)
        || !report["finished"].is_null()
        || !report["artwork_finished"].is_null()
        || report["artwork_session_completed"].as_bool() != Some(false)
        || report["closed_negative_candidate"].as_bool() != Some(false)
        || report["connection_closed"].as_bool() != Some(true)
        || report["listener_closed"].as_bool() != Some(true)
        || report["response_sent"].as_bool() != Some(true)
        || report["resource_mode"].as_bool() != Some(true)
        || report["profile"] != json!(withdrawal.profile())
        || peer_after != withdrawal.peer()
    {
        return Err(invalid(
            "Contradictory or incomplete context-withdrawal closure",
        ));
    }
    report["failure"] = json!({"kind":"context_changed","stage":withdrawal.stage().as_str(),"request_id":withdrawal.request_id()});
    report["closed_negative_candidate"] = json!(true);
    report["context_withdrawal_message_credentials"] = json!({"pid":withdrawal.message().pid,"uid":withdrawal.message().uid,"gid":withdrawal.message().gid});
    report["peer_after_context_withdrawal"] = json!(peer_after);
    report["installation_after_context_withdrawal"] = installation_after;
    report["closed_negative_acceptance_enabled"] = json!(false);
    // Explicitly negative; these were checked false/null, never erased success.
    report["finished"] = Value::Null;
    report["finished_sent"] = json!(false);
    report["artwork_session_completed"] = json!(false);
    report["qualified"] = json!(false);
    Ok(())
}

#[cfg(target_arch = "arm")]
mod native {
    use super::*;
    use crate::{artwork_transfer, broker_installation::Installation, broker_transport::Listener};
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
        profile: Profile,
    ) -> Result<protocol::PeerEvidence, Box<dyn std::error::Error>> {
        remaining(deadline)?;
        installation.revalidate()?;
        enrollment.validate_for_profile(unix_now()?, profile)?;
        let peer = crate::broker_server::installed_peer(enrollment.client_pid, profile)?;
        installation.admit_peer(connect, message, &peer)?;
        protocol::authorize_for_profile(enrollment, connect, message, &peer, profile)?;
        remaining(deadline)?;
        Ok(peer)
    }
    pub(super) fn run(
        report: &mut Value,
        profile: Profile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(20);
        alarm(report)?; // Before synchronous installation/proc/LS2 work.
        let installation = Installation::load(profile)?;
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
        let peer = crate::broker_server::installed_peer(
            u32::try_from(connect.pid).map_err(|_| invalid("Invalid client PID"))?,
            profile,
        )?;
        installation.admit_peer(&connect, &packet.credentials, &peer)?;
        let marker = protocol::Marker {
            schema: 1,
            nonce: pins.rendezvous_nonce.clone(),
            ui_sha256: pins.ui_sha256.clone(),
            broker_sha256: pins.broker_sha256.clone(),
        };
        let enrollment = enrolled_peer_for_profile(
            &marker,
            &pins.context_reader_sha256,
            &connect,
            &packet.credentials,
            &peer,
            unix_now()?,
            profile,
        )?;
        report["enrollment"] = json!(enrollment);
        report["peer_before"] = json!(peer);
        let context = crate::broker_context::collect(deadline, &pins.context_reader_sha256);
        report["context_before"] = context;
        checked_context_for_profile(
            &report["context_before"],
            &pins.context_reader_sha256,
            profile,
        )?;
        report["peer_at_identify_send"] = json!(admit(
            &installation,
            &enrollment,
            &connect,
            &packet.credentials,
            deadline,
            profile
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
        response.validate_for_profile(profile)?;
        connection.send_until(&serde_json::to_vec(&response)?, deadline)?;
        report["response"] = json!(response);
        report["response_sent"] = json!(true);
        let packet = connection.recv_until(4096, deadline)?;
        if artwork_transfer::parse_request(&packet.bytes).is_ok() {
            report["resource_mode"] = json!(true);
            report["context_cli_gets_only"] = json!(false);
            let served = crate::broker_server::serve_installed_artwork(
                &connection,
                &enrollment,
                connect,
                packet,
                deadline,
                report,
                &installation,
            );
            if let Err(error) = served {
                let withdrawal = error
                    .downcast_ref::<crate::broker_server::ContextWithdrawal>()
                    .cloned();
                let fresh =
                    withdrawal.as_ref().map(|withdrawal| {
                        (|| -> Result<(protocol::PeerEvidence, Value), Box<dyn std::error::Error>> {
                        remaining(deadline)?;
                        if withdrawal.profile() != profile {
                            return Err(invalid("Context withdrawal profile changed").into());
                        }
                        let peer = admit(&installation, &enrollment, &connect,
                            &withdrawal.message(), deadline, profile)?;
                        if &peer != withdrawal.peer() {
                            return Err(invalid("Context withdrawal caller changed").into());
                        }
                        installation.revalidate()?;
                        remaining(deadline)?;
                        Ok((peer, installation.evidence()))
                    })()
                    });
                return after_dropping_endpoints(connection, listener, || {
                    report["connection_closed"] = json!(true);
                    report["listener_closed"] = json!(true);
                    match (withdrawal, fresh) {
                        (Some(withdrawal), Some(Ok((peer, evidence)))) => {
                            record_closed_withdrawal(report, &withdrawal, &peer, evidence)?;
                            Err(error)
                        }
                        (Some(_), Some(Err(revalidation))) => Err(revalidation),
                        _ => Err(error),
                    }
                });
            }
            report["finished"] = report["artwork_finished"].clone();
            report["finished_sent"] = json!(true);
            // The shared operation sends Finished only after its final context
            // and peer readback. The finite installed session now closes.
            return after_dropping_endpoints(connection, listener, || {
                report["connection_closed"] = json!(true);
                report["listener_closed"] = json!(true);
                report["qualified"] = json!(true);
                Ok(())
            });
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
            deadline,
            profile
        )?);
        let context = crate::broker_context::collect(deadline, &pins.context_reader_sha256);
        report["context_after"] = context;
        checked_context_for_profile(
            &report["context_after"],
            &pins.context_reader_sha256,
            profile,
        )?;
        report["peer_after"] = json!(admit(
            &installation,
            &enrollment,
            &connect,
            &packet.credentials,
            deadline,
            profile
        )?);
        let finished = artwork_transfer::Finished::new_for_profile(&finish, &response, profile)?;
        artwork_transfer::send_finished_for_profile(&connection, &finished, deadline, profile)?;
        report["finished"] = json!(finished);
        report["finished_sent"] = json!(true);
        // Closing this accepted connection and listener is the only endpoint
        // cleanup. No PID file, daemon, filesystem socket or Root write exists.
        after_dropping_endpoints(connection, listener, || {
            report["connection_closed"] = json!(true);
            report["listener_closed"] = json!(true);
            report["qualified"] = json!(true);
            Ok(())
        })
    }
    pub(super) fn entry(profile: Profile) -> Result<(), String> {
        let began = Instant::now();
        let mut report = json!({"schema":1,"stage":"startup_proof_complete","profile":profile,"server_pid":std::process::id(),"operation_budget_ms":20000,"listener_bound":false,"listener_closed":false,"response_sent":false,"finished":null,"finished_sent":false,"connection_closed":false,"closed_negative_candidate":false,"artwork_session_completed":false,"qualified":false,"icon_reads":false,"artwork_reads":false,"commands":false,"setting_writes":false,"root_file_writes":false,"daemonized":false,"normal_loader_qualified":false,"production_startup_qualified":false,"context_cli_gets_only":true});
        let result = run(&mut report, profile);
        report["outcome"] = json!(
            if result.is_err() && report["closed_negative_candidate"] == true {
                "context_changed_closed"
            } else if result.is_ok() {
                if report["artwork_session_completed"] == true {
                    "identified_artwork_finished"
                } else {
                    "identified_finished"
                }
            } else {
                "refused"
            }
        );
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
    entry_for_profile(arguments, Profile::Proof)
}

/// Fixed Preview entry; no argument, environment or wire value selects the role.
pub fn preview_main_entry(arguments: &[String]) -> Result<(), String> {
    entry_for_profile(arguments, Profile::Preview)
}

fn entry_for_profile(arguments: &[String], profile: Profile) -> Result<(), String> {
    no_arguments(arguments).map_err(|error| error.to_string())?;
    #[cfg(target_arch = "arm")]
    {
        native::entry(profile)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Err(match profile {
            Profile::Proof => "Installed startup proof requires the admitted ARM Proof package",
            Profile::Preview => "Installed helper requires the admitted ARM Preview package",
        }
        .into())
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
    #[test]
    fn preview_context_is_explicit_and_preserves_pinned_reader_and_sole_card_gates() {
        let mut preview = context();
        preview["foreground"]["appId"] = json!(Profile::Preview.app_id());
        preview["windows"]["windows"][0]["appId"] = json!(Profile::Preview.app_id());
        assert!(checked_context_for_profile(&preview, &"d".repeat(64), Profile::Preview).is_ok());
        assert!(checked_context(&preview, &"d".repeat(64)).is_err());
        assert!(
            checked_context_for_profile(&context(), &"d".repeat(64), Profile::Preview).is_err()
        );
        let mut mixed = preview.clone();
        mixed["windows"]["windows"][0]["appId"] = json!(Profile::Proof.app_id());
        assert!(checked_context_for_profile(&mixed, &"d".repeat(64), Profile::Preview).is_err());
        preview["_reader"]["cleanup_known"] = json!(false);
        assert!(checked_context_for_profile(&preview, &"d".repeat(64), Profile::Preview).is_err());
    }
    #[test]
    fn preview_enrollment_binds_kernel_identity_and_exact_installed_paths() {
        let marker = protocol::Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        for uid in [5724, 7001] {
            let connect = Credentials {
                pid: 123,
                uid,
                gid: uid,
            };
            let peer = protocol::PeerEvidence {
                pid: 123,
                uid_fields: [uid; 4],
                start_ticks: 456,
                exe_sha256: marker.ui_sha256.clone(),
                exe_path: Profile::Preview.peer_exe_path(),
                root_path: Profile::Preview.jail_root(),
            };
            let enrolled = enrolled_peer_for_profile(
                &marker,
                &"d".repeat(64),
                &connect,
                &connect,
                &peer,
                100,
                Profile::Preview,
            )
            .unwrap();
            assert_eq!(enrolled.client_uid, uid);
            assert_eq!(enrolled.expires_unix, 125);
            assert!(
                enrolled_peer(&marker, &"d".repeat(64), &connect, &connect, &peer, 100).is_err()
            );
            let changed_message = Credentials { gid: 0, ..connect };
            assert!(enrolled_peer_for_profile(
                &marker,
                &"d".repeat(64),
                &connect,
                &changed_message,
                &peer,
                100,
                Profile::Preview
            )
            .is_err());
            let mut drifted_uid = peer.clone();
            drifted_uid.uid_fields[2] += 1;
            assert!(enrolled_peer_for_profile(
                &marker,
                &"d".repeat(64),
                &connect,
                &connect,
                &drifted_uid,
                100,
                Profile::Preview
            )
            .is_err());
            for other in [
                protocol::PeerEvidence {
                    exe_path: Profile::Proof.peer_exe_path(),
                    ..peer.clone()
                },
                protocol::PeerEvidence {
                    root_path: Profile::Proof.jail_root(),
                    ..peer.clone()
                },
            ] {
                assert!(enrolled_peer_for_profile(
                    &marker,
                    &"d".repeat(64),
                    &connect,
                    &connect,
                    &other,
                    100,
                    Profile::Preview
                )
                .is_err());
            }
            let proof_peer = protocol::PeerEvidence {
                exe_path: Profile::Proof.peer_exe_path(),
                root_path: Profile::Proof.jail_root(),
                ..peer
            };
            assert!(enrolled_peer_for_profile(
                &marker,
                &"d".repeat(64),
                &connect,
                &connect,
                &proof_peer,
                100,
                Profile::Preview
            )
            .is_err());
            if uid == 5724 {
                assert!(enrolled_peer(
                    &marker,
                    &"d".repeat(64),
                    &connect,
                    &connect,
                    &proof_peer,
                    100
                )
                .is_err());
            }
        }
    }
    #[test]
    fn both_fixed_entries_reject_role_selection_arguments_before_native_dispatch() {
        for arguments in [
            vec!["--profile".into(), "preview".into()],
            vec!["proof".into()],
            vec!["".into()],
        ] {
            assert!(main_entry(&arguments).is_err());
            assert!(preview_main_entry(&arguments).is_err());
        }
        #[cfg(not(target_arch = "arm"))]
        {
            assert!(main_entry(&[]).unwrap_err().contains("ARM Proof"));
            assert!(preview_main_entry(&[]).unwrap_err().contains("ARM Preview"));
        }
    }
}

#[cfg(test)]
mod context_withdrawal_closure_tests {
    use super::*;
    use crate::broker_server::{ContextWithdrawal, WithdrawalStage};
    use serde_json::json;
    fn fixture(stage: WithdrawalStage) -> (Value, ContextWithdrawal, protocol::PeerEvidence) {
        let peer = protocol::PeerEvidence {
            pid: 123,
            uid_fields: [6583; 4],
            start_ticks: 456,
            exe_sha256: "b".repeat(64),
            exe_path: Profile::Proof.peer_exe_path(),
            root_path: Profile::Proof.jail_root(),
        };
        let w = ContextWithdrawal::fixture(
            stage,
            if stage == WithdrawalStage::ReadAfter {
                1
            } else {
                2
            },
            Profile::Proof,
            Credentials {
                pid: 123,
                uid: 6583,
                gid: 5000,
            },
            peer.clone(),
        );
        let report = json!({"profile":Profile::Proof,"qualified":false,"finished_sent":false,"finished":null,"artwork_session_completed":false,"closed_negative_candidate":false,"connection_closed":false,"listener_closed":false,"response_sent":true,"resource_mode":true,"artwork_reads":[{"header":{"encoded_bytes":4},"encoded_data_released":true,"completed":false}]});
        let mut report = report;
        if stage == WithdrawalStage::Finish {
            report["artwork_reads"][0]["complete"] =
                json!({"op":"artwork_complete","request_id":1});
            report["artwork_reads"][0]["completed"] = json!(true);
        }
        (report, w, peer)
    }
    #[test]
    fn descriptor_drops_precede_the_receipt_callback() {
        use std::cell::RefCell;
        use std::rc::Rc;
        struct Owned(&'static str, Rc<RefCell<Vec<&'static str>>>);
        impl Drop for Owned {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let result = after_dropping_endpoints(
            Owned("connection", events.clone()),
            Owned("listener", events.clone()),
            || {
                assert_eq!(&*events.borrow(), &["connection", "listener"]);
                "closed"
            },
        );
        assert_eq!(result, "closed");
    }
    #[test]
    fn candidate_is_negative_preserves_partial_records_and_never_fabricates_finished() {
        for stage in [WithdrawalStage::ReadAfter, WithdrawalStage::Finish] {
            let (mut report, w, peer) = fixture(stage);
            let partial = report["artwork_reads"].clone();
            report["connection_closed"] = json!(true);
            report["listener_closed"] = json!(true);
            record_closed_withdrawal(&mut report, &w, &peer, json!({"root_process":{"pid":7}}))
                .unwrap();
            assert_eq!(
                report["failure"],
                json!({"kind":"context_changed","stage":stage.as_str(),"request_id":w.request_id()})
            );
            assert_eq!(report["closed_negative_candidate"], true);
            assert_eq!(report["qualified"], false);
            assert_eq!(report["finished_sent"], false);
            assert!(report["finished"].is_null());
            assert_eq!(report["artwork_session_completed"], false);
            assert_eq!(report["artwork_reads"], partial);
            assert_eq!(report["closed_negative_acceptance_enabled"], false);
            assert_eq!(
                report["context_withdrawal_message_credentials"],
                json!({"pid":123,"uid":6583,"gid":5000})
            );
        }
    }
    #[test]
    fn closure_peer_or_success_contradiction_never_becomes_candidate() {
        for (field, value) in [
            ("connection_closed", json!(false)),
            ("listener_closed", json!(false)),
            ("qualified", json!(true)),
            ("finished_sent", json!(true)),
            ("finished", json!({"fake":true})),
            ("artwork_finished", json!({"fake":true})),
            ("artwork_session_completed", json!(true)),
            ("response_sent", json!(false)),
            ("resource_mode", json!(false)),
        ] {
            let (mut report, w, peer) = fixture(WithdrawalStage::Finish);
            report["connection_closed"] = json!(true);
            report["listener_closed"] = json!(true);
            report[field] = value;
            assert!(record_closed_withdrawal(&mut report, &w, &peer, json!({})).is_err());
            assert_eq!(report["closed_negative_candidate"], false);
        }
        let (mut report, w, mut peer) = fixture(WithdrawalStage::Finish);
        report["connection_closed"] = json!(true);
        report["listener_closed"] = json!(true);
        peer.start_ticks += 1;
        assert!(record_closed_withdrawal(&mut report, &w, &peer, json!({})).is_err());
        let (mut report, w, peer) = fixture(WithdrawalStage::Finish);
        report["connection_closed"] = json!(true);
        report["listener_closed"] = json!(true);
        report["profile"] = json!(Profile::Preview);
        assert!(record_closed_withdrawal(&mut report, &w, &peer, json!({})).is_err());
    }
}
