//! Reality-contact harness for producing finite trace certificates.

use kerf::cert::{
    CERTIFICATE_CHECK_SCHEMA, CertificateClaim, CertificateInput, CertificateResult, LtsSubject,
    TracePoint, TraceStep, trace_subject_sha256, verify,
};
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub const OBSERVATION_PLAN_SCHEMA: &str = "kerf.observation-plan.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationPlan {
    pub schema_version: String,
    pub subject: LtsSubject,
    pub binding: ProcessBinding,
    pub actions: Vec<String>,
    #[serde(default = "default_repetitions")]
    pub repetitions: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessBinding {
    pub id: String,
    pub working_directory: String,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    pub open: Invocation,
    pub reset: Invocation,
    pub initial_probe: Invocation,
    pub close: Invocation,
    pub action_invocations: Vec<ActionInvocation>,
    pub output_classes: Vec<OutputClass>,
    pub negative_controls: Vec<NegativeControlProbe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub program: String,
    pub program_sha256: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin_hex: String,
    #[serde(default)]
    pub expected_control: Option<RawExpectation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawExpectation {
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout_hex: String,
    #[serde(default)]
    pub stderr_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionInvocation {
    pub action: String,
    pub invocation: Invocation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputClass {
    pub id: String,
    pub exit_code: Option<i32>,
    pub stdout_hex: String,
    pub stderr_hex: String,
    pub observation: String,
    #[serde(default)]
    pub emission: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControlProbe {
    pub id: String,
    pub invocation: Invocation,
    pub expected_rejection: String,
}

#[derive(Debug, Serialize)]
pub struct ObservationResult {
    pub algorithm: &'static str,
    pub status: ObservationStatus,
    pub binding_sha256: String,
    pub subject_sha256: String,
    pub repetitions_requested: usize,
    pub repetitions_completed: usize,
    pub deterministic_raw_observations: bool,
    pub negative_controls_passed: bool,
    pub receipts: Vec<InvocationReceipt>,
    pub certificate: Option<CertificateInput>,
    pub verification: Option<CertificateResult>,
    pub diagnostic: Option<String>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Conformant,
    Contradicted,
    Nondeterministic,
    BindingRejected,
    NegativeControlFailed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InvocationReceipt {
    pub run: usize,
    pub phase: String,
    pub program: String,
    pub program_sha256: String,
    pub exit_code: Option<i32>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Clone, PartialEq, Eq)]
struct RawInvocation {
    receipt: InvocationReceipt,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct CaptureRun {
    raw: Vec<RawInvocation>,
    certificate: Result<CertificateInput, String>,
}

fn default_repetitions() -> usize {
    2
}

/// Execute a declared, environment-cleared observation plan and check the
/// resulting trace against its supplied finite model.
///
/// # Errors
///
/// Returns [`ToolError`] for an invalid plan, unreadable executable, or process
/// launch/I/O failure. Semantic disagreement and nondeterminism are completed
/// results, never errors or passes.
#[allow(clippy::too_many_lines)]
pub fn observe(plan: &ObservationPlan) -> Result<ObservationResult, ToolError> {
    validate_plan(plan)?;
    let subject_sha256 = trace_subject_sha256(&plan.subject)?;
    let binding_sha256 = canonical_binding_sha256(&plan.binding)?;
    let mut runs = Vec::new();
    for run in 0..plan.repetitions {
        runs.push(capture_run(plan, run)?);
    }
    let mut receipts: Vec<InvocationReceipt> = runs
        .iter()
        .flat_map(|run| run.raw.iter().map(|item| item.receipt.clone()))
        .collect();
    let deterministic = runs
        .windows(2)
        .all(|pair| raw_observations_equal(&pair[0].raw, &pair[1].raw));
    if !deterministic {
        return Ok(result(
            ObservationStatus::Nondeterministic,
            binding_sha256,
            subject_sha256,
            plan.repetitions,
            runs.len(),
            false,
            false,
            receipts,
            None,
            None,
            Some("repeated runs produced different raw exit/stdout/stderr observations".to_owned()),
        ));
    }
    let certificate = match &runs[0].certificate {
        Ok(certificate) => certificate.clone(),
        Err(diagnostic) => {
            return Ok(result(
                ObservationStatus::BindingRejected,
                binding_sha256,
                subject_sha256,
                plan.repetitions,
                runs.len(),
                true,
                false,
                receipts,
                None,
                None,
                Some(diagnostic.clone()),
            ));
        }
    };
    let (negative_controls_passed, mut control_receipts, control_failure) =
        run_negative_controls(plan, runs.len())?;
    receipts.append(&mut control_receipts);
    if !negative_controls_passed {
        return Ok(result(
            ObservationStatus::NegativeControlFailed,
            binding_sha256,
            subject_sha256,
            plan.repetitions,
            runs.len(),
            true,
            false,
            receipts,
            Some(certificate),
            None,
            control_failure,
        ));
    }
    let verification = verify(&certificate)?;
    let status = if verification.accepted {
        ObservationStatus::Conformant
    } else {
        ObservationStatus::Contradicted
    };
    Ok(result(
        status,
        binding_sha256,
        subject_sha256,
        plan.repetitions,
        runs.len(),
        true,
        true,
        receipts,
        Some(certificate),
        Some(verification),
        None,
    ))
}

#[allow(clippy::too_many_arguments)]
fn result(
    status: ObservationStatus,
    binding_sha256: String,
    subject_sha256: String,
    repetitions_requested: usize,
    repetitions_completed: usize,
    deterministic_raw_observations: bool,
    negative_controls_passed: bool,
    receipts: Vec<InvocationReceipt>,
    certificate: Option<CertificateInput>,
    verification: Option<CertificateResult>,
    diagnostic: Option<String>,
) -> ObservationResult {
    ObservationResult {
        algorithm: "environment_cleared_trace_capture_v1",
        status,
        binding_sha256,
        subject_sha256,
        repetitions_requested,
        repetitions_completed,
        deterministic_raw_observations,
        negative_controls_passed,
        receipts,
        certificate,
        verification,
        diagnostic,
        nonclaims: vec![
            "A conformant result covers only the declared actions, repetitions, binding, environment, and finite subject.",
            "A rejected trace indicts the model-or-binding pair; negative controls reduce but do not eliminate binding error.",
            "The harness clears inherited environment variables but does not sandbox commands, bound runtime/output, or normalize the operating system, filesystem, loader, or hardware.",
            "Path hashing immediately before spawn narrows but does not eliminate an executable-replacement race on a concurrently writable filesystem.",
            "Repeated deterministic observations do not prove the target is deterministic outside this plan.",
            "Model state is reconstructed from the declared transition system; only decoded observations and emissions come from command output.",
            "The trace certificate alone proves model-internal replay; reality contact belongs to this complete observation envelope and its binding receipts.",
        ],
    }
}

#[allow(clippy::too_many_lines)]
fn validate_plan(plan: &ObservationPlan) -> Result<(), ToolError> {
    if plan.schema_version != OBSERVATION_PLAN_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {:?}",
            plan.schema_version, OBSERVATION_PLAN_SCHEMA
        )));
    }
    if plan.binding.id.is_empty() {
        return Err(ToolError::invalid("binding.id must not be empty"));
    }
    if plan.repetitions < 2 {
        return Err(ToolError::invalid(
            "repetitions must be at least 2 to detect observed nondeterminism",
        ));
    }
    let working_directory = Path::new(&plan.binding.working_directory);
    if !working_directory.is_absolute() || !working_directory.is_dir() {
        return Err(ToolError::invalid(
            "binding.working_directory must be an existing absolute directory",
        ));
    }
    for (phase, invocation) in [
        ("open", &plan.binding.open),
        ("reset", &plan.binding.reset),
        ("initial_probe", &plan.binding.initial_probe),
        ("close", &plan.binding.close),
    ] {
        validate_invocation(phase, invocation)?;
    }
    for (phase, invocation) in [
        ("open", &plan.binding.open),
        ("reset", &plan.binding.reset),
        ("close", &plan.binding.close),
    ] {
        if invocation.expected_control.is_none() {
            return Err(ToolError::invalid(format!(
                "{phase}.expected_control must declare the lifecycle command receipt"
            )));
        }
    }
    let action_set: BTreeSet<&str> = plan
        .subject
        .machine
        .actions
        .iter()
        .map(String::as_str)
        .collect();
    let mut action_invocations = BTreeSet::new();
    for mapping in &plan.binding.action_invocations {
        if !action_set.contains(mapping.action.as_str()) {
            return Err(ToolError::invalid(format!(
                "action invocation references unknown model action {:?}",
                mapping.action
            )));
        }
        if !action_invocations.insert(mapping.action.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate action invocation {:?}",
                mapping.action
            )));
        }
        validate_invocation("action", &mapping.invocation)?;
    }
    for action in &plan.actions {
        if !action_invocations.contains(action.as_str()) {
            return Err(ToolError::invalid(format!(
                "trace action {action:?} has no invocation binding"
            )));
        }
    }
    if plan.binding.output_classes.is_empty() {
        return Err(ToolError::invalid("output_classes must not be empty"));
    }
    let mut class_ids = BTreeSet::new();
    let mut signatures = BTreeSet::new();
    for class in &plan.binding.output_classes {
        if class.id.is_empty() || class.observation.is_empty() {
            return Err(ToolError::invalid(
                "output class ID and observation must not be empty",
            ));
        }
        if !class_ids.insert(class.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate output class {:?}",
                class.id
            )));
        }
        let stdout = decode_hex(&class.stdout_hex, "output_class.stdout_hex")?;
        let stderr = decode_hex(&class.stderr_hex, "output_class.stderr_hex")?;
        if !signatures.insert((class.exit_code, stdout, stderr)) {
            return Err(ToolError::invalid(
                "output classes must have distinct raw exit/stdout/stderr signatures",
            ));
        }
    }
    if plan.binding.negative_controls.is_empty() {
        return Err(ToolError::invalid(
            "negative_controls must contain at least one nearby rejected probe",
        ));
    }
    let mut controls = BTreeSet::new();
    for control in &plan.binding.negative_controls {
        if control.id.is_empty() || control.expected_rejection.is_empty() {
            return Err(ToolError::invalid(
                "negative control ID and expected_rejection must not be empty",
            ));
        }
        if !controls.insert(control.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate negative control {:?}",
                control.id
            )));
        }
        validate_invocation("negative_control", &control.invocation)?;
    }
    trace_subject_sha256(&plan.subject)?;
    Ok(())
}

fn validate_invocation(field: &str, invocation: &Invocation) -> Result<(), ToolError> {
    let program = Path::new(&invocation.program);
    if !program.is_absolute() || !program.is_file() {
        return Err(ToolError::invalid(format!(
            "{field}.program must be an existing absolute file"
        )));
    }
    require_sha256(
        &format!("{field}.program_sha256"),
        &invocation.program_sha256,
    )?;
    let actual_sha256 = sha256_hex(&fs::read(program).map_err(|error| {
        ToolError::io(format!("could not read {}: {error}", invocation.program))
    })?);
    if actual_sha256 != invocation.program_sha256 {
        return Err(ToolError::invalid(format!(
            "{field}.program_sha256 does not match executable {:?}",
            invocation.program
        )));
    }
    decode_hex(&invocation.stdin_hex, &format!("{field}.stdin_hex"))?;
    if let Some(expected) = &invocation.expected_control {
        decode_hex(
            &expected.stdout_hex,
            &format!("{field}.expected_control.stdout_hex"),
        )?;
        decode_hex(
            &expected.stderr_hex,
            &format!("{field}.expected_control.stderr_hex"),
        )?;
    }
    Ok(())
}

fn capture_run(plan: &ObservationPlan, run: usize) -> Result<CaptureRun, ToolError> {
    let mut raw = Vec::new();
    raw.push(run_invocation(plan, run, "open", &plan.binding.open)?);
    raw.push(run_invocation(plan, run, "reset", &plan.binding.reset)?);
    raw.push(run_invocation(
        plan,
        run,
        "initial_probe",
        &plan.binding.initial_probe,
    )?);
    for (index, action) in plan.actions.iter().enumerate() {
        let mapping = plan
            .binding
            .action_invocations
            .iter()
            .find(|mapping| &mapping.action == action)
            .expect("validated action binding exists");
        raw.push(run_invocation(
            plan,
            run,
            &format!("action:{index}:{action}"),
            &mapping.invocation,
        )?);
    }
    raw.push(run_invocation(plan, run, "close", &plan.binding.close)?);
    let certificate = certificate_from_raw(plan, &raw);
    Ok(CaptureRun { raw, certificate })
}

fn certificate_from_raw(
    plan: &ObservationPlan,
    raw: &[RawInvocation],
) -> Result<CertificateInput, String> {
    for (phase, invocation, observation) in [
        ("open", &plan.binding.open, &raw[0]),
        ("reset", &plan.binding.reset, &raw[1]),
        (
            "close",
            &plan.binding.close,
            raw.last().expect("capture always includes close"),
        ),
    ] {
        if !matches_control(invocation, observation) {
            return Err(format!(
                "{phase} result did not match its declared expected_control receipt"
            ));
        }
    }
    let initial_class = classify(plan, &raw[2])
        .ok_or_else(|| "initial probe output matched no declared output class".to_owned())?;
    let initial = TracePoint {
        state: plan.subject.machine.initial.clone(),
        context: plan
            .subject
            .context
            .as_ref()
            .map(|context| context.initial.clone()),
        observation: initial_class.observation.clone(),
    };
    let mut state = plan.subject.machine.initial.as_str();
    let mut context = plan
        .subject
        .context
        .as_ref()
        .map(|item| item.initial.as_str());
    let mut steps = Vec::new();
    for (index, action) in plan.actions.iter().enumerate() {
        let class = classify(plan, &raw[index + 3])
            .ok_or_else(|| format!("action {action:?} output matched no declared output class"))?;
        let emission = class.emission.clone().ok_or_else(|| {
            format!(
                "output class {:?} has no emission for action {action:?}",
                class.id
            )
        })?;
        let source = plan
            .subject
            .machine
            .states
            .iter()
            .find(|item| item.id == state)
            .ok_or_else(|| format!("model state {state:?} is absent"))?;
        let transition = source
            .transitions
            .get(action)
            .ok_or_else(|| format!("model action {action:?} is absent from state {state:?}"))?;
        let next_context = match (&plan.subject.context, context) {
            (Some(monitor), Some(current)) => {
                let context_state = monitor
                    .states
                    .iter()
                    .find(|item| item.id == current)
                    .ok_or_else(|| format!("context state {current:?} is absent"))?;
                Some(
                    context_state
                        .transitions
                        .get(action)
                        .ok_or_else(|| {
                            format!("action {action:?} is disabled in context {current:?}")
                        })?
                        .as_str(),
                )
            }
            (None, None) => None,
            _ => return Err("context binding is internally inconsistent".to_owned()),
        };
        steps.push(TraceStep {
            action: action.clone(),
            emission,
            to: TracePoint {
                state: transition.to.clone(),
                context: next_context.map(str::to_owned),
                observation: class.observation.clone(),
            },
        });
        state = &transition.to;
        context = next_context;
    }
    let subject_sha256 = trace_subject_sha256(&plan.subject).map_err(|error| error.to_string())?;
    Ok(CertificateInput {
        schema_version: CERTIFICATE_CHECK_SCHEMA.to_owned(),
        claim: CertificateClaim::LtsTrace {
            subject: plan.subject.clone(),
            subject_sha256,
            initial,
            steps,
            provenance: None,
        },
        expected_provenance: None,
    })
}

fn classify<'a>(plan: &'a ObservationPlan, raw: &RawInvocation) -> Option<&'a OutputClass> {
    plan.binding.output_classes.iter().find(|class| {
        class.exit_code == raw.receipt.exit_code
            && decode_hex(&class.stdout_hex, "stdout_hex").is_ok_and(|value| value == raw.stdout)
            && decode_hex(&class.stderr_hex, "stderr_hex").is_ok_and(|value| value == raw.stderr)
    })
}

fn run_negative_controls(
    plan: &ObservationPlan,
    run_offset: usize,
) -> Result<(bool, Vec<InvocationReceipt>, Option<String>), ToolError> {
    let mut receipts = Vec::new();
    for (index, control) in plan.binding.negative_controls.iter().enumerate() {
        let run = run_offset + index;
        let open = run_invocation(
            plan,
            run,
            &format!("control:{}:open", control.id),
            &plan.binding.open,
        )?;
        let reset = run_invocation(
            plan,
            run,
            &format!("control:{}:reset", control.id),
            &plan.binding.reset,
        )?;
        let probe = run_invocation(
            plan,
            run,
            &format!("control:{}:probe", control.id),
            &control.invocation,
        )?;
        let close = run_invocation(
            plan,
            run,
            &format!("control:{}:close", control.id),
            &plan.binding.close,
        )?;
        let lifecycle_failure = [
            ("open", &plan.binding.open, &open),
            ("reset", &plan.binding.reset, &reset),
            ("close", &plan.binding.close, &close),
        ]
        .into_iter()
        .find(|(_, invocation, observation)| !matches_control(invocation, observation))
        .map(|(phase, _, _)| phase);
        let matched = classify(plan, &probe).map(|class| class.id.clone());
        receipts.extend([open.receipt, reset.receipt, probe.receipt, close.receipt]);
        if let Some(phase) = lifecycle_failure {
            return Ok((
                false,
                receipts,
                Some(format!(
                    "negative control {:?} {phase} result did not match its declared expected_control receipt",
                    control.id
                )),
            ));
        }
        if let Some(class) = matched {
            return Ok((
                false,
                receipts,
                Some(format!(
                    "negative control {:?} unexpectedly matched output class {:?}; expected {}",
                    control.id, class, control.expected_rejection
                )),
            ));
        }
    }
    Ok((true, receipts, None))
}

fn matches_control(invocation: &Invocation, raw: &RawInvocation) -> bool {
    invocation
        .expected_control
        .as_ref()
        .is_some_and(|expected| {
            expected.exit_code == raw.receipt.exit_code
                && decode_hex(&expected.stdout_hex, "expected_control.stdout_hex")
                    .is_ok_and(|value| value == raw.stdout)
                && decode_hex(&expected.stderr_hex, "expected_control.stderr_hex")
                    .is_ok_and(|value| value == raw.stderr)
        })
}

fn run_invocation(
    plan: &ObservationPlan,
    run: usize,
    phase: &str,
    invocation: &Invocation,
) -> Result<RawInvocation, ToolError> {
    let program_bytes = fs::read(&invocation.program).map_err(|error| {
        ToolError::io(format!(
            "could not read executable {}: {error}",
            invocation.program
        ))
    })?;
    let program_sha256 = sha256_hex(&program_bytes);
    if program_sha256 != invocation.program_sha256 {
        return Err(ToolError::invalid(format!(
            "executable {:?} changed after plan validation",
            invocation.program
        )));
    }
    let stdin = decode_hex(&invocation.stdin_hex, "invocation.stdin_hex")?;
    let mut child = Command::new(&invocation.program) // ubs:ignore — reviewed-plan command; absolute path and SHA-256 revalidated immediately before spawn; no shell
        .args(&invocation.args)
        .current_dir(&plan.binding.working_directory)
        .env_clear()
        .envs(&plan.binding.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            ToolError::io(format!("could not launch {}: {error}", invocation.program))
        })?;
    if !stdin.is_empty() {
        child
            .stdin
            .as_mut()
            .ok_or_else(|| ToolError::internal("child stdin was not piped"))?
            .write_all(&stdin)
            .map_err(|error| ToolError::io(format!("could not write child stdin: {error}")))?;
    }
    let output = child.wait_with_output().map_err(|error| {
        ToolError::io(format!(
            "could not collect {} output: {error}",
            invocation.program
        ))
    })?;
    let receipt = InvocationReceipt {
        run,
        phase: phase.to_owned(),
        program: invocation.program.clone(),
        program_sha256,
        exit_code: output.status.code(),
        stdout_sha256: sha256_hex(&output.stdout),
        stderr_sha256: sha256_hex(&output.stderr),
    };
    Ok(RawInvocation {
        receipt,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn raw_observations_equal(left: &[RawInvocation], right: &[RawInvocation]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.receipt.phase == right.receipt.phase
                && left.receipt.program_sha256 == right.receipt.program_sha256
                && left.receipt.exit_code == right.receipt.exit_code
                && left.stdout == right.stdout
                && left.stderr == right.stderr
        })
}

fn canonical_binding_sha256(binding: &ProcessBinding) -> Result<String, ToolError> {
    let mut normalized = binding.clone();
    normalized
        .action_invocations
        .sort_by(|left, right| left.action.cmp(&right.action));
    normalized
        .output_classes
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .negative_controls
        .sort_by(|left, right| left.id.cmp(&right.id));
    let bytes = serde_json::to_vec(&normalized).map_err(|error| {
        ToolError::internal(format!("could not serialize observation binding: {error}"))
    })?;
    Ok(sha256_hex(&bytes))
}

fn decode_hex(value: &str, field: &str) -> Result<Vec<u8>, ToolError> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ToolError::invalid(format!(
            "{field} must contain an even number of hexadecimal characters"
        )));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("ASCII hex is UTF-8");
            u8::from_str_radix(text, 16)
                .map_err(|error| ToolError::invalid(format!("invalid {field}: {error}")))
        })
        .collect()
}

fn require_sha256(field: &str, value: &str) -> Result<(), ToolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ToolError::invalid(format!(
            "{field} must be 64 lowercase hexadecimal characters"
        )));
    }
    Ok(())
}
