//! One inert ordinary installed-profile HBC capability observation; default Proof.
//! The caller creates and
//! joins its worker; the LS2 registration stays on that worker. This module has
//! no broker/command/URI selection, elevation, CLI, socket or resource operation.
use crate::broker_installation::Profile;
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
    fn take_context_trace(&mut self) -> Option<crate::broker_probe::ContextTrace> {
        None
    }
    fn check_root(&mut self, budget: Duration) -> Result<String, Failure>;
}
// Bounded observation history only; none of these fields are admission inputs.
#[cfg(any(target_arch = "arm", test))]
#[derive(Default, serde::Serialize)]
struct ContextDiagnostics {
    initial: Vec<InitialContextTrace>,
    after: Option<crate::broker_probe::ContextTrace>,
    trace_truncated: bool,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(serde::Serialize)]
struct InitialContextTrace {
    attempt: u32,
    trace: crate::broker_probe::ContextTrace,
}
#[cfg(any(target_arch = "arm", test))]
impl ContextDiagnostics {
    fn initial(&mut self, attempt: u32, trace: Option<crate::broker_probe::ContextTrace>) {
        let Some(trace) = trace else {
            self.trace_truncated = true;
            return;
        };
        self.trace_truncated |= trace.trace_truncated;
        if self.initial.len() == 8 {
            self.initial.remove(1); // Preserve the first and seven most recent sets.
            self.trace_truncated = true;
        }
        self.initial.push(InitialContextTrace { attempt, trace });
    }
    fn after(&mut self, trace: Option<crate::broker_probe::ContextTrace>) {
        self.trace_truncated |= trace.as_ref().is_none_or(|trace| trace.trace_truncated);
        self.after = trace;
    }
}
#[cfg(any(target_arch = "arm", test))]
fn report(app_id: &str, uid: u32) -> Value {
    json!({"schema":1,"app_id":app_id,"uid":uid,"inert_check_only":true,"uri":URI,
        "payload":{},"registered":false,"request_attempted":false,"check_root_calls":0,
        "own_before":null,"own_at_call":null,"own_after":null,"context_first":null,"context_before":null,
        "context_after":null,"foreground_checks":0,"identity_stable":false,
        "context_valid_before":false,"context_valid_after":false,"response":null,
        "outcome":"not_attempted","verified":false,"negative_observation_verified":false,
        "operations_budget_ms":8000,"call_budget_ms":0,"operation_deadline_expired":false,
        "reported_elapsed_ms":0,"command_execution_requested":false,"broker_child_requested":false,
        "elevation_requested":false,"resource_reads":false,"normal_loader_qualified":false,
        "production_startup_qualified":false,"error":null})
}
#[cfg(any(target_arch = "arm", test))]
fn valid_own(own: &crate::broker_probe::OwnIdentity, uid: u32, profile: Profile) -> bool {
    own.pid > 1
        && own.uid == uid
        && crate::broker_probe::ordinary_for_profile(profile.app_id(), uid, profile)
        && own.start_ticks > 0
        && own.ui_sha256.len() == 64
        && own
            .ui_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
#[cfg(any(target_arch = "arm", test))]
fn run(backend: &mut impl Backend) -> Value {
    run_for_profile(backend, Profile::Proof)
}
#[cfg(any(target_arch = "arm", test))]
fn run_for_profile(backend: &mut impl Backend, profile: Profile) -> Value {
    use crate::broker_probe::{context_valid_for_profile, ordinary_for_profile};
    let began = backend.now();
    let mut output = report(backend.app_id(), backend.uid());
    let mut operations_deadline = None;
    let mut context_diagnostics = ContextDiagnostics::default();
    let observed = (|| -> Result<(), (&'static str, Value)> {
        if !ordinary_for_profile(backend.app_id(), backend.uid(), profile) {
            return Err((
                "identity_failure",
                json!("Only the selected ordinary installed caller is admitted"),
            ));
        }
        let own = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        output["own_before"] = json!(own);
        if !valid_own(&own, backend.uid(), profile) {
            return Err((
                "identity_failure",
                json!("Invalid ordinary installed self identity"),
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
            context_diagnostics.initial(attempts, backend.take_context_trace());
            let valid = context_valid_for_profile(&context, profile);
            if attempts == 1 {
                // Diagnostic evidence only. Admission still uses the current
                // before/after contexts, never this potentially stale sample.
                output["context_first"] = context.clone();
            }
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
                    json!("Selected sole CARD/model gate unavailable within 5 seconds"),
                ));
            }
            backend.pause(left.min(Duration::from_millis(100)));
        }
        let at_call = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        output["own_at_call"] = json!(at_call);
        if at_call != own
            || !valid_own(&at_call, backend.uid(), profile)
            || !ordinary_for_profile(backend.app_id(), backend.uid(), profile)
        {
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
        context_diagnostics.after(backend.take_context_trace());
        let after_valid = context_valid_for_profile(&after_context, profile);
        output["context_after"] = after_context;
        output["context_valid_after"] = json!(after_valid);
        let after = backend
            .identity()
            .map_err(|error| ("identity_failure", json!(bounded(&error, RAW_LIMIT))))?;
        let stable = own == after
            && at_call == after
            && valid_own(&after, backend.uid(), profile)
            && ordinary_for_profile(backend.app_id(), backend.uid(), profile);
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
                json!("Selected model/foreground/window context unavailable after request"),
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
    output["context_diagnostics"] = json!({"schema":1,"initial":context_diagnostics.initial,
        "after":context_diagnostics.after,"trace_truncated":context_diagnostics.trace_truncated});
    output
}

#[cfg(target_arch = "arm")]
struct Native {
    profile: Profile,
    bus: Option<plx_platform::webos::ls2::Registration>,
    last_context_trace: Option<crate::broker_probe::ContextTrace>,
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
        crate::broker_probe::own_identity_for_profile(self.profile)
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
        self.last_context_trace = None;
        match &self.bus {
            Some(bus) => {
                let (context, trace) = crate::broker_probe::read_context_observed(bus, deadline);
                self.last_context_trace = Some(trace);
                context
            }
            None => json!({"unavailable":true}),
        }
    }
    fn take_context_trace(&mut self) -> Option<crate::broker_probe::ContextTrace> {
        self.last_context_trace.take()
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
    run(&mut Native {
        profile: Profile::Proof,
        bus: None,
        last_context_trace: None,
    })
}
/// Closed source-selected role; this observes HBC status without executing a command.
#[cfg(target_arch = "arm")]
pub fn collect_for_profile(profile: Profile) -> Value {
    run_for_profile(
        &mut Native {
            profile,
            bus: None,
            last_context_trace: None,
        },
        profile,
    )
}
#[cfg(not(target_arch = "arm"))]
pub fn collect() -> Value {
    collect_for_profile(Profile::Proof)
}
#[cfg(not(target_arch = "arm"))]
pub fn collect_for_profile(_profile: Profile) -> Value {
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
    fn first_concrete_context_survives_deadline_placeholders_without_granting_access() {
        let mut initial = context();
        initial["foreground"]["appId"] = json!("another.app");
        let exhausted = json!({"system_info":{"deadline":true},"foreground":{"deadline":true},"windows":{"deadline":true}});
        let mut fake = Fake {
            context_cost: Duration::from_millis(100),
            ..Default::default()
        };
        fake.contexts.push_back(initial.clone());
        fake.contexts
            .extend(std::iter::repeat_n(exhausted.clone(), 64));
        let report = run(&mut fake);
        assert_eq!(report["context_first"], initial);
        assert_eq!(report["context_before"], exhausted);
        assert_eq!(report["outcome"], "context_unavailable");
        assert_eq!(fake.calls, 0);
        assert_eq!(report["verified"], false);
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
    fn preview_backend(uid: u32) -> Fake {
        let identity = OwnIdentity { uid, ..own() };
        let mut preview = context();
        preview["foreground"]["appId"] = json!(crate::identity::PREVIEW);
        preview["windows"]["windows"][0]["appId"] = json!(crate::identity::PREVIEW);
        Fake {
            caller: crate::identity::PREVIEW,
            uid,
            identities: VecDeque::from([Ok(identity.clone()), Ok(identity.clone()), Ok(identity)]),
            contexts: VecDeque::from([preview.clone(), preview]),
            ..Fake::default()
        }
    }
    #[test]
    fn preview_inert_check_requires_explicit_role_and_accepts_dynamic_ordinary_uids() {
        for uid in [5724, 7001] {
            let mut f = preview_backend(uid);
            let r = run_for_profile(&mut f, Profile::Preview);
            assert_eq!(r["verified"], true);
            assert_eq!(r["app_id"], crate::identity::PREVIEW);
            assert_eq!(r["uid"], uid);
            assert_eq!(r["check_root_calls"], 1);
            assert_eq!(r["uri"], URI);
            assert_eq!(r["payload"], json!({}));
            for key in [
                "command_execution_requested",
                "broker_child_requested",
                "elevation_requested",
                "resource_reads",
                "normal_loader_qualified",
                "production_startup_qualified",
            ] {
                assert_eq!(r[key], false);
            }
            let mut default_role = preview_backend(uid);
            let refused = run(&mut default_role);
            assert_eq!(refused["outcome"], "identity_failure");
            assert!(!refused["registered"].as_bool().unwrap());
            assert_eq!(default_role.calls, 0);
        }
    }
    #[test]
    fn profile_or_cross_card_mismatch_declines_before_the_inert_service_call() {
        let mut proof = Fake::default();
        let r = run_for_profile(&mut proof, Profile::Preview);
        assert_eq!(r["outcome"], "identity_failure");
        assert_eq!(r["registered"], false);
        assert_eq!(proof.calls, 0);
        let mut preview = preview_backend(5724);
        preview.contexts = (0..100).map(|_| context()).collect();
        let r = run_for_profile(&mut preview, Profile::Preview);
        assert_eq!(r["outcome"], "context_unavailable");
        assert_eq!(preview.calls, 0);
        let mut wrong_app = preview_backend(5724);
        wrong_app.caller = "com.ash.nativehome.preview.other";
        let r = run_for_profile(&mut wrong_app, Profile::Preview);
        assert_eq!(r["registered"], false);
        assert_eq!(wrong_app.calls, 0);
    }
    #[test]
    fn preview_late_identity_drift_never_qualifies_and_bare_false_stays_negative() {
        let mut changed = preview_backend(5724);
        changed.identities[2] = Ok(OwnIdentity {
            uid: 5724,
            start_ticks: 999,
            ..own()
        });
        let r = run_for_profile(&mut changed, Profile::Preview);
        assert_eq!(r["outcome"], "identity_changed");
        assert_eq!(r["response"]["root"], true);
        assert_eq!(r["verified"], false);
        assert_eq!(r["negative_observation_verified"], false);
        assert_eq!(changed.calls, 1);
        let mut negative = preview_backend(5724);
        negative.answer = Ok("{\"returnValue\":false}".into());
        let r = run_for_profile(&mut negative, Profile::Preview);
        assert_eq!(r["outcome"], "root_false");
        assert_eq!(r["verified"], false);
        assert_eq!(r["negative_observation_verified"], true);
        assert_eq!(negative.calls, 1);
    }
    #[cfg(not(target_arch = "arm"))]
    #[test]
    fn host_collect_is_inert_and_makes_no_native_capability_claim() {
        let r = collect();
        assert_eq!(r["outcome"], "unsupported_host");
        assert_eq!(r["check_root_calls"], 0);
        assert_eq!(r["verified"], false);
        for profile in [Profile::Proof, Profile::Preview] {
            let r = collect_for_profile(profile);
            assert_eq!(r["outcome"], "unsupported_host");
            assert_eq!(r["check_root_calls"], 0);
            assert_eq!(r["verified"], false);
        }
    }
    #[test]
    fn context_trace_history_keeps_first_and_recent_sets_with_after_independent() {
        let trace = || crate::broker_probe::ContextTrace {
            operations: Vec::new(),
            trace_truncated: false,
        };
        let mut diagnostics = ContextDiagnostics::default();
        for attempt in 1..=100 {
            diagnostics.initial(attempt, Some(trace()));
        }
        diagnostics.after(Some(trace()));
        assert_eq!(diagnostics.initial.len(), 8);
        assert_eq!(diagnostics.initial[0].attempt, 1);
        assert_eq!(diagnostics.initial[1].attempt, 94);
        assert_eq!(diagnostics.initial[7].attempt, 100);
        assert!(diagnostics.after.is_some());
        assert!(diagnostics.trace_truncated);
        let mut missing = ContextDiagnostics::default();
        missing.initial(1, None);
        missing.after(None);
        assert!(missing.trace_truncated);
        assert!(missing.initial.is_empty());
    }
    #[test]
    fn diagnostic_metadata_does_not_supply_context_or_change_capability_policy() {
        struct ObservedFake(Fake);
        impl Backend for ObservedFake {
            fn app_id(&self) -> &str {
                self.0.app_id()
            }
            fn uid(&self) -> u32 {
                self.0.uid()
            }
            fn now(&self) -> Instant {
                self.0.now()
            }
            fn pause(&mut self, duration: Duration) {
                self.0.pause(duration);
            }
            fn identity(&mut self) -> Result<OwnIdentity, String> {
                self.0.identity()
            }
            fn register(&mut self) -> Result<(), Failure> {
                self.0.register()
            }
            fn context(&mut self, deadline: Instant) -> Value {
                self.0.context(deadline)
            }
            fn take_context_trace(&mut self) -> Option<crate::broker_probe::ContextTrace> {
                Some(crate::broker_probe::ContextTrace {
                    operations: Vec::new(),
                    trace_truncated: true,
                })
            }
            fn check_root(&mut self, budget: Duration) -> Result<String, Failure> {
                self.0.check_root(budget)
            }
        }
        for invalid_after in [false, true] {
            let mut ordinary = Fake::default();
            let mut observed = ObservedFake(Fake::default());
            if invalid_after {
                ordinary.contexts = vec![context(), json!({"unavailable":true})].into();
                observed.0.contexts = ordinary.contexts.clone();
            }
            let mut expected = run(&mut ordinary);
            let mut actual = run(&mut observed);
            assert_eq!(ordinary.calls, 1);
            assert_eq!(observed.0.calls, 1);
            assert_eq!(
                actual["outcome"],
                if invalid_after {
                    "context_changed"
                } else {
                    "root_true"
                }
            );
            assert_eq!(actual["verified"], !invalid_after);
            expected
                .as_object_mut()
                .unwrap()
                .remove("context_diagnostics");
            actual
                .as_object_mut()
                .unwrap()
                .remove("context_diagnostics");
            assert_eq!(actual, expected);
        }
    }
}
