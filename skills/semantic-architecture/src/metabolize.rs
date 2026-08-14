//! Isolated apply, verify, measure, and semantic-model rebuild for generated changes.

use crate::generate::{
    GeneratorSubject, PredictedDirection, TransformationBatch, TransformationCandidate,
    TransformationRequest, validate_transformation_batch,
};
use crate::semantic::{SemanticModel, SubjectKind, analyze, parse_no_duplicates};
use kerf::candidates::Direction;
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub const METABOLISM_PLAN_SCHEMA: &str = "kerf.metabolism-plan.v1";
pub const METABOLISM_RECEIPT_SCHEMA: &str = "kerf.metabolism-receipt.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundProgram {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundExternalFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundCommand {
    pub program: BoundProgram,
    #[serde(default)]
    pub external_files: Vec<BoundExternalFile>,
    #[serde(default)]
    pub arguments: Vec<String>,
    pub cwd: String,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundSourceFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiteralReplacement {
    pub old: String,
    pub new: String,
    pub expected_occurrences: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FileMutation {
    ReplaceText {
        path: String,
        preimage_sha256: String,
        replacements: Vec<LiteralReplacement>,
    },
    CreateText {
        path: String,
        content: String,
        postimage_sha256: String,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Success,
    StdoutParity,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCheck {
    pub id: String,
    pub kind: CheckKind,
    pub command: BoundCommand,
    #[serde(default)]
    pub protected_observation_ids: Vec<String>,
    #[serde(default)]
    pub hard_constraint_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControl {
    pub id: String,
    pub command: BoundCommand,
    pub expected_exit_code: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricProbe {
    pub metric_id: String,
    pub command: BoundCommand,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRebuild {
    pub command: BoundCommand,
    pub model_path: String,
    pub implementation_subject_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IsolationPolicy {
    pub workspace_root: String,
    pub copy_paths: Vec<String>,
    #[serde(default)]
    pub excluded_names: Vec<String>,
    pub max_files: usize,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetabolismPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub candidate_id: String,
    pub source_revision: String,
    pub source_manifest: Vec<BoundSourceFile>,
    pub isolation: IsolationPolicy,
    pub mutations: Vec<FileMutation>,
    pub checks: Vec<VerificationCheck>,
    pub negative_controls: Vec<NegativeControl>,
    pub metrics: Vec<MetricProbe>,
    #[serde(default)]
    pub forbidden_regression_metrics: Vec<String>,
    pub model_rebuild: ModelRebuild,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetabolismStatus {
    Accepted,
    Rejected,
    BindingRejected,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedDirection {
    Better,
    Worse,
    Unchanged,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MutationReceipt {
    pub path: String,
    pub preimage_sha256: Option<String>,
    pub postimage_sha256: String,
    pub changed: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandReceipt {
    pub id: String,
    pub kind: String,
    pub baseline_exit_code: i32,
    pub candidate_exit_code: i32,
    pub baseline_stdout_sha256: String,
    pub candidate_stdout_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricReceipt {
    pub metric_id: String,
    pub baseline: i128,
    pub candidate: i128,
    pub observed_direction: ObservedDirection,
    pub predicted_direction: PredictedDirection,
    pub prediction_matched: Option<bool>,
    pub forbidden_regression: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelReceipt {
    pub path: String,
    pub sha256: String,
    pub model_id: String,
    pub model_revision: String,
    pub structurally_valid: bool,
    pub blocker_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetabolismReceipt {
    pub schema_version: &'static str,
    pub plan_id: String,
    pub plan_sha256: String,
    pub request_sha256: String,
    pub batch_sha256: String,
    pub candidate_id: String,
    pub candidate_workspace: String,
    pub baseline_revision: String,
    pub candidate_revision: Option<String>,
    pub status: MetabolismStatus,
    pub accepted: bool,
    pub mutation_receipts: Vec<MutationReceipt>,
    pub command_receipts: Vec<CommandReceipt>,
    pub metric_receipts: Vec<MetricReceipt>,
    pub model_receipt: Option<ModelReceipt>,
    pub counterexamples: Vec<String>,
    pub nonclaims: Vec<String>,
}

struct RunOutput {
    code: i32,
    stdout: Vec<u8>,
}

/// Execute one generated transformation in fresh baseline/candidate copies.
///
/// Completed refutations are receipts, not errors. Errors are reserved for
/// malformed plans and operating-system failures that prevent observation.
///
/// # Errors
///
/// Returns [`ToolError`] when inputs are invalid, content bindings disagree,
/// the isolation root already exists, or an invocation cannot be executed.
#[allow(clippy::too_many_lines)]
pub fn execute_metabolism(
    request: &TransformationRequest,
    batch: &TransformationBatch,
    plan: &MetabolismPlan,
) -> Result<MetabolismReceipt, ToolError> {
    validate_transformation_batch(request, batch)?;
    let candidate = batch
        .candidates
        .iter()
        .find(|item| item.id == plan.candidate_id)
        .ok_or_else(|| ToolError::invalid("metabolism plan references an unknown candidate"))?;
    validate_plan(request, plan, candidate)?;
    let request_sha256 = canonical_sha256(request, "transformation request")?;
    let batch_sha256 = canonical_sha256(batch, "transformation batch")?;
    let plan_sha256 = canonical_sha256(plan, "metabolism plan")?;
    let source_root = source_root(request)?;
    verify_source_manifest(source_root, &plan.source_manifest)?;
    verify_programs(plan)?;

    let workspace = Path::new(&plan.isolation.workspace_root);
    prepare_workspace(source_root, workspace, &plan.isolation)?;
    let baseline_root = workspace.join("baseline");
    let candidate_root = workspace.join("candidate");
    let mut receipt = MetabolismReceipt {
        schema_version: METABOLISM_RECEIPT_SCHEMA,
        plan_id: plan.plan_id.clone(),
        plan_sha256,
        request_sha256,
        batch_sha256,
        candidate_id: candidate.id.clone(),
        candidate_workspace: candidate_root.display().to_string(),
        baseline_revision: plan.source_revision.clone(),
        candidate_revision: None,
        status: MetabolismStatus::Rejected,
        accepted: false,
        mutation_receipts: Vec::new(),
        command_receipts: Vec::new(),
        metric_receipts: Vec::new(),
        model_receipt: None,
        counterexamples: Vec::new(),
        nonclaims: plan.nonclaims.clone(),
    };

    for mutation in &plan.mutations {
        receipt
            .mutation_receipts
            .push(apply_mutation(&candidate_root, mutation)?);
    }
    if receipt.mutation_receipts.iter().any(|item| !item.changed) {
        receipt
            .counterexamples
            .push("at least one declared mutation left its source bytes unchanged".to_owned());
    }
    let candidate_revision =
        source_revision(&candidate_root, &plan.source_manifest, &plan.mutations)?;
    receipt.candidate_revision = Some(candidate_revision.clone());
    if candidate_revision == plan.source_revision {
        receipt
            .counterexamples
            .push("candidate source identity equals the baseline identity".to_owned());
    }

    for check in &plan.checks {
        let baseline = run_command(&baseline_root, &check.command, None)?;
        let candidate_output = run_command(&candidate_root, &check.command, None)?;
        let passed = baseline.code == 0
            && candidate_output.code == 0
            && (check.kind == CheckKind::Success || baseline.stdout == candidate_output.stdout);
        if !passed {
            receipt.counterexamples.push(format!(
                "verification check {:?} did not preserve its declared contract",
                check.id
            ));
        }
        receipt.command_receipts.push(command_receipt(
            &check.id,
            match check.kind {
                CheckKind::Success => "verification_success",
                CheckKind::StdoutParity => "protected_stdout_parity",
            },
            &baseline,
            &candidate_output,
            passed,
        ));
    }
    for control in &plan.negative_controls {
        let baseline = run_command(&baseline_root, &control.command, None)?;
        let candidate_output = run_command(&candidate_root, &control.command, None)?;
        let passed = baseline.code == control.expected_exit_code
            && candidate_output.code == control.expected_exit_code;
        if !passed {
            receipt.counterexamples.push(format!(
                "negative control {:?} did not discriminate as declared",
                control.id
            ));
        }
        receipt.command_receipts.push(command_receipt(
            &control.id,
            "negative_control",
            &baseline,
            &candidate_output,
            passed,
        ));
    }

    let metric_intents = request
        .metrics
        .iter()
        .map(|metric| (metric.id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    let forbidden = plan
        .forbidden_regression_metrics
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for probe in &plan.metrics {
        let baseline = run_command(&baseline_root, &probe.command, None)?;
        let candidate_output = run_command(&candidate_root, &probe.command, None)?;
        let baseline_value = parse_metric(&probe.metric_id, &baseline)?;
        let candidate_value = parse_metric(&probe.metric_id, &candidate_output)?;
        let intent = metric_intents
            .get(probe.metric_id.as_str())
            .ok_or_else(|| {
                ToolError::internal("validated metric probe was absent from the purpose envelope")
            })?;
        let observed = observed_direction(intent.direction, baseline_value, candidate_value);
        let prediction = candidate
            .predicted_metric_deltas
            .get(&probe.metric_id)
            .ok_or_else(|| {
                ToolError::internal("validated candidate was missing a metric prediction")
            })?
            .direction;
        let prediction_matched = prediction_match(prediction, observed);
        let forbidden_regression =
            observed == ObservedDirection::Worse && forbidden.contains(probe.metric_id.as_str());
        if forbidden_regression {
            receipt.counterexamples.push(format!(
                "metric {:?} regressed from {baseline_value} to {candidate_value}",
                probe.metric_id
            ));
        }
        receipt.metric_receipts.push(MetricReceipt {
            metric_id: probe.metric_id.clone(),
            baseline: baseline_value,
            candidate: candidate_value,
            observed_direction: observed,
            predicted_direction: prediction,
            prediction_matched,
            forbidden_regression,
        });
    }
    if !receipt
        .metric_receipts
        .iter()
        .any(|item| item.observed_direction == ObservedDirection::Better)
    {
        receipt
            .counterexamples
            .push("no declared metric improved".to_owned());
    }

    let rebuild_output = run_command(
        &candidate_root,
        &plan.model_rebuild.command,
        Some((&candidate_revision, &candidate.id)),
    )?;
    if rebuild_output.code != 0 {
        receipt.counterexamples.push(format!(
            "semantic model rebuild exited with code {}",
            rebuild_output.code
        ));
    } else {
        let model_path = safe_join(&candidate_root, &plan.model_rebuild.model_path)?;
        let bytes = fs::read(&model_path).map_err(|error| {
            ToolError::io(format!(
                "could not read rebuilt model {}: {error}",
                model_path.display()
            ))
        })?;
        let model: SemanticModel = parse_no_duplicates(&bytes)?;
        let analysis = analyze(&model)?;
        let model_subject = model.subjects.iter().find(|subject| {
            subject.id == plan.model_rebuild.implementation_subject_id
                && subject.kind == SubjectKind::Implementation
        });
        let model_revision = model_subject
            .and_then(|subject| subject.revision.clone())
            .unwrap_or_default();
        if model_revision != candidate_revision {
            receipt.counterexamples.push(format!(
                "rebuilt model names revision {model_revision:?}, expected {candidate_revision:?}"
            ));
        }
        let blockers = analysis.finding_counts.get("blocker").copied().unwrap_or(0);
        let errors = analysis.finding_counts.get("error").copied().unwrap_or(0);
        receipt.model_receipt = Some(ModelReceipt {
            path: model_path.display().to_string(),
            sha256: sha256_hex(&bytes),
            model_id: model.model_id,
            model_revision,
            structurally_valid: analysis.structurally_valid,
            blocker_count: blockers,
            error_count: errors,
        });
    }

    if receipt.counterexamples.is_empty()
        && receipt.command_receipts.iter().all(|item| item.passed)
        && receipt.mutation_receipts.iter().all(|item| item.changed)
        && receipt.model_receipt.as_ref().is_some_and(|model| {
            model.structurally_valid && model.model_revision == candidate_revision
        })
    {
        receipt.status = MetabolismStatus::Accepted;
        receipt.accepted = true;
    }
    Ok(receipt)
}

#[allow(clippy::too_many_lines)]
fn validate_plan(
    request: &TransformationRequest,
    plan: &MetabolismPlan,
    candidate: &TransformationCandidate,
) -> Result<(), ToolError> {
    if plan.schema_version != METABOLISM_PLAN_SCHEMA {
        return Err(ToolError::invalid("unsupported metabolism plan schema"));
    }
    if plan.plan_id.is_empty() || plan.nonclaims.is_empty() {
        return Err(ToolError::invalid(
            "plan_id and nonclaims must not be empty",
        ));
    }
    let GeneratorSubject::Brownfield { revision, .. } = &request.subject else {
        return Err(ToolError::invalid(
            "metabolism execution currently requires a brownfield source subject",
        ));
    };
    if plan.source_revision != *revision
        || candidate.parent_search_basis_sha256 != request.search_basis_sha256
    {
        return Err(ToolError::invalid(
            "plan source/search identities do not match the request",
        ));
    }
    if plan.source_manifest.is_empty()
        || plan.isolation.copy_paths.is_empty()
        || plan.mutations.is_empty()
        || plan.checks.is_empty()
        || plan.negative_controls.is_empty()
        || plan.metrics.is_empty()
        || plan.isolation.max_files == 0
        || plan.isolation.max_bytes == 0
    {
        return Err(ToolError::invalid(
            "metabolism plans require sources, isolation paths, mutations, checks, negative controls, metrics, and positive limits",
        ));
    }
    unique(
        "source manifest paths",
        plan.source_manifest.iter().map(|item| item.path.as_str()),
    )?;
    unique("check IDs", plan.checks.iter().map(|item| item.id.as_str()))?;
    unique(
        "negative-control IDs",
        plan.negative_controls.iter().map(|item| item.id.as_str()),
    )?;
    let metric_ids = unique(
        "metric probes",
        plan.metrics.iter().map(|item| item.metric_id.as_str()),
    )?;
    let expected_metrics = request
        .metrics
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    if metric_ids != expected_metrics {
        return Err(ToolError::invalid(
            "metric probes must exactly cover the purpose envelope metrics",
        ));
    }
    let observations = plan
        .checks
        .iter()
        .flat_map(|item| item.protected_observation_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let expected_observations = request
        .purpose
        .protected_observations
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    if observations != expected_observations {
        return Err(ToolError::invalid(
            "verification checks must exactly cover protected observations",
        ));
    }
    let constraints = plan
        .checks
        .iter()
        .flat_map(|item| item.hard_constraint_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let expected_constraints = request
        .hard_constraints
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    if constraints != expected_constraints {
        return Err(ToolError::invalid(
            "verification checks must exactly cover hard constraints",
        ));
    }
    if !plan
        .forbidden_regression_metrics
        .iter()
        .all(|id| expected_metrics.contains(id.as_str()))
    {
        return Err(ToolError::invalid(
            "forbidden regression metrics must be declared metrics",
        ));
    }
    for path in plan
        .source_manifest
        .iter()
        .map(|item| item.path.as_str())
        .chain(plan.isolation.copy_paths.iter().map(String::as_str))
    {
        validate_relative(path)?;
    }
    for source in &plan.source_manifest {
        require_sha256(&source.sha256, "source manifest SHA-256")?;
    }
    for mutation in &plan.mutations {
        match mutation {
            FileMutation::ReplaceText {
                path,
                preimage_sha256,
                replacements,
            } => {
                validate_relative(path)?;
                require_sha256(preimage_sha256, "mutation preimage SHA-256")?;
                if replacements.is_empty()
                    || replacements.iter().any(|item| {
                        item.old.is_empty()
                            || item.expected_occurrences == 0
                            || item.old == item.new
                    })
                {
                    return Err(ToolError::invalid(
                        "text replacements must be nonempty, effective, and have positive expected counts",
                    ));
                }
            }
            FileMutation::CreateText {
                path,
                content,
                postimage_sha256,
            } => {
                validate_relative(path)?;
                require_sha256(postimage_sha256, "mutation postimage SHA-256")?;
                if content.is_empty() || sha256_hex(content.as_bytes()) != *postimage_sha256 {
                    return Err(ToolError::invalid(
                        "created text does not match its postimage SHA-256",
                    ));
                }
            }
        }
    }
    validate_relative(&plan.model_rebuild.model_path)?;
    Ok(())
}

fn source_root(request: &TransformationRequest) -> Result<&Path, ToolError> {
    let GeneratorSubject::Brownfield {
        repository_root, ..
    } = &request.subject
    else {
        return Err(ToolError::invalid("brownfield source required"));
    };
    Ok(Path::new(repository_root))
}

fn verify_source_manifest(root: &Path, manifest: &[BoundSourceFile]) -> Result<(), ToolError> {
    for source in manifest {
        let path = safe_join(root, &source.path)?;
        let bytes = fs::read(&path).map_err(|error| {
            ToolError::io(format!(
                "could not read bound source {}: {error}",
                path.display()
            ))
        })?;
        if sha256_hex(&bytes) != source.sha256 {
            return Err(ToolError::invalid(format!(
                "bound source {:?} changed before execution",
                source.path
            )));
        }
    }
    Ok(())
}

fn verify_programs(plan: &MetabolismPlan) -> Result<(), ToolError> {
    for command in plan
        .checks
        .iter()
        .map(|item| &item.command)
        .chain(plan.negative_controls.iter().map(|item| &item.command))
        .chain(plan.metrics.iter().map(|item| &item.command))
        .chain(std::iter::once(&plan.model_rebuild.command))
    {
        verify_program(&command.program)?;
        for external in &command.external_files {
            verify_external_file(external)?;
        }
        validate_relative_or_dot(&command.cwd)?;
    }
    Ok(())
}

fn verify_external_file(file: &BoundExternalFile) -> Result<(), ToolError> {
    let path = Path::new(&file.path);
    if !path.is_absolute() {
        return Err(ToolError::invalid(
            "bound external input paths must be absolute",
        ));
    }
    require_sha256(&file.sha256, "external input SHA-256")?;
    let bytes = fs::read(path).map_err(|error| {
        ToolError::io(format!(
            "could not read bound external input {}: {error}",
            path.display()
        ))
    })?;
    if sha256_hex(&bytes) != file.sha256 {
        return Err(ToolError::invalid(format!(
            "bound external input {} does not match its SHA-256",
            path.display()
        )));
    }
    Ok(())
}

fn verify_program(program: &BoundProgram) -> Result<(), ToolError> {
    let path = Path::new(&program.path);
    if !path.is_absolute() {
        return Err(ToolError::invalid("bound program paths must be absolute"));
    }
    require_sha256(&program.sha256, "program SHA-256")?;
    let bytes = fs::read(path).map_err(|error| {
        ToolError::io(format!(
            "could not read bound program {}: {error}",
            path.display()
        ))
    })?;
    if sha256_hex(&bytes) != program.sha256 {
        return Err(ToolError::invalid(format!(
            "bound program {} does not match its SHA-256",
            path.display()
        )));
    }
    Ok(())
}

fn prepare_workspace(
    source: &Path,
    workspace: &Path,
    policy: &IsolationPolicy,
) -> Result<(), ToolError> {
    if !workspace.is_absolute() || workspace.exists() {
        return Err(ToolError::invalid(
            "isolation workspace_root must be absolute and not already exist",
        ));
    }
    let source = source
        .canonicalize()
        .map_err(|error| ToolError::io(format!("could not resolve source root: {error}")))?;
    let parent = workspace
        .parent()
        .ok_or_else(|| ToolError::invalid("workspace root requires a parent"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| ToolError::io(format!("could not resolve workspace parent: {error}")))?;
    let name = workspace
        .file_name()
        .ok_or_else(|| ToolError::invalid("workspace root requires a final component"))?;
    let resolved = parent.join(name);
    if resolved.starts_with(&source) || source.starts_with(&resolved) {
        return Err(ToolError::invalid(
            "isolation workspace must be outside the source repository",
        ));
    }
    fs::create_dir(&resolved).map_err(|error| {
        ToolError::io(format!(
            "could not create workspace {}: {error}",
            resolved.display()
        ))
    })?;
    let baseline = resolved.join("baseline");
    let candidate = resolved.join("candidate");
    fs::create_dir(&baseline)
        .map_err(|error| ToolError::io(format!("could not create baseline workspace: {error}")))?;
    fs::create_dir(&candidate)
        .map_err(|error| ToolError::io(format!("could not create candidate workspace: {error}")))?;
    let excluded = policy
        .excluded_names
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut files = 0usize;
    let mut bytes = 0u64;
    for relative in &policy.copy_paths {
        let from = safe_join(&source, relative)?;
        copy_entry(
            &from,
            &baseline.join(relative),
            &excluded,
            &mut files,
            &mut bytes,
            policy,
        )?;
        copy_entry(
            &from,
            &candidate.join(relative),
            &excluded,
            &mut files,
            &mut bytes,
            policy,
        )?;
    }
    Ok(())
}

fn copy_entry(
    from: &Path,
    to: &Path,
    excluded: &BTreeSet<&str>,
    files: &mut usize,
    bytes: &mut u64,
    policy: &IsolationPolicy,
) -> Result<(), ToolError> {
    let metadata = fs::symlink_metadata(from)
        .map_err(|error| ToolError::io(format!("could not inspect {}: {error}", from.display())))?;
    if metadata.file_type().is_symlink() {
        return Err(ToolError::invalid(format!(
            "isolation copy rejects symbolic link {}",
            from.display()
        )));
    }
    if metadata.is_dir() {
        fs::create_dir_all(to).map_err(|error| {
            ToolError::io(format!("could not create {}: {error}", to.display()))
        })?;
        let mut entries = fs::read_dir(from)
            .map_err(|error| ToolError::io(format!("could not read {}: {error}", from.display())))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                ToolError::io(format!("could not enumerate {}: {error}", from.display()))
            })?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name();
            if name.to_str().is_some_and(|item| excluded.contains(item)) {
                continue;
            }
            copy_entry(
                &entry.path(),
                &to.join(name),
                excluded,
                files,
                bytes,
                policy,
            )?;
        }
    } else if metadata.is_file() {
        *files += 1;
        *bytes = bytes.saturating_add(metadata.len());
        if *files > policy.max_files || *bytes > policy.max_bytes {
            return Err(ToolError::resource_limit(
                "isolation copy exceeded its file or byte bound",
            ));
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ToolError::io(format!("could not create {}: {error}", parent.display()))
            })?;
        }
        fs::copy(from, to).map_err(|error| {
            ToolError::io(format!("could not copy {}: {error}", from.display()))
        })?;
    } else {
        return Err(ToolError::invalid(format!(
            "isolation copy rejects special file {}",
            from.display()
        )));
    }
    Ok(())
}

fn apply_mutation(root: &Path, mutation: &FileMutation) -> Result<MutationReceipt, ToolError> {
    match mutation {
        FileMutation::ReplaceText {
            path,
            preimage_sha256,
            replacements,
        } => {
            let target = safe_join(root, path)?;
            let bytes = fs::read(&target).map_err(|error| {
                ToolError::io(format!(
                    "could not read mutation target {}: {error}",
                    target.display()
                ))
            })?;
            let observed = sha256_hex(&bytes);
            if observed != *preimage_sha256 {
                return Err(ToolError::invalid(format!(
                    "mutation preimage for {path:?} does not match"
                )));
            }
            let mut text = String::from_utf8(bytes).map_err(|_| {
                ToolError::invalid(format!("mutation target {path:?} is not UTF-8"))
            })?;
            for replacement in replacements {
                let count = text.matches(&replacement.old).count();
                if count != replacement.expected_occurrences {
                    return Err(ToolError::invalid(format!(
                        "replacement in {path:?} expected {} occurrences, found {count}",
                        replacement.expected_occurrences
                    )));
                }
                text = text.replace(&replacement.old, &replacement.new);
            }
            let postimage = sha256_hex(text.as_bytes());
            fs::write(&target, text.as_bytes()).map_err(|error| {
                ToolError::io(format!(
                    "could not write mutation target {}: {error}",
                    target.display()
                ))
            })?;
            Ok(MutationReceipt {
                path: path.clone(),
                preimage_sha256: Some(observed.clone()),
                postimage_sha256: postimage.clone(),
                changed: observed != postimage,
            })
        }
        FileMutation::CreateText {
            path,
            content,
            postimage_sha256,
        } => {
            let target = safe_join(root, path)?;
            if target.exists() {
                return Err(ToolError::invalid(format!(
                    "created mutation target {path:?} already exists"
                )));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    ToolError::io(format!("could not create {}: {error}", parent.display()))
                })?;
            }
            fs::write(&target, content.as_bytes()).map_err(|error| {
                ToolError::io(format!("could not write {}: {error}", target.display()))
            })?;
            Ok(MutationReceipt {
                path: path.clone(),
                preimage_sha256: None,
                postimage_sha256: postimage_sha256.clone(),
                changed: true,
            })
        }
    }
}

fn source_revision(
    root: &Path,
    manifest: &[BoundSourceFile],
    mutations: &[FileMutation],
) -> Result<String, ToolError> {
    let mut paths = manifest
        .iter()
        .map(|item| item.path.clone())
        .collect::<BTreeSet<_>>();
    for mutation in mutations {
        let path = match mutation {
            FileMutation::ReplaceText { path, .. } | FileMutation::CreateText { path, .. } => path,
        };
        paths.insert(path.clone());
    }
    let mut identity = b"kerf.source-revision.v1\0".to_vec();
    for path in paths {
        let bytes = fs::read(safe_join(root, &path)?)
            .map_err(|error| ToolError::io(format!("could not hash source {path:?}: {error}")))?;
        identity.extend_from_slice(path.as_bytes());
        identity.push(0);
        identity.extend_from_slice(sha256_hex(&bytes).as_bytes());
        identity.push(0);
    }
    Ok(sha256_hex(&identity))
}

fn run_command(
    root: &Path,
    command: &BoundCommand,
    revision: Option<(&str, &str)>,
) -> Result<RunOutput, ToolError> {
    verify_program(&command.program)?;
    for external in &command.external_files {
        verify_external_file(external)?;
    }
    let cwd = if command.cwd == "." {
        root.to_path_buf()
    } else {
        safe_join(root, &command.cwd)?
    };
    let mut child = Command::new(&command.program.path) // ubs:ignore — absolute executable is content-bound immediately before spawn; no shell
        .args(&command.arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(&command.environment)
        .envs(revision.into_iter().flat_map(|(revision, candidate)| {
            [
                ("KERF_POST_REVISION", revision),
                ("KERF_CANDIDATE_ID", candidate),
            ]
        }))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            ToolError::io(format!(
                "could not execute {}: {error}",
                command.program.path
            ))
        })?;
    if !command.stdin.is_empty() {
        child
            .stdin
            .take()
            .ok_or_else(|| ToolError::internal("child stdin was unavailable"))?
            .write_all(command.stdin.as_bytes())
            .map_err(|error| ToolError::io(format!("could not write child stdin: {error}")))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| ToolError::io(format!("could not collect child output: {error}")))?;
    Ok(RunOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
    })
}

fn command_receipt(
    id: &str,
    kind: &str,
    baseline: &RunOutput,
    candidate: &RunOutput,
    passed: bool,
) -> CommandReceipt {
    CommandReceipt {
        id: id.to_owned(),
        kind: kind.to_owned(),
        baseline_exit_code: baseline.code,
        candidate_exit_code: candidate.code,
        baseline_stdout_sha256: sha256_hex(&baseline.stdout),
        candidate_stdout_sha256: sha256_hex(&candidate.stdout),
        passed,
    }
}

fn parse_metric(id: &str, output: &RunOutput) -> Result<i128, ToolError> {
    if output.code != 0 {
        return Err(ToolError::invalid(format!(
            "metric probe {id:?} exited with code {}",
            output.code
        )));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| ToolError::invalid(format!("metric probe {id:?} did not emit UTF-8")))?;
    text.trim().parse::<i128>().map_err(|_| {
        ToolError::invalid(format!("metric probe {id:?} must emit one signed integer"))
    })
}

fn observed_direction(direction: Direction, baseline: i128, candidate: i128) -> ObservedDirection {
    if baseline == candidate {
        return ObservedDirection::Unchanged;
    }
    match direction {
        Direction::Minimize if candidate < baseline => ObservedDirection::Better,
        Direction::Maximize if candidate > baseline => ObservedDirection::Better,
        Direction::Minimize | Direction::Maximize => ObservedDirection::Worse,
    }
}

fn prediction_match(predicted: PredictedDirection, observed: ObservedDirection) -> Option<bool> {
    match predicted {
        PredictedDirection::Unknown => None,
        PredictedDirection::Better => Some(observed == ObservedDirection::Better),
        PredictedDirection::Worse => Some(observed == ObservedDirection::Worse),
        PredictedDirection::Unchanged => Some(observed == ObservedDirection::Unchanged),
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, ToolError> {
    validate_relative(relative)?;
    Ok(root.join(relative))
}

fn validate_relative(path: &str) -> Result<(), ToolError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(ToolError::invalid(format!(
            "path {} must be a nonempty normalized relative path",
            path.display()
        )));
    }
    Ok(())
}

fn validate_relative_or_dot(path: &str) -> Result<(), ToolError> {
    if path == "." {
        Ok(())
    } else {
        validate_relative(path)
    }
}

fn unique<'a>(
    label: &str,
    values: impl Iterator<Item = &'a str>,
) -> Result<BTreeSet<&'a str>, ToolError> {
    let mut result = BTreeSet::new();
    for value in values {
        if value.is_empty() || !result.insert(value) {
            return Err(ToolError::invalid(format!(
                "{label} must be nonempty and unique"
            )));
        }
    }
    Ok(result)
}

fn require_sha256(value: &str, label: &str) -> Result<(), ToolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ToolError::invalid(format!(
            "{label} must be lowercase hexadecimal"
        )));
    }
    Ok(())
}

fn canonical_sha256<T: Serialize>(value: &T, label: &str) -> Result<String, ToolError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| ToolError::internal(format!("could not serialize {label}: {error}")))
}
