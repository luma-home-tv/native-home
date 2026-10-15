//! Explicit actual-Preview diagnostic only; ordinary interactive startup stays off.
use crate::{broker_installation::Profile, model::Page, runtime::Window, shell::Shell};
use serde_json::{json, Value};
use std::{
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    time::{Duration, Instant},
};

fn save(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

/// Only the fixed inert checkRoot call executes before this hold. No resource
/// provider, pending ticket or Root command is created here, even on refusal.
pub fn preflight(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut capability = std::thread::Builder::new()
        .name("preview-artwork-capability".into())
        .spawn(|| crate::artwork_startup_probe::collect_for_profile(Profile::Preview))?
        .join()
        .map_err(|_| "Preview inert capability worker panicked")?;
    capability["worker_joined"] = json!(true);
    save(&output.join("preview-artwork-capability.json"), &capability)?;
    let preferences = shell.preferences().clone();
    shell.open(Page::Home);
    let began = Instant::now();
    let mut interrupted = false;
    let mut requests = 0usize;
    while began.elapsed() < Duration::from_secs(8) {
        while let Some(event) = crate::runtime::poll() {
            interrupted |= matches!(event.kind, 1..=8 | 10 | 11);
        }
        requests += shell.take_requests().len();
        shell.draw();
        window.swap();
        std::thread::sleep(Duration::from_millis(17));
    }
    shell.draw();
    let frame = output.join("preview-artwork-preflight-home.ppm");
    window.capture(&frame)?;
    // Only this newly created own diagnostic capture is normalized.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&frame)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    window.swap();
    Ok(
        json!({"capability":capability,"reported_elapsed_ms":began.elapsed().as_millis() as u64,
        "interrupted":interrupted,"requests_empty":requests==0,
        "preferences_unchanged":shell.preferences()==&preferences,"shell_exit":shell.should_exit()}),
    )
}

pub fn admitted(preflight: &Value) -> bool {
    let cap = &preflight["capability"];
    preflight["reported_elapsed_ms"]
        .as_u64()
        .is_some_and(|value| value >= 8000)
        && preflight["interrupted"] == false
        && preflight["requests_empty"] == true
        && preflight["preferences_unchanged"] == true
        && preflight["shell_exit"] == false
        && cap["worker_joined"] == true
        && cap["verified"] == true
        && cap["outcome"] == "root_true"
        && cap["response"]["root"] == true
        && cap["app_id"] == crate::identity::PREVIEW
        && cap["inert_check_only"] == true
        && cap["command_execution_requested"] == false
        && cap["broker_child_requested"] == false
}

pub fn complete(
    output: &Path,
    preflight: Value,
    worker_created: bool,
    observation: Option<Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    if worker_created != admitted(&preflight) || worker_created != observation.is_some() {
        return Err("Preview admission phase ordering is inconsistent".into());
    }
    let qualified = observation
        .as_ref()
        .is_some_and(|value| value["qualified"] == true);
    let report = json!({"schema":1,"profile":"preview","qualified":qualified,
        "capability":preflight["capability"],"preflight":preflight,
        "resource_worker_created":worker_created,"resource_dispatch_enabled":worker_created,
        "root_exec_attempts":if worker_created { Value::Null } else { json!(0) },
        "submitted_jobs":if worker_created { Value::Null } else { json!(0) },
        "outcome":if !worker_created {"capability_declined"} else if qualified {"resource_observed"} else {"resource_failed"},
        "observation":observation,"normal_loader_qualified":false,
        "production_startup_qualified":false,"performance_qualified":false});
    save(&output.join("preview-artwork-admission.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Value {
        json!({"reported_elapsed_ms":8000,"interrupted":false,"requests_empty":true,
            "preferences_unchanged":true,"shell_exit":false,"capability":{
                "worker_joined":true,"verified":true,"outcome":"root_true",
                "response":{"root":true},"app_id":crate::identity::PREVIEW,
                "inert_check_only":true,"command_execution_requested":false,"broker_child_requested":false}})
    }
    #[test]
    fn diagnostic_never_starts_after_inert_failure_or_interruption() {
        assert!(admitted(&ready()));
        for path in [
            "interrupted",
            "requests_empty",
            "preferences_unchanged",
            "shell_exit",
        ] {
            let mut value = ready();
            value[path] = json!(!value[path].as_bool().unwrap());
            assert!(!admitted(&value));
        }
        let mut value = ready();
        value["reported_elapsed_ms"] = json!(7999);
        assert!(!admitted(&value));
        for (key, bad) in [
            ("verified", json!(false)),
            ("worker_joined", json!(false)),
            ("app_id", json!(crate::identity::PROOF)),
            ("outcome", json!("root_false")),
            ("command_execution_requested", json!(true)),
            ("broker_child_requested", json!(true)),
        ] {
            let mut value = ready();
            value["capability"][key] = bad;
            assert!(!admitted(&value));
        }
    }
}
