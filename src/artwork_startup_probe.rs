//! One inert ordinary-Proof HBC capability observation. The caller creates and
//! joins its worker; the LS2 registration stays on that worker. This module has
//! no broker/command/URI selection, elevation, CLI, socket or resource operation.
use serde::Deserialize;
use serde_json::{json, Value};
#[cfg(any(target_arch = "arm", test))]
use std::time::{Duration, Instant};

pub const URI: &str = "luna://org.webosbrew.hbchannel.service/checkRoot";
const RAW_LIMIT: usize = 16 * 1024;
#[cfg(any(target_arch = "arm", test))]
const OPERATIONS_BUDGET: Duration = Duration::from_secs(8);

// Distinguish absent from present-null: an error flag with null still prevents
// a positive result. Derived struct parsing also rejects duplicate/unknown keys.
#[derive(Default)]
enum Present<T> {
    #[default]
    Absent,
    Value(T),
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Present<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::Value)
    }
}
impl<T> Present<T> {
    fn get(&self) -> Option<&T> {
        match self {
            Self::Absent => None,
            Self::Value(value) => Some(value),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    #[serde(default, rename = "returnValue")]
    returned: Present<Value>,
    #[serde(default, rename = "errorCode")]
    code: Present<Value>,
    #[serde(default, rename = "errorText")]
    detail: Present<Value>,
}
fn bounded(text: &str, cap: usize) -> String {
    let mut end = text.len().min(cap);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
fn response(outcome: &str) -> Value {
    json!({"outcome":outcome,"root":null,"raw":null,"raw_bytes":0,
        "raw_truncated":false,"reply":null,"code":null,"detail":null,"stage":null})
}
/// Grades only the SDK-returned UTF-8 reply. The bound is applied AFTER the
/// platform call; it is not a hard limit on transport allocation or SDK work.
pub fn parse_response(raw: &str) -> Value {
    let mut result = response("malformed");
    result["raw_bytes"] = json!(raw.len());
    result["raw"] = json!(bounded(raw, RAW_LIMIT));
    result["raw_truncated"] = json!(raw.len() > RAW_LIMIT);
    if raw.len() > RAW_LIMIT {
        result["outcome"] = json!("oversize");
        return result;
    }
    let parsed: Reply = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            result["stage"] = json!("parse");
            result["detail"] = json!(bounded(&error.to_string(), 4096));
            return result;
        }
    };
    let returned = match parsed.returned.get() {
        Some(Value::Bool(value)) => Some(*value),
        None => None,
        _ => {
            result["detail"] = json!("returnValue must be a literal boolean");
            return result;
        }
    };
    let has_error = parsed.code.get().is_some() || parsed.detail.get().is_some();
    if returned == Some(true) && has_error {
        result["detail"] = json!("A true result contradicts explicit error fields");
        return result;
    }
    if has_error {
        if parsed
            .code
            .get()
            .is_some_and(|value| value.as_i64().is_none() && !value.is_string())
            || parsed.detail.get().is_some_and(|value| !value.is_string())
        {
            result["detail"] = json!("Explicit error code/text has an invalid type");
            return result;
        }
        result["outcome"] = json!("service_refusal");
        result["code"] = parsed.code.get().cloned().unwrap_or(Value::Null);
        result["detail"] = parsed.detail.get().cloned().unwrap_or(Value::Null);
    } else if let Some(root) = returned {
        result["outcome"] = json!(if root { "root_true" } else { "root_false" });
        result["root"] = json!(root);
    } else {
        result["detail"] = json!("Missing literal returnValue or explicit error envelope");
        return result;
    }
    // Accepted shapes have only typed scalar fields, so no nested ambiguous
    // Value can be normalized into an apparently valid reply here.
    result["reply"] = serde_json::from_str(raw).unwrap_or(Value::Null);
    result
}

#[cfg(any(target_arch = "arm", test))]
#[derive(Clone)]
enum Failure {
    Timeout,
    Setup {
        stage: String,
        code: Option<i32>,
        detail: String,
    },
}
#[cfg(any(target_arch = "arm", test))]
impl Failure {
    fn evidence(&self) -> Value {
        match self {
            Self::Timeout => json!({"stage":"call","code":null,"detail":"SDK call timeout"}),
            Self::Setup {
                stage,
                code,
                detail,
            } => json!({"stage":bounded(stage,256),
                "code":code,"detail":bounded(detail,RAW_LIMIT)}),
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
trait Backend {
    fn app_id(&self) -> &str;
    fn uid(&self) -> u32;
    fn now(&self) -> Instant;
    fn pause(&mut self, duration: Duration);
    fn identity(&mut self) -> Result<crate::broker_probe::OwnIdentity, String>;
    fn register(&mut self) -> Result<(), Failure>;
    fn context(&mut self, deadline: Instant) -> Value;
    fn check_root(&mut self, budget: Duration) -> Result<String, Failure>;
}
#[cfg(any(target_arch = "arm", test))]
fn report(app_id: &str, uid: u32) -> Value {
    json!({"schema":1,"app_id":app_id,"uid":uid,"inert_check_only":true,"uri":URI,
        "payload":{},"registered":false,"request_attempted":false,"check_root_calls":0,
        "own_before":null,"own_at_call":null,"own_after":null,"context_before":null,
        "context_after":null,"foreground_checks":0,"identity_stable":false,
        "context_valid_before":false,"context_valid_after":false,"response":null,
        "outcome":"not_attempted","verified":false,"negative_observation_verified":false,
        "operations_budget_ms":8000,"call_budget_ms":0,"operation_deadline_expired":false,
        "reported_elapsed_ms":0,"command_execution_requested":false,"broker_child_requested":false,
        "elevation_requested":false,"resource_reads":false,"normal_loader_qualified":false,
        "production_startup_qualified":false,"error":null})
}
#[cfg(any(target_arch = "arm", test))]
fn valid_own(own: &crate::broker_probe::OwnIdentity, uid: u32) -> bool {
    own.pid > 1
        && own.uid == uid
        && uid >= 1000
        && uid != 5724
        && own.start_ticks > 0
        && own.ui_sha256.len() == 64
        && own
            .ui_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
#[cfg(any(target_arch = "arm", test))]
fn run(backend: &mut impl Backend) -> Value {
    use crate::broker_probe::context_valid;
    let began = backend.now();
    let mut output = report(backend.app_id(), backend.uid());
    let mut operations_deadline = None;
    let observed = (|| -> Result<(), (&'static str, Value)> {
        if backend.app_id() != crate::identity::PROOF {
            return Err((
                "identity_failure",
                json!("Only the ordinary Proof caller is admitted"),
            ));
        }
        let own = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        output["own_before"] = json!(own);
        if !valid_own(&own, backend.uid()) {
            return Err((
                "identity_failure",
                json!("Invalid ordinary Proof self identity"),
            ));
        }
        backend
            .register()
            .map_err(|error| ("registration_failure", error.evidence()))?;
        output["registered"] = json!(true);
        // Initial local identity/registration and eventual SDK drop are not
        // cancellable waits. These operation deadlines are not a hard wall time.
        let deadline = backend.now() + OPERATIONS_BUDGET;
        operations_deadline = Some(deadline);
        let gate = backend.now() + Duration::from_secs(5);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let context = backend.context(gate);
            let valid = context_valid(&context);
            output["context_before"] = context;
            output["foreground_checks"] = json!(attempts);
            output["context_valid_before"] = json!(valid);
            if backend.now() >= gate {
                return Err((
                    "context_unavailable",
                    json!("Context completed after the 5 second foreground gate"),
                ));
            }
            if valid {
                break;
            }
            let left = gate.saturating_duration_since(backend.now());
            if left.is_zero() {
                return Err((
                    "context_unavailable",
                    json!("Sole Proof CARD/model gate unavailable within 5 seconds"),
                ));
            }
            backend.pause(left.min(Duration::from_millis(100)));
        }
        let at_call = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        output["own_at_call"] = json!(at_call);
        if at_call != own || !valid_own(&at_call, backend.uid()) {
            return Err((
                "identity_changed",
                json!("Self identity changed before the inert request"),
            ));
        }
        let remaining = deadline.saturating_duration_since(backend.now());
        if remaining.is_zero() {
            return Err((
                "deadline",
                json!("Operation deadline expired before request"),
            ));
        }
        let budget = remaining.min(Duration::from_secs(1));
        output["request_attempted"] = json!(true);
        output["check_root_calls"] = json!(1);
        output["call_budget_ms"] = json!(budget.as_millis());
        let received = match backend.check_root(budget) {
            Ok(raw) => parse_response(&raw),
            Err(error) => {
                let mut failure = response(match error {
                    Failure::Timeout => "timeout",
                    Failure::Setup { .. } => "transport_setup_failure",
                });
                let evidence = error.evidence();
                for key in ["stage", "code", "detail"] {
                    failure[key] = evidence[key].clone();
                }
                failure
            }
        };
        output["response"] = received;
        let after_context = backend.context(deadline);
        let after_valid = context_valid(&after_context);
        output["context_after"] = after_context;
        output["context_valid_after"] = json!(after_valid);
        let after = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        let stable = own == after && at_call == after && valid_own(&after, backend.uid());
        output["own_after"] = json!(after);
        output["identity_stable"] = json!(stable);
        if backend.now() >= deadline {
            return Err(("deadline", json!("Operation returned after its deadline")));
        }
        if !stable {
            return Err((
                "identity_changed",
                json!("Self identity changed after the inert request"),
            ));
        }
        if !after_valid {
            return Err((
                "context_changed",
                json!("Proof model/foreground/window context unavailable after request"),
            ));
        }
        let root_true = output["response"]["outcome"] == "root_true";
        let root_false = output["response"]["outcome"] == "root_false";
        output["verified"] = json!(root_true);
        output["negative_observation_verified"] = json!(root_false);
        output["outcome"] = output["response"]["outcome"].clone();
        Ok(())
    })();
    if let Err((outcome, error)) = observed {
        output["outcome"] = json!(outcome);
        output["error"] = error;
    }
    output["operation_deadline_expired"] =
        json!(operations_deadline.is_some_and(|end| backend.now() >= end));
    output["reported_elapsed_ms"] =
        json!(backend.now().saturating_duration_since(began).as_millis());
    output
}

#[cfg(target_arch = "arm")]
#[derive(Default)]
struct Native {
    bus: Option<plx_platform::webos::ls2::Registration>,
}
#[cfg(target_arch = "arm")]
impl Backend for Native {
    fn app_id(&self) -> &str {
        plx_base::paths::app_id()
    }
    fn uid(&self) -> u32 {
        unsafe { libc::geteuid() }
    }
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn pause(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn identity(&mut self) -> Result<crate::broker_probe::OwnIdentity, String> {
        crate::broker_probe::own_identity()
    }
    fn register(&mut self) -> Result<(), Failure> {
        match plx_platform::webos::ls2::register() {
            Ok(bus) => {
                self.bus = Some(bus);
                Ok(())
            }
            Err(plx_platform::webos::ls2::RegisterFail::Setup {
                stage,
                detail,
                code,
            }) => Err(Failure::Setup {
                stage: stage.into(),
                detail,
                code,
            }),
        }
    }
    fn context(&mut self, deadline: Instant) -> Value {
        match &self.bus {
            Some(bus) => crate::broker_probe::read_context(bus, deadline),
            None => json!({"unavailable":true}),
        }
    }
    fn check_root(&mut self, budget: Duration) -> Result<String, Failure> {
        let Some(bus) = &self.bus else {
            return Err(Failure::Setup {
                stage: "registration".into(),
                code: None,
                detail: "No worker-owned registration".into(),
            });
        };
        match bus.call(URI, "{}", budget.min(Duration::from_secs(1))) {
            Ok(raw) => Ok(raw),
            Err(plx_platform::webos::ls2::Fail::Timeout) => Err(Failure::Timeout),
            Err(plx_platform::webos::ls2::Fail::Setup {
                stage,
                detail,
                code,
            }) => Err(Failure::Setup {
                stage: stage.into(),
                detail,
                code,
            }),
        }
    }
}
/// Invoke ONLY on the caller's diagnostic worker. Native registration creation,
/// use and drop stay on that thread; the caller owns joining and records its
/// actual join. A true reply does not prove a broker, child or resource route.
#[cfg(target_arch = "arm")]
pub fn collect() -> Value {
    run(&mut Native::default())
}
#[cfg(not(target_arch = "arm"))]
pub fn collect() -> Value {
    json!({"schema":1,"unsupported_host":true,"uri":URI,"payload":{},
        "inert_check_only":true,"registered":false,"request_attempted":false,
        "check_root_calls":0,"verified":false,"negative_observation_verified":false,
        "outcome":"unsupported_host","command_execution_requested":false,
        "broker_child_requested":false,"elevation_requested":false,"resource_reads":false,
        "normal_loader_qualified":false,"production_startup_qualified":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_probe::OwnIdentity;
    use std::collections::VecDeque;
    fn own() -> OwnIdentity {
        OwnIdentity {
            pid: 123,
            uid: 6583,
            start_ticks: 456,
            ui_sha256: "a".repeat(64),
        }
    }
    fn context() -> Value {
        json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},
            "foreground":{"returnValue":true,"appId":crate::identity::PROOF},
            "windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]}})
    }
    struct Fake {
        origin: Instant,
        elapsed: Duration,
        identities: VecDeque<Result<OwnIdentity, String>>,
        contexts: VecDeque<Value>,
        answer: Result<String, Failure>,
        register_error: Option<Failure>,
        calls: u32,
        budgets: Vec<Duration>,
        context_cost: Duration,
        call_cost: Duration,
        caller: &'static str,
        uid: u32,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                origin: Instant::now(),
                elapsed: Duration::ZERO,
                identities: VecDeque::new(),
                contexts: VecDeque::new(),
                answer: Ok("{\"returnValue\":true}".into()),
                register_error: None,
                calls: 0,
                budgets: Vec::new(),
                context_cost: Duration::ZERO,
                call_cost: Duration::ZERO,
                caller: crate::identity::PROOF,
                uid: 6583,
            }
        }
    }
    impl Backend for Fake {
        fn app_id(&self) -> &str {
            self.caller
        }
        fn uid(&self) -> u32 {
            self.uid
        }
        fn now(&self) -> Instant {
            self.origin + self.elapsed
        }
        fn pause(&mut self, d: Duration) {
            self.elapsed += d;
        }
        fn identity(&mut self) -> Result<OwnIdentity, String> {
            self.identities.pop_front().unwrap_or_else(|| Ok(own()))
        }
        fn register(&mut self) -> Result<(), Failure> {
            self.register_error.take().map_or(Ok(()), Err)
        }
        fn context(&mut self, _: Instant) -> Value {
            self.elapsed += self.context_cost;
            self.contexts.pop_front().unwrap_or_else(context)
        }
        fn check_root(&mut self, budget: Duration) -> Result<String, Failure> {
            self.calls += 1;
            self.budgets.push(budget);
            self.elapsed += self.call_cost;
            self.answer.clone()
        }
    }
    #[test]
    fn exact_bare_boolean_results_preserve_positive_and_negative_meaning() {
        for (raw, outcome, root) in [
            ("{\"returnValue\":true}", "root_true", true),
            (" {\"returnValue\":false}\n", "root_false", false),
        ] {
            let r = parse_response(raw);
            assert_eq!(r["outcome"], outcome);
            assert_eq!(r["root"], root);
            assert_eq!(r["raw"], raw);
        }
    }
    #[test]
    fn explicit_error_is_refusal_not_a_guess_about_permission_or_service_presence() {
        for raw in [
            "{\"returnValue\":false,\"errorCode\":-1,\"errorText\":\"service denied or missing\"}",
            "{\"errorCode\":\"E_FAULT\",\"errorText\":\"opaque detail\"}",
        ] {
            let r = parse_response(raw);
            assert_eq!(r["outcome"], "service_refusal");
            assert!(r["root"].is_null());
            assert!(r["reply"].is_object());
            assert_eq!(r["raw"], raw);
        }
    }
    #[test]
    fn contradictory_flags_nulls_coercion_duplicates_and_unknown_fields_never_qualify() {
        for raw in [
            "{\"returnValue\":true,\"errorCode\":0}",
            "{\"returnValue\":true,\"errorText\":null}",
            "{\"returnValue\":\"true\"}",
            "{\"returnValue\":1}",
            "{\"returnValue\":null}",
            "{\"returnValue\":null,\"returnValue\":true}",
            "{\"returnValue\":false,\"returnValue\":true}",
            "{\"returnValue\":true,\"extra\":false}",
            "{\"returnValue\":false,\"errorCode\":true}",
            "{\"returnValue\":false,\"errorCode\":{\"a\":1,\"a\":2}}",
            "{}",
            "[]",
            "{",
            "{\"returnValue\":true} {}",
        ] {
            let r = parse_response(raw);
            assert_eq!(r["outcome"], "malformed", "{raw}");
            assert!(r["root"].is_null());
        }
    }
    #[test]
    fn post_call_oversize_retains_bounded_utf8_prefix_and_actual_byte_count() {
        let raw = "é".repeat(RAW_LIMIT);
        let r = parse_response(&raw);
        assert_eq!(r["outcome"], "oversize");
        assert_eq!(r["raw_bytes"], raw.len());
        assert_eq!(r["raw_truncated"], true);
        assert!(r["raw"].as_str().unwrap().len() <= RAW_LIMIT);
        assert!(r["reply"].is_null());
    }
    #[test]
    fn verified_requires_stable_identity_context_and_one_bare_true_call() {
        let mut f = Fake::default();
        let r = run(&mut f);
        assert_eq!(r["verified"], true);
        assert_eq!(r["identity_stable"], true);
        assert_eq!(r["negative_observation_verified"], false);
        assert_eq!(f.calls, 1);
        assert_eq!(f.budgets, [Duration::from_secs(1)]);
        assert_eq!(r["uri"], URI);
        assert_eq!(r["payload"], json!({}));
    }
    #[test]
    fn coherent_false_is_verified_negative_only_without_retry_or_fallback() {
        let mut f = Fake {
            answer: Ok("{\"returnValue\":false}".into()),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "root_false");
        assert_eq!(r["verified"], false);
        assert_eq!(r["negative_observation_verified"], true);
        assert_eq!(f.calls, 1);
    }
    #[test]
    fn timeout_transport_and_registration_failures_remain_distinct_with_codes() {
        for failure in [
            Failure::Timeout,
            Failure::Setup {
                stage: "call".into(),
                code: Some(-9),
                detail: "raw hub words".into(),
            },
        ] {
            let expected = if matches!(failure, Failure::Timeout) {
                "timeout"
            } else {
                "transport_setup_failure"
            };
            let mut f = Fake {
                answer: Err(failure),
                ..Fake::default()
            };
            let r = run(&mut f);
            assert_eq!(r["outcome"], expected);
            assert_eq!(r["verified"], false);
            assert_eq!(f.calls, 1);
        }
        let mut f = Fake {
            register_error: Some(Failure::Setup {
                stage: "attach".into(),
                code: Some(-4),
                detail: "opaque registration failure".into(),
            }),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "registration_failure");
        assert_eq!(r["error"]["code"], -4);
        assert_eq!(f.calls, 0);
    }
    #[test]
    fn wrong_caller_or_retained_root_uid_declines_before_service_registration() {
        for (caller, uid) in [
            (crate::identity::PREVIEW, 6583),
            (crate::identity::PROOF, 5724),
            (crate::identity::PROOF, 0),
        ] {
            let mut f = Fake {
                caller,
                uid,
                ..Fake::default()
            };
            let r = run(&mut f);
            assert_eq!(r["outcome"], "identity_failure");
            assert_eq!(r["registered"], false);
            assert_eq!(f.calls, 0);
        }
    }
    #[test]
    fn late_pid_start_or_elf_change_preserves_reply_without_root_qualification() {
        for changed in [
            OwnIdentity { pid: 124, ..own() },
            OwnIdentity {
                start_ticks: 457,
                ..own()
            },
            OwnIdentity {
                ui_sha256: "b".repeat(64),
                ..own()
            },
        ] {
            let mut f = Fake {
                identities: VecDeque::from([Ok(own()), Ok(own()), Ok(changed)]),
                ..Fake::default()
            };
            let r = run(&mut f);
            assert_eq!(r["outcome"], "identity_changed");
            assert_eq!(r["response"]["outcome"], "root_true");
            assert_eq!(r["verified"], false);
            assert_eq!(f.calls, 1);
        }
    }
    #[test]
    fn pre_call_identity_change_never_sends_and_late_context_change_never_verifies() {
        let mut f = Fake {
            identities: VecDeque::from([
                Ok(own()),
                Ok(OwnIdentity {
                    start_ticks: 999,
                    ..own()
                }),
            ]),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "identity_changed");
        assert_eq!(f.calls, 0);
        let mut changed = context();
        changed["foreground"]["appId"] = json!("another.app");
        let mut f = Fake {
            contexts: VecDeque::from([context(), changed]),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "context_changed");
        assert_eq!(r["response"]["root"], true);
        assert_eq!(r["verified"], false);
    }
    #[test]
    fn foreground_wait_is_finite_and_overdue_reply_cannot_be_promoted() {
        let mut f = Fake {
            contexts: (0..100).map(|_| json!({"unavailable":true})).collect(),
            context_cost: Duration::from_millis(500),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "context_unavailable");
        assert!(f.elapsed >= Duration::from_secs(5));
        assert!(f.elapsed < Duration::from_secs(6));
        assert_eq!(f.calls, 0);
        let mut late_gate = Fake {
            context_cost: Duration::from_secs(6),
            ..Fake::default()
        };
        let late = run(&mut late_gate);
        assert_eq!(late["outcome"], "context_unavailable");
        assert_eq!(late_gate.calls, 0);
        let mut f = Fake {
            call_cost: Duration::from_secs(9),
            ..Fake::default()
        };
        let r = run(&mut f);
        assert_eq!(r["outcome"], "deadline");
        assert_eq!(r["operation_deadline_expired"], true);
        assert_eq!(r["verified"], false);
        assert_eq!(f.calls, 1);
        assert_eq!(f.budgets, [Duration::from_secs(1)]);
    }
    #[cfg(not(target_arch = "arm"))]
    #[test]
    fn host_collect_is_inert_and_makes_no_native_capability_claim() {
        let r = collect();
        assert_eq!(r["outcome"], "unsupported_host");
        assert_eq!(r["check_root_calls"], 0);
        assert_eq!(r["verified"], false);
    }
}
