use kerf::report::sha256_hex;
use semantic_architecture::observe::{ObservationPlan, ObservationStatus, observe};
use serde_json::Value;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn plan_value() -> Value {
    let true_sha256 = sha256_hex(&std::fs::read("/usr/bin/true").expect("true must be readable"));
    let printf_sha256 =
        sha256_hex(&std::fs::read("/usr/bin/printf").expect("printf must be readable"));
    serde_json::json!({
        "schema_version": "kerf.observation-plan.v1",
        "subject": {
            "schema_version": "kerf.cert.lts-subject.v1",
            "machine": {
                "initial": "locked",
                "actions": ["toggle"],
                "states": [
                    {
                        "id": "locked",
                        "observation": "locked",
                        "transitions": {
                            "toggle": {"to": "unlocked", "emission": "changed"}
                        }
                    },
                    {
                        "id": "unlocked",
                        "observation": "unlocked",
                        "transitions": {
                            "toggle": {"to": "locked", "emission": "changed"}
                        }
                    }
                ]
            },
            "context": null
        },
        "binding": {
            "id": "printf-toggle",
            "working_directory": env!("CARGO_MANIFEST_DIR"),
            "environment": {},
            "open": {"program": "/usr/bin/true", "program_sha256": true_sha256, "expected_control": {"exit_code": 0}},
            "reset": {"program": "/usr/bin/true", "program_sha256": true_sha256, "expected_control": {"exit_code": 0}},
            "initial_probe": {
                "program": "/usr/bin/printf",
                "program_sha256": printf_sha256,
                "args": ["locked"]
            },
            "close": {"program": "/usr/bin/true", "program_sha256": true_sha256, "expected_control": {"exit_code": 0}},
            "action_invocations": [{
                "action": "toggle",
                "invocation": {
                    "program": "/usr/bin/printf",
                    "program_sha256": printf_sha256,
                    "args": ["changed|unlocked"]
                }
            }],
            "output_classes": [
                {
                    "id": "initial-locked",
                    "exit_code": 0,
                    "stdout_hex": "6c6f636b6564",
                    "stderr_hex": "",
                    "observation": "locked"
                },
                {
                    "id": "changed-unlocked",
                    "exit_code": 0,
                    "stdout_hex": "6368616e6765647c756e6c6f636b6564",
                    "stderr_hex": "",
                    "observation": "unlocked",
                    "emission": "changed"
                }
            ],
            "negative_controls": [{
                "id": "nearby-bogus-output",
                "invocation": {
                    "program": "/usr/bin/printf",
                    "program_sha256": printf_sha256,
                    "args": ["bogus"]
                },
                "expected_rejection": "output belongs to no modeled observation class"
            }]
        },
        "actions": ["toggle"],
        "repetitions": 2
    })
}

fn parse(value: &Value) -> ObservationPlan {
    serde_json::from_value(value.clone()).expect("observation plan must parse")
}

#[test]
fn real_command_observations_emit_an_accepted_trace_certificate() {
    let result = observe(&parse(&plan_value())).expect("observation must execute");
    assert_eq!(result.status, ObservationStatus::Conformant);
    assert!(result.deterministic_raw_observations);
    assert!(result.negative_controls_passed);
    assert_eq!(result.binding_sha256.len(), 64);
    assert_eq!(result.subject_sha256.len(), 64);
    assert!(
        result
            .verification
            .as_ref()
            .is_some_and(|item| item.accepted)
    );
    assert!(result.certificate.is_some());
    assert_eq!(result.receipts.len(), 14);
}

#[test]
fn real_observation_that_disagrees_with_model_is_a_completed_contradiction() {
    let mut value = plan_value();
    value["binding"]["output_classes"][1]["emission"] = Value::String("wrong".to_owned());
    let result = observe(&parse(&value)).expect("observation must execute");
    assert_eq!(result.status, ObservationStatus::Contradicted);
    let verification = result
        .verification
        .expect("contradiction needs checker output");
    assert!(!verification.accepted);
    assert_eq!(
        verification.rejection.expect("witness required").code,
        "trace_emission_mismatch"
    );
}

#[test]
fn negative_control_matching_a_class_prevents_conformance() {
    let mut value = plan_value();
    value["binding"]["negative_controls"][0]["invocation"]["args"] = serde_json::json!(["locked"]);
    let result = observe(&parse(&value)).expect("observation must execute");
    assert_eq!(result.status, ObservationStatus::NegativeControlFailed);
    assert!(!result.negative_controls_passed);
    assert!(result.diagnostic.is_some());
}

#[test]
fn differing_raw_repetitions_are_nondeterministic_not_a_pass() {
    let mut value = plan_value();
    let shell_sha256 = sha256_hex(&std::fs::read("/bin/sh").expect("shell must be readable"));
    value["binding"]["initial_probe"] = serde_json::json!({
        "program": "/bin/sh",
        "program_sha256": shell_sha256,
        "args": ["-c", "printf %s $$"]
    });
    let result = observe(&parse(&value)).expect("observation must execute");
    assert_eq!(result.status, ObservationStatus::Nondeterministic);
    assert!(!result.deterministic_raw_observations);
    assert!(result.verification.is_none());
}

#[test]
fn failed_reset_receipt_rejects_the_binding() {
    let mut value = plan_value();
    value["binding"]["reset"]["expected_control"]["exit_code"] = serde_json::json!(1);
    let result = observe(&parse(&value)).expect("observation must execute");
    assert_eq!(result.status, ObservationStatus::BindingRejected);
    assert!(
        result
            .diagnostic
            .is_some_and(|diagnostic| diagnostic.contains("reset result"))
    );
    assert!(result.verification.is_none());
}

#[test]
fn observe_cli_emits_versioned_json() {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "kerf-observe-{}-{sequence}.json",
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("temporary plan path must be exclusively created");
    file.write_all(&serde_json::to_vec(&plan_value()).expect("plan serializes"))
        .expect("temporary plan must write");
    drop(file);
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-observe"))
        .arg(&path)
        .output()
        .expect("kerf-observe must run");
    std::fs::remove_file(&path).expect("temporary plan must be removable");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout must be JSON");
    assert_eq!(envelope["schema_version"], "kerf.observe-result.v1");
    assert_eq!(envelope["result"]["status"], "conformant");
    assert!(Path::new("/usr/bin/printf").is_absolute());
}
