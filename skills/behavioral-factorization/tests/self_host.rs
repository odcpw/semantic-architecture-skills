use kerf::report::sha256_hex;
use kerf::self_host::{CaseRole, SelfHostInput, check};
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn current_head() -> String {
    let output = Command::new("git")
        .args(["-C", env!("CARGO_MANIFEST_DIR"), "rev-parse", "HEAD"])
        .output()
        .expect("git must run");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("Git HEAD must be UTF-8")
        .trim()
        .to_owned()
}

fn corpus() -> (Vec<u8>, SelfHostInput) {
    let bytes = fs::read(repo_path("self-host/corpus.json")).expect("corpus must read");
    let input = serde_json::from_slice(&bytes).expect("corpus must be valid JSON");
    (bytes, input)
}

fn read_json(relative: &str) -> Value {
    serde_json::from_slice(&fs::read(repo_path(relative)).expect("fixture must read"))
        .expect("fixture must be valid JSON")
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2));
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex must be UTF-8");
            u8::from_str_radix(pair, 16).expect("hex must decode")
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut encoded, byte| {
            write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
            encoded
        },
    )
}

#[test]
fn exact_corpus_invokes_real_kerf() {
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-selfhost"))
        .args([
            "--kerf",
            env!("CARGO_BIN_EXE_kerf"),
            "--repo-root",
            env!("CARGO_MANIFEST_DIR"),
            "--expect-head",
            &current_head(),
            "--corpus",
        ])
        .arg(repo_path("self-host/corpus.json"))
        .arg("--allow-dirty")
        .output()
        .expect("self-host adapter must run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout must be JSON");
    assert_eq!(envelope["schema_version"], "kerf.self-host-result.v1");
    assert_eq!(envelope["result"]["matched"], true);
    assert_eq!(envelope["result"]["case_count"], 8);
    assert_eq!(envelope["result"]["expected_git_head"], current_head());
    assert_eq!(envelope["result"]["observed_git_head"], current_head());
    assert_eq!(
        envelope["result"]["executable_sha256"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    assert_eq!(
        envelope["result"]["corpus_sha256"].as_str().map(str::len),
        Some(64)
    );

    let cases = envelope["result"]["cases"]
        .as_array()
        .expect("cases must be an array");
    let distinguishing = cases
        .iter()
        .find(|case| case["role"] == "distinguishing_witness")
        .expect("distinguishing role must exist");
    let distinguishing_envelope: Value = serde_json::from_slice(&decode_hex(
        distinguishing["actual"]["stdout_hex"]
            .as_str()
            .expect("stdout hex must be a string"),
    ))
    .expect("distinguishing stdout must be JSON");
    assert!(
        distinguishing_envelope["result"]["witness"]
            .as_object()
            .is_some()
    );

    let typed_error = cases
        .iter()
        .find(|case| case["role"] == "typed_error")
        .expect("typed error role must exist");
    let resource_unknown = cases
        .iter()
        .find(|case| case["role"] == "resource_unknown")
        .expect("resource unknown role must exist");
    assert_eq!(typed_error["actual"]["exit_code"], 2);
    assert_eq!(resource_unknown["actual"]["exit_code"], 3);
    #[cfg(unix)]
    {
        assert_eq!(typed_error["actual"]["unix_raw_wait_status"], 512);
        assert_eq!(resource_unknown["actual"]["unix_raw_wait_status"], 768);
    }
}

#[test]
fn wrong_revision_fails_before_corpus_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-selfhost"))
        .args([
            "--kerf",
            env!("CARGO_BIN_EXE_kerf"),
            "--repo-root",
            env!("CARGO_MANIFEST_DIR"),
            "--expect-head",
            "0000000000000000000000000000000000000000",
            "--corpus",
        ])
        .arg(repo_path("self-host/corpus.json"))
        .arg("--allow-dirty")
        .output()
        .expect("self-host adapter must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr must be JSON");
    assert_eq!(error["schema_version"], "kerf.self-host-error.v1");
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("Git HEAD mismatch"))
    );
}

#[test]
fn valid_json_byte_drift_is_a_checked_mismatch() {
    let (bytes, mut input) = corpus();
    let successful = input
        .cases
        .iter_mut()
        .find(|case| case.role == CaseRole::SuccessfulLts)
        .expect("successful role must exist");
    successful.expected.stdout_hex.push_str("20");
    let result = check(
        Path::new(env!("CARGO_BIN_EXE_kerf")),
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &current_head(),
        false,
        &repo_path("self-host"),
        sha256_hex(&bytes),
        &input,
    )
    .expect("semantically valid drift must be checked");
    assert!(!result.matched);
    let successful = result
        .cases
        .iter()
        .find(|case| case.role == CaseRole::SuccessfulLts)
        .expect("successful result must exist");
    assert_eq!(successful.mismatch_fields, ["stdout"]);
}

#[test]
fn witness_role_rejects_a_verdict_without_a_witness() {
    let (bytes, mut input) = corpus();
    let distinguishing = input
        .cases
        .iter_mut()
        .find(|case| case.role == CaseRole::DistinguishingWitness)
        .expect("distinguishing role must exist");
    let mut envelope: Value =
        serde_json::from_slice(&decode_hex(&distinguishing.expected.stdout_hex))
            .expect("expected stdout must be JSON");
    envelope["result"]["witness"] = Value::Null;
    let mut serialized = serde_json::to_vec(&envelope).expect("envelope must serialize");
    serialized.push(b'\n');
    distinguishing.expected.stdout_hex = encode_hex(&serialized);

    let error = check(
        Path::new(env!("CARGO_BIN_EXE_kerf")),
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &current_head(),
        false,
        &repo_path("self-host"),
        sha256_hex(&bytes),
        &input,
    )
    .expect_err("a witness role without a witness must be invalid");
    assert_eq!(error.code, "invalid_input");
    assert!(error.message.contains("non-null result.witness"));
}

#[test]
fn controls_change_only_the_intended_semantic_coordinate() {
    let lawful = read_json("self-host/process.factor.json");
    let mut repaired_illegal = read_json("self-host/process-illegal.factor.json");
    repaired_illegal["factor_map"][3]["boundary_state"] = Value::from("witness");
    assert_eq!(repaired_illegal, lawful);

    let equivalent = read_json("self-host/process.recompose.json");
    let mut repaired_corrupt = read_json("self-host/process-corrupt.recompose.json");
    repaired_corrupt["emission_readouts"]["probe"][1]["output"] =
        Value::from("distinguishing-witness");
    assert_eq!(repaired_corrupt, equivalent);
    assert_eq!(lawful["machine"], equivalent["reference"]);
    assert_eq!(lawful["context"], equivalent["context"]);
}
