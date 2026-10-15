//! One-shot ordinary-process artwork transfer proof. Never used by the normal
//! refresh worker; no privileged launch, filesystem fallback or GL upload.
#[cfg(any(target_arch = "arm", test))]
use crate::app_artwork::{RootClass, Source, SourceMap};
#[cfg(any(target_arch = "arm", test))]
use crate::artwork_transfer::Header;
use serde_json::Value;
#[cfg(target_arch = "arm")]
fn digest(bytes: &[u8]) -> String {
    let mut state = plx_base::sha256::Sha256::new();
    state.update(bytes);
    state.finish().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(any(target_arch = "arm", test))]
fn selections(map: &SourceMap) -> Vec<Source> {
    RootClass::ALL
        .into_iter()
        .filter_map(|class| {
            map.values()
                .filter_map(|entry| entry.icon.as_ref())
                .find(|source| source.root_class() == class)
                .cloned()
        })
        .collect()
}
#[cfg(any(target_arch = "arm", test))]
fn header_matches(header: &Header, source: &Source) -> bool {
    let key = source.key();
    header.app_id == key.app_id
        && header.version == key.version
        && RootClass::from(header.class) == key.class
        && crate::app_artwork::IconKind::from(header.kind) == key.kind
}

#[cfg(target_arch = "arm")]
pub fn collect(raw: &Value) -> Value {
    use crate::artwork_transfer as wire;
    use crate::broker_probe::{
        bind_response, context_valid, own_identity, read_context, read_marker,
    };
    use crate::broker_protocol::{Operation, Response};
    use crate::broker_transport::Connection;
    use serde_json::json;
    use std::time::{Duration, Instant};
    let origin = Instant::now();
    let mut report = json!({"schema":1,"app_id":plx_base::paths::app_id(),"uid":unsafe{libc::geteuid()},
        "handshake_only":false,"icon_reads":true,"decoder":true,"normal_loader_qualified":false,
        "root_filesystem_access_granted":false,"server_executable_independently_measured_by_client":false,
        "request_attempted":false,"request_sent":false,"reads":[],"verified":false,"session_finished":false,"clean_eof":false});
    let mut stage = "identity";
    let result = (|| -> Result<(), String> {
        let own = own_identity()?;
        report["own_before"] = json!(own);
        let marker = read_marker()?;
        if marker.ui_sha256 != own.ui_sha256 {
            return Err("Marker UI identity mismatch".into());
        }
        report["marker"] = json!(marker);
        stage = "catalogue_selection";
        let map = crate::app_artwork::sources_from_raw(raw, crate::identity::PROOF)
            .map_err(|e| format!("Artwork catalogue unavailable: {e:?}"))?;
        let chosen = selections(&map);
        report["selected_sources"] = json!(chosen.iter().map(Source::key).collect::<Vec<_>>());
        report["selected_class_count"] = json!(chosen.len());
        if chosen.is_empty() {
            return Err("No validated catalogue icons".into());
        }
        stage = "foreground";
        let bus = plx_platform::webos::ls2::register().map_err(|e| e.to_string())?;
        let gate = Instant::now() + Duration::from_secs(5);
        loop {
            let context = read_context(&bus, gate);
            let valid = context_valid(&context);
            report["initial_context"] = context;
            if valid {
                break;
            }
            let left = gate.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err("Proof sole-CARD foreground unavailable".into());
            }
            std::thread::sleep(left.min(Duration::from_millis(100)));
        }
        stage = "connect";
        let connection =
            Connection::connect_until(&marker.nonce, Instant::now() + Duration::from_secs(15))
                .map_err(|e| e.to_string())?;
        let peer = connection.peer_credentials().map_err(|e| e.to_string())?;
        report["server_peer"] = json!({"pid":peer.pid,"uid":peer.uid,"gid":peer.gid});
        if peer.pid <= 1 || peer.uid != 0 {
            return Err("Peer is not Root".into());
        }
        let identity = Response {
            schema: 1,
            op: Operation::Identify,
            nonce: marker.nonce.clone(),
            server_pid: peer.pid as u32,
            server_uid: 0,
            client_pid: own.pid,
            client_uid: own.uid,
            client_start_ticks: own.start_ticks,
            ui_sha256: marker.ui_sha256.clone(),
            broker_sha256: marker.broker_sha256.clone(),
        };
        identity.validate().map_err(|e| e.to_string())?;
        // This is a single bounded session, not five renewed time budgets.
        let deadline = Instant::now() + Duration::from_secs(24);
        let mut decoded_count = 0_usize;
        for (index, source) in chosen.iter().enumerate() {
            stage = "artwork_exchange";
            let request = wire::Request::new(
                index as u32 + 1,
                &marker.nonce,
                source.app_id(),
                wire::Variant::Icon,
            )
            .map_err(|e| e.to_string())?;
            let mut record = json!({"request":request,"source_key":source.key(),"decoded":false});
            let operation = (|| -> Result<(), String> {
                let context = read_context(&bus, deadline);
                let valid = context_valid(&context);
                record["context_before"] = context;
                if !valid || own_identity()? != own {
                    return Err("Context changed before artwork".into());
                }
                report["request_attempted"] = json!(true);
                wire::send_request(&connection, &request, deadline).map_err(|e| e.to_string())?;
                report["request_sent"] = json!(true);
                let exchange = wire::receive(&connection, &request, &identity, peer, deadline);
                // Check context even if transport/parse refused the transaction.
                let context = read_context(&bus, deadline);
                let valid = context_valid(&context);
                record["context_after"] = context;
                let after = own_identity()?;
                record["own_after"] = json!(after);
                if !valid {
                    return Err("Context changed after artwork".into());
                }
                match exchange.map_err(|e| e.to_string())? {
                    wire::Outcome::Refused(refusal) => {
                        record["refusal"] = json!(refusal);
                    }
                    wire::Outcome::Artwork(artwork) => {
                        bind_response(&marker, &own, &after, peer, peer, &artwork.header.identity)?;
                        if !header_matches(&artwork.header, source) {
                            return Err("Server catalogue source differs".into());
                        }
                        record["header"] = json!(artwork.header);
                        record["complete"] = json!(artwork.complete);
                        let (width, height, rgba) = plx_gfx::img::img_decode_owned_bounded(
                            &artwork.bytes,
                            512,
                            1024 * 1024,
                        )
                        .ok_or("Bounded ordinary decoder declined encoded artwork")?;
                        record["decode"] = json!({"width":width,"height":height,"rgba_bytes":rgba.len(),
                            "rgba_sha256":digest(&rgba),"encoded_bytes":artwork.bytes.len(),
                            "encoded_sha256":digest(&artwork.bytes),"max_axis":512,"max_rgba_bytes":1048576});
                        record["decoded"] = json!(true);
                        decoded_count += 1;
                    }
                }
                Ok(())
            })();
            if let Err(error) = &operation {
                record["error"] = json!(error);
            }
            report["reads"]
                .as_array_mut()
                .ok_or("Missing read receipts")?
                .push(record);
            operation?;
        }
        stage = "finish";
        let finish = wire::FinishRequest::new(chosen.len() as u32 + 1, &marker.nonce)
            .map_err(|e| e.to_string())?;
        wire::send_finish_request(&connection, &finish, deadline).map_err(|e| e.to_string())?;
        let finished = wire::receive_finished(&connection, &finish, &identity, peer, deadline)
            .map_err(|e| e.to_string())?;
        report["finished"] = json!(finished);
        report["session_finished"] = json!(true);
        report["clean_eof"] = json!(true);
        let context = read_context(&bus, deadline);
        let valid = context_valid(&context);
        report["final_context"] = context;
        let after = own_identity()?;
        report["own_after"] = json!(after);
        bind_response(&marker, &own, &after, peer, peer, &finished.identity)?;
        if !valid || decoded_count != chosen.len() {
            return Err("Final context or class coverage unqualified".into());
        }
        report["decoded_class_count"] = json!(decoded_count);
        report["verified"] = json!(true);
        Ok(())
    })();
    report["elapsed_ms"] = serde_json::json!(origin.elapsed().as_millis());
    report["euid_after"] = serde_json::json!(unsafe { libc::geteuid() });
    match result {
        Ok(()) => report["outcome"] = serde_json::json!("artwork_decoded"),
        Err(error) => {
            report["outcome"] = serde_json::json!("unqualified");
            report["stage"] = serde_json::json!(stage);
            report["error"] = serde_json::json!(error);
        }
    }
    report
}
#[cfg(not(target_arch = "arm"))]
pub fn collect(_: &Value) -> Value {
    serde_json::json!({"schema":1,"unsupported_host":true,"verified":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn chooses_one_sorted_icon_per_observed_class_and_excludes_own() {
        let apps = [
            ("fixture.z", RootClass::LgSystem),
            ("fixture.a", RootClass::LgSystem),
            ("fixture.dev", RootClass::DeveloperApps),
            (crate::identity::PROOF, RootClass::DeveloperApps),
        ];
        let raw = json!({"replies":{"apps":{"returnValue":true,"apps":apps.iter().map(|(id,class)|
            json!({"id":id,"title":"Fixture","visible":true,"type":"native","version":"1",
                "folderPath":format!("{}/{id}",class.prefix()),"icon":"icon.png"})).collect::<Vec<_>>()},
            "launch_points":{"returnValue":true,"launchPoints":apps.iter().map(|(id,_)|json!({"id":id})).collect::<Vec<_>>()}}});
        let map = crate::app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap();
        let selected = selections(&map);
        assert_eq!(
            selected.iter().map(Source::app_id).collect::<Vec<_>>(),
            vec!["fixture.a", "fixture.dev"]
        );
        assert!(selected.len() <= 5);
        assert!(selections(&SourceMap::new()).is_empty());
        let source = &selected[0];
        let mut header: Header = serde_json::from_value(json!({"schema":1,"op":"artwork","request_id":1,
            "identity":{"schema":1,"op":"identify","nonce":"a".repeat(32),"server_pid":42,"server_uid":0,
                "client_pid":43,"client_uid":6000,"client_start_ticks":1,"ui_sha256":"b".repeat(64),"broker_sha256":"c".repeat(64)},
            "app_id":source.app_id(),"version":"1","class":"lg_system","kind":"icon",
            "file_identity":{"device":1,"inode":1,"bytes":1,"mode":33188,"uid":0,"gid":0,
                "modified_seconds":1,"modified_nanos":0,"changed_seconds":1,"changed_nanos":0},
            "encoded_bytes":1,"encoded_sha256":"d".repeat(64),"chunk_count":1})).unwrap();
        assert!(header_matches(&header, source));
        header.version = "2".into();
        assert!(!header_matches(&header, source));
        header.version = "1".into();
        header.class = crate::artwork_transfer::Class::DeveloperApps;
        assert!(!header_matches(&header, source));
    }
}
