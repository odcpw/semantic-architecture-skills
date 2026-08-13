use crate::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub const SELF_HOST_SCHEMA: &str = "kerf.self-host-corpus.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfHostInput {
    pub schema_version: String,
    pub cases: Vec<ProcessCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessCase {
    pub id: String,
    pub role: CaseRole,
    pub operation: String,
    pub input: String,
    pub expected: ExpectedObservation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CaseRole {
    SuccessfulLts,
    TypedError,
    ResourceUnknown,
    DistinguishingWitness,
    LawfulFactor,
    UnlawfulFactor,
    EquivalentRecomposition,
    InequivalentRecomposition,
}

impl CaseRole {
    const ALL: [Self; 8] = [
        Self::SuccessfulLts,
        Self::TypedError,
        Self::ResourceUnknown,
        Self::DistinguishingWitness,
        Self::LawfulFactor,
        Self::UnlawfulFactor,
        Self::EquivalentRecomposition,
        Self::InequivalentRecomposition,
    ];

    const fn operation(self) -> &'static str {
        match self {
            Self::SuccessfulLts | Self::TypedError => "validate-lts",
            Self::ResourceUnknown | Self::DistinguishingWitness => "distinguish",
            Self::LawfulFactor | Self::UnlawfulFactor => "factor",
            Self::EquivalentRecomposition | Self::InequivalentRecomposition => "recompose",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedObservation {
    pub exit_code: Option<i32>,
    pub stdout_hex: String,
    pub stderr_hex: String,
}

#[derive(Debug, Serialize)]
pub struct SelfHostResult {
    pub algorithm: &'static str,
    pub matched: bool,
    pub expected_git_head: String,
    pub observed_git_head: String,
    pub git_dirty: bool,
    pub executable_sha256: String,
    pub corpus_sha256: String,
    pub case_count: usize,
    pub cases: Vec<ProcessCaseResult>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ProcessCaseResult {
    pub id: String,
    pub role: CaseRole,
    pub operation: String,
    pub input_sha256: String,
    pub matched: bool,
    pub mismatch_fields: Vec<&'static str>,
    pub actual: ProcessObservation,
}

#[derive(Debug, Serialize)]
pub struct ProcessObservation {
    pub exit_code: Option<i32>,
    pub success: bool,
    pub unix_raw_wait_status: Option<i32>,
    pub stdout_hex: String,
    pub stderr_hex: String,
}

/// Invoke an exact local `kerf` executable against a fixed process corpus.
///
/// # Errors
///
/// Returns [`ToolError`] when the corpus is malformed, a path escapes the
/// corpus root, or the executable or one of its inputs cannot be read or run.
pub fn check(
    executable: &Path,
    repository_root: &Path,
    expected_git_head: &str,
    require_clean: bool,
    corpus_root: &Path,
    corpus_sha256: String,
    input: &SelfHostInput,
) -> Result<SelfHostResult, ToolError> {
    if input.schema_version != SELF_HOST_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {SELF_HOST_SCHEMA:?}",
            input.schema_version
        )));
    }
    let revision = check_revision(repository_root, expected_git_head, require_clean)?;
    let executable = fs::canonicalize(executable).map_err(|error| {
        ToolError::io(format!(
            "could not resolve kerf executable {}: {error}",
            executable.display()
        ))
    })?;
    if !executable.is_file() {
        return Err(ToolError::invalid(format!(
            "kerf executable {} is not a regular file",
            executable.display()
        )));
    }
    let executable_bytes = fs::read(&executable).map_err(|error| {
        ToolError::io(format!(
            "could not read kerf executable {}: {error}",
            executable.display()
        ))
    })?;
    let corpus_root = fs::canonicalize(corpus_root).map_err(|error| {
        ToolError::io(format!(
            "could not resolve corpus root {}: {error}",
            corpus_root.display()
        ))
    })?;
    if !corpus_root.is_dir() {
        return Err(ToolError::invalid(format!(
            "corpus root {} is not a directory",
            corpus_root.display()
        )));
    }

    let ordered = validate_cases(input)?;
    let mut matched = true;
    let mut results = Vec::with_capacity(ordered.len());
    for case in ordered {
        let result = run_case(&executable, &corpus_root, case)?;
        matched &= result.matched;
        results.push(result);
    }

    Ok(SelfHostResult {
        algorithm: "exact_process_corpus_v1",
        matched,
        expected_git_head: expected_git_head.to_owned(),
        observed_git_head: revision.head,
        git_dirty: revision.dirty,
        executable_sha256: sha256_hex(&executable_bytes),
        corpus_sha256,
        case_count: results.len(),
        cases: results,
        nonclaims: vec![
            "The result checks one repository HEAD and binds one local executable hash, but it does not prove a reproducible source-to-binary derivation or source-code equivalence.",
            "The fixed corpus is not complete behavior coverage and does not infer a model, factor map, readout, or architecture.",
            "The adapter assumes the selected local executable terminates and does not sandbox it or bound its output.",
            "Environment clearing removes inherited variables but does not normalize the operating system, loader, filesystem, or hardware.",
        ],
    })
}

fn run_case(
    executable: &Path,
    corpus_root: &Path,
    case: &ProcessCase,
) -> Result<ProcessCaseResult, ToolError> {
    let input_path = resolve_case_input(corpus_root, &case.input)?;
    let input_bytes = fs::read(&input_path).map_err(|error| {
        ToolError::io(format!(
            "could not read corpus input {}: {error}",
            input_path.display()
        ))
    })?;
    let expected_stdout = decode_hex(&case.expected.stdout_hex, "expected.stdout_hex")?;
    let expected_stderr = decode_hex(&case.expected.stderr_hex, "expected.stderr_hex")?;
    validate_role_observation(
        case.role,
        case.expected.exit_code,
        &expected_stdout,
        &expected_stderr,
    )?;

    let output = Command::new(executable)
        .arg(&case.operation)
        .arg(&input_path)
        .current_dir(corpus_root)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            ToolError::io(format!(
                "could not run {} for case {:?}: {error}",
                executable.display(),
                case.id
            ))
        })?;
    let mut mismatch_fields = Vec::new();
    if output.status.code() != case.expected.exit_code {
        mismatch_fields.push("exit_code");
    }
    if output.stdout != expected_stdout {
        mismatch_fields.push("stdout");
    }
    if output.stderr != expected_stderr {
        mismatch_fields.push("stderr");
    }
    let matched = mismatch_fields.is_empty();
    #[cfg(unix)]
    let unix_raw_wait_status = {
        use std::os::unix::process::ExitStatusExt;
        Some(output.status.into_raw())
    };
    #[cfg(not(unix))]
    let unix_raw_wait_status = None;
    Ok(ProcessCaseResult {
        id: case.id.clone(),
        role: case.role,
        operation: case.operation.clone(),
        input_sha256: sha256_hex(&input_bytes),
        matched,
        mismatch_fields,
        actual: ProcessObservation {
            exit_code: output.status.code(),
            success: output.status.success(),
            unix_raw_wait_status,
            stdout_hex: encode_hex(&output.stdout),
            stderr_hex: encode_hex(&output.stderr),
        },
    })
}

struct RevisionObservation {
    head: String,
    dirty: bool,
}

fn check_revision(
    repository_root: &Path,
    expected_git_head: &str,
    require_clean: bool,
) -> Result<RevisionObservation, ToolError> {
    if expected_git_head.len() != 40
        || !expected_git_head
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ToolError::invalid(
            "expected Git HEAD must be exactly 40 lowercase hexadecimal digits",
        ));
    }
    let repository_root = fs::canonicalize(repository_root).map_err(|error| {
        ToolError::io(format!(
            "could not resolve repository root {}: {error}",
            repository_root.display()
        ))
    })?;
    let top_level = git_output(&repository_root, &["rev-parse", "--show-toplevel"])?;
    let top_level = fs::canonicalize(top_level.trim()).map_err(|error| {
        ToolError::io(format!(
            "could not resolve Git top level {top_level:?}: {error}"
        ))
    })?;
    if top_level != repository_root {
        return Err(ToolError::invalid(format!(
            "repository root {} is not the Git top level {}",
            repository_root.display(),
            top_level.display()
        )));
    }
    let head = git_output(&repository_root, &["rev-parse", "--verify", "HEAD"])?;
    let head = head.trim().to_owned();
    if head != expected_git_head {
        return Err(ToolError::invalid(format!(
            "Git HEAD mismatch: expected {expected_git_head}, observed {head}"
        )));
    }
    let status = git_output(
        &repository_root,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    let dirty = !status.is_empty();
    if require_clean && dirty {
        return Err(ToolError::invalid(
            "repository has tracked or untracked changes; exact revision binding requires a clean tree",
        ));
    }
    Ok(RevisionObservation { head, dirty })
}

fn git_output(repository_root: &Path, args: &[&str]) -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(repository_root)
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| ToolError::io(format!("could not run git: {error}")))?;
    if !output.status.success() {
        return Err(ToolError::invalid(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| ToolError::invalid(format!("git output was not UTF-8: {error}")))
}

fn validate_cases(input: &SelfHostInput) -> Result<Vec<&ProcessCase>, ToolError> {
    let mut ordered: Vec<&ProcessCase> = input.cases.iter().collect();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    if ordered.iter().any(|case| case.id.is_empty()) {
        return Err(ToolError::invalid("self-host case IDs must not be empty"));
    }
    if ordered.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(ToolError::invalid("self-host case IDs must be unique"));
    }
    let roles: BTreeSet<CaseRole> = ordered.iter().map(|case| case.role).collect();
    let expected_roles: BTreeSet<CaseRole> = CaseRole::ALL.into_iter().collect();
    if roles != expected_roles || ordered.len() != CaseRole::ALL.len() {
        return Err(ToolError::invalid(
            "self-host corpus must contain exactly one case for every v1 role",
        ));
    }
    for case in &ordered {
        if case.operation != case.role.operation() {
            return Err(ToolError::invalid(format!(
                "case {:?} with role {:?} must use operation {:?}",
                case.id,
                case.role,
                case.role.operation()
            )));
        }
    }
    Ok(ordered)
}

fn resolve_case_input(root: &Path, relative: &str) -> Result<PathBuf, ToolError> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ToolError::invalid(
            "self-host case inputs must be non-empty relative paths without traversal",
        ));
    }
    let resolved = fs::canonicalize(root.join(relative)).map_err(|error| {
        ToolError::io(format!(
            "could not resolve corpus input {}: {error}",
            relative.display()
        ))
    })?;
    if !resolved.starts_with(root) || !resolved.is_file() {
        return Err(ToolError::invalid(format!(
            "corpus input {} must resolve to a regular file inside the corpus root",
            relative.display()
        )));
    }
    Ok(resolved)
}

fn validate_role_observation(
    role: CaseRole,
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), ToolError> {
    let expectation = role_expectation(role);
    let (payload, other) = if expectation.result_on_stdout {
        (stdout, stderr)
    } else {
        (stderr, stdout)
    };
    if exit_code != Some(expectation.exit_code) {
        return Err(ToolError::invalid(format!(
            "role {role:?} must declare exit_code {}",
            expectation.exit_code
        )));
    }
    if !other.is_empty() {
        return Err(ToolError::invalid(format!(
            "role {role:?} must keep the non-result stream empty"
        )));
    }
    let value: Value = serde_json::from_slice(payload).map_err(|error| {
        ToolError::invalid(format!(
            "role {role:?} expected bytes must contain one JSON envelope: {error}"
        ))
    })?;
    if value.get("status").and_then(Value::as_str) != Some(expectation.status) {
        return Err(ToolError::invalid(format!(
            "role {role:?} expected envelope has the wrong status"
        )));
    }
    if value.get("operation").and_then(Value::as_str) != Some(role.operation()) {
        return Err(ToolError::invalid(format!(
            "role {role:?} expected envelope has the wrong operation"
        )));
    }
    if let Some((field, expected)) = expectation.result_field {
        let actual = json_path(&value, field);
        let valid = match expected {
            ExpectedField::Present => actual.is_some(),
            ExpectedField::Bool(expected) => actual.and_then(Value::as_bool) == Some(expected),
            ExpectedField::String(expected) => actual.and_then(Value::as_str) == Some(expected),
        };
        if !valid {
            return Err(ToolError::invalid(format!(
                "role {role:?} expected envelope does not establish {field}"
            )));
        }
    }
    if let Some(field) = expectation.required_non_null
        && json_path(&value, field).is_none_or(Value::is_null)
    {
        return Err(ToolError::invalid(format!(
            "role {role:?} expected envelope must contain a non-null {field}"
        )));
    }
    if expectation.witness_must_be_null
        && json_path(&value, "result.witness").is_some_and(|witness| !witness.is_null())
    {
        return Err(ToolError::invalid(
            "role EquivalentRecomposition expected envelope must not contain a witness",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ExpectedField {
    Present,
    Bool(bool),
    String(&'static str),
}

struct RoleExpectation {
    exit_code: i32,
    status: &'static str,
    result_on_stdout: bool,
    result_field: Option<(&'static str, ExpectedField)>,
    required_non_null: Option<&'static str>,
    witness_must_be_null: bool,
}

fn role_expectation(role: CaseRole) -> RoleExpectation {
    match role {
        CaseRole::SuccessfulLts => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.machine_states", ExpectedField::Present)),
            required_non_null: None,
            witness_must_be_null: false,
        },
        CaseRole::TypedError => RoleExpectation {
            exit_code: 2,
            status: "error",
            result_on_stdout: false,
            result_field: Some(("error.code", ExpectedField::String("invalid_input"))),
            required_non_null: None,
            witness_must_be_null: false,
        },
        CaseRole::ResourceUnknown => RoleExpectation {
            exit_code: 3,
            status: "unknown",
            result_on_stdout: false,
            result_field: Some(("error.code", ExpectedField::String("resource_limit"))),
            required_non_null: None,
            witness_must_be_null: false,
        },
        CaseRole::DistinguishingWitness => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.equivalent", ExpectedField::Bool(false))),
            required_non_null: Some("result.witness"),
            witness_must_be_null: false,
        },
        CaseRole::LawfulFactor => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.lawful", ExpectedField::Bool(true))),
            required_non_null: Some("result.representation"),
            witness_must_be_null: false,
        },
        CaseRole::UnlawfulFactor => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.lawful", ExpectedField::Bool(false))),
            required_non_null: Some("result.witness"),
            witness_must_be_null: false,
        },
        CaseRole::EquivalentRecomposition => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.verdict", ExpectedField::String("equivalent"))),
            required_non_null: None,
            witness_must_be_null: true,
        },
        CaseRole::InequivalentRecomposition => RoleExpectation {
            exit_code: 0,
            status: "ok",
            result_on_stdout: true,
            result_field: Some(("result.verdict", ExpectedField::String("inequivalent"))),
            required_non_null: Some("result.witness"),
            witness_must_be_null: false,
        },
    }
}

fn json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.get(segment))
}

fn decode_hex(value: &str, label: &str) -> Result<Vec<u8>, ToolError> {
    if !value.len().is_multiple_of(2) {
        return Err(ToolError::invalid(format!(
            "{label} must contain an even number of lowercase hexadecimal digits"
        )));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0]).ok_or_else(|| {
                ToolError::invalid(format!("{label} must contain lowercase hexadecimal digits"))
            })?;
            let low = hex_nibble(pair[1]).ok_or_else(|| {
                ToolError::invalid(format!("{label} must contain lowercase hexadecimal digits"))
            })?;
            Ok((high << 4) | low)
        })
        .collect()
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_codec_is_exact_and_lowercase_only() {
        assert_eq!(
            decode_hex("0001feff", "value").expect("hex must decode"),
            [0, 1, 254, 255]
        );
        assert_eq!(encode_hex(&[0, 1, 254, 255]), "0001feff");
        assert_eq!(
            decode_hex("F0", "value")
                .expect_err("uppercase must fail")
                .code,
            "invalid_input"
        );
        assert_eq!(
            decode_hex("0", "value")
                .expect_err("odd hex must fail")
                .code,
            "invalid_input"
        );
    }
}
