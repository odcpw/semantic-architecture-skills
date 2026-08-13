//! Executed, provenance-pinned evidence and typed filesystem topology.
//!
//! This module deliberately does not attest source-to-binary derivation. It
//! records and verifies source, executable, dependency, toolchain, invocation,
//! and artifact identities, then applies an explicit validator-authority
//! ceiling to the claims that consume those observations.

use crate::semantic::{ClaimKind, ClaimVerdict, ClosureKind, EpistemicClass};
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub const EXECUTION_PLAN_SCHEMA: &str = "kerf.executed-evidence-plan.v1";
pub const EXECUTED_MANIFEST_SCHEMA: &str = "kerf.executed-evidence-manifest.v1";
pub const TOPOLOGY_ALGORITHM: &str = "kerf.fs-topology.v1";
pub const PARITY_CORPUS_SCHEMA: &str = "kerf.parity-corpus-spec.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlan {
    pub schema_version: String,
    pub manifest_id: String,
    pub decision_id: String,
    pub closure: ClosureKind,
    pub git_program: BoundFile,
    pub subjects: Vec<SubjectPin>,
    pub invocations: Vec<InvocationPlan>,
    pub validators: Vec<ValidatorPlan>,
    pub claims: Vec<ExecutedClaimPlan>,
    pub negative_controls: Vec<NegativeControlPlan>,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectPin {
    pub id: String,
    pub repository: String,
    pub git: GitObjectPin,
    pub checkout: CheckoutBinding,
    pub dirty: DirtyBinding,
    pub executable: BoundFile,
    #[serde(default)]
    pub locks: Vec<BoundFile>,
    pub toolchain: ToolchainPin,
    pub build_attestation: BuildAttestation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitObjectPin {
    pub commit: String,
    pub tree: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutBinding {
    ObjectOnly,
    CurrentCheckout,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirtyBinding {
    Clean { patch_sha256: String },
    Dirty { patch_sha256: String },
    Unattested,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainPin {
    pub program: BoundFile,
    pub args: Vec<String>,
    pub stdout_sha256: String,
    pub target: String,
    pub profile: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildAttestation {
    None,
    Declared,
    ReproducibleReceipt,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationPlan {
    pub id: String,
    pub case_id: String,
    pub subject_id: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin_hex: String,
    #[serde(default)]
    pub input_topologies: Vec<TopologyPin>,
    #[serde(default)]
    pub output_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyPin {
    pub root: String,
    pub digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorPlan {
    pub id: String,
    pub kind: ValidatorKind,
    pub authority: ValidatorAuthority,
    pub observed_invocation_id: String,
    pub expected: ExpectedObservation,
    #[serde(default)]
    pub authority_evidence: Option<GitFixtureEvidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitFixtureEvidence {
    pub subject_id: String,
    pub commit: String,
    pub path: String,
    pub sha256: String,
    pub origin: AuthorityEvidenceOrigin,
    #[serde(default)]
    pub json_pointer: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityEvidenceOrigin {
    ProspectiveBaseline,
    ExternalReceipt,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorKind {
    ExactExit,
    ExactStdout,
    ExactStderr,
    ExactTopology,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorAuthority {
    SelfProduced,
    IndependentFixture,
    SecondImplementation,
    TrustedOracle,
    ReproducibleBuild,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpectedObservation {
    Invocation { invocation_id: String },
    Literal { value: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedClaimPlan {
    pub id: String,
    pub description: String,
    pub subject: ExecutedClaimSubject,
    pub kind: ClaimKind,
    pub epistemic_class: EpistemicClass,
    pub closure: ClosureKind,
    pub validator_ids: Vec<String>,
    pub negative_control_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutedClaimSubject {
    pub coordinate: ExecutedClaimCoordinate,
    pub subject_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutedClaimCoordinate {
    BoundExecutable,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControlPlan {
    pub id: String,
    pub claim_id: String,
    pub validator_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParityCorpusSpec {
    pub schema_version: String,
    pub manifest_id: String,
    pub decision_id: String,
    pub closure: ClosureKind,
    pub git_program: BoundFile,
    pub subjects: Vec<SubjectPin>,
    pub baseline_subject_id: String,
    pub candidate_subject_id: String,
    pub validator_kind: ValidatorKind,
    pub fixture: ParityCorpusFixture,
    pub cases: Vec<ParityCorpusCase>,
    pub negative_control: ParityCorpusNegativeControl,
    pub claim: ParityCorpusClaim,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParityCorpusFixture {
    pub subject_id: String,
    pub commit: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParityCorpusCase {
    pub id: String,
    pub baseline: InvocationCommand,
    pub candidate: InvocationCommand,
    pub expected_literal: String,
    pub fixture_json_pointer: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParityCorpusNegativeControl {
    pub id: String,
    pub case_id: String,
    pub invocation_id: String,
    pub validator_id: String,
    pub command: InvocationCommand,
    pub expected_literal: String,
    pub fixture_json_pointer: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParityCorpusClaim {
    pub id: String,
    pub description: String,
    pub kind: ClaimKind,
    pub epistemic_class: EpistemicClass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationCommand {
    pub argv: Vec<String>,
    pub cwd: String,
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin_hex: String,
    #[serde(default)]
    pub input_topologies: Vec<TopologyPin>,
    #[serde(default)]
    pub output_root: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestStatus {
    Accepted,
    Refuted,
    BindingRejected,
    NegativeControlFailed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorStatus {
    Accepted,
    Refuted,
    Unknown,
    NotRun,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedEvidenceManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub decision_id: String,
    pub plan_sha256: String,
    pub closure: ClosureKind,
    pub status: ManifestStatus,
    pub subject_receipts: Vec<SubjectReceipt>,
    pub invocation_receipts: Vec<InvocationReceipt>,
    pub validator_receipts: Vec<ValidatorReceipt>,
    pub claim_receipts: Vec<ExecutedClaimReceipt>,
    pub negative_control_receipts: Vec<NegativeControlReceipt>,
    pub counterexamples: Vec<Counterexample>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<ManifestWitness>,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectReceipt {
    pub id: String,
    pub repository: String,
    pub commit: String,
    pub tree: String,
    pub dirty_patch_sha256: Option<String>,
    pub executable_sha256: String,
    pub lock_sha256s: Vec<String>,
    pub toolchain_stdout_sha256: String,
    pub build_attestation: BuildAttestation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationReceipt {
    pub id: String,
    pub case_id: String,
    pub subject_id: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub environment: BTreeMap<String, String>,
    pub stdin_sha256: String,
    pub input_topologies: Vec<TopologySnapshot>,
    pub program_sha256_pre: String,
    pub program_sha256_pre_spawn: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
    pub stdout_sha256: String,
    pub stdout_len: u64,
    pub stderr_sha256: String,
    pub stderr_len: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_topology: Option<TopologySnapshot>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TopologySnapshot {
    pub algorithm: String,
    pub root: String,
    pub digest: String,
    pub nodes: Vec<TopologyNode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TopologyNode {
    pub relative_path: String,
    pub kind: TopologyNodeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_len: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dangling: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TopologyNodeKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorReceipt {
    pub id: String,
    pub kind: ValidatorKind,
    pub authority: ValidatorAuthority,
    pub observed_invocation_id: String,
    pub expected_source: ExpectedObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_evidence: Option<GitFixtureEvidenceReceipt>,
    pub observed: String,
    pub expected: String,
    pub status: ValidatorStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitFixtureEvidenceReceipt {
    pub subject_id: String,
    pub commit: String,
    pub path: String,
    pub sha256: String,
    pub origin: AuthorityEvidenceOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_pointer: Option<String>,
    pub observed_value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutedClaimReceipt {
    pub id: String,
    pub subject: ExecutedClaimSubject,
    pub kind: ClaimKind,
    pub epistemic_class: EpistemicClass,
    pub closure: ClosureKind,
    pub verdict: ClaimVerdict,
    pub validator_ids: Vec<String>,
    pub negative_control_id: String,
    pub negative_control_passed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControlReceipt {
    pub id: String,
    pub claim_id: String,
    pub validator_id: String,
    pub observed_status: ValidatorStatus,
    pub passed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counterexample {
    pub claim_id: String,
    pub validator_id: String,
    pub witness: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestWitness {
    pub kind: String,
    pub subject: String,
    pub expected: Option<String>,
    pub observed: Option<String>,
}

/// Snapshot one filesystem root without following symlinks.
///
/// Directories, including empty directories, are nodes. File contents and raw
/// UTF-8 symlink targets are hashed. Metadata such as permissions, ownership,
/// timestamps, Windows junction/reparse subtype, mount identity, and extended
/// attributes is intentionally excluded from this v1 observation alphabet.
///
/// # Errors
///
/// Returns [`ToolError`] when the root cannot be inspected, a path/target is
/// not UTF-8, an unsupported node kind is encountered, or bytes cannot be read.
pub fn snapshot_topology(root: &Path) -> Result<TopologySnapshot, ToolError> {
    if !root.is_absolute() {
        return Err(ToolError::invalid("topology root must be absolute"));
    }
    let mut nodes = Vec::new();
    collect_topology(root, root, &mut nodes)?;
    nodes.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let digest = topology_digest(&nodes)?;
    Ok(TopologySnapshot {
        algorithm: TOPOLOGY_ALGORITHM.to_owned(),
        root: root.to_string_lossy().into_owned(),
        digest,
        nodes,
    })
}

fn collect_topology(
    root: &Path,
    path: &Path,
    nodes: &mut Vec<TopologyNode>,
) -> Result<(), ToolError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        ToolError::io(format!("inspect topology node {}: {error}", path.display()))
    })?;
    let relative_path = if path == root {
        String::new()
    } else {
        normalized_relative(root, path)?
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = fs::read_link(path).map_err(|error| {
            ToolError::io(format!("read symlink target {}: {error}", path.display()))
        })?;
        let target = target.to_str().ok_or_else(|| {
            ToolError::invalid(format!(
                "symlink target for {} is not valid UTF-8",
                path.display()
            ))
        })?;
        let dangling = !path.try_exists().map_err(|error| {
            ToolError::io(format!(
                "determine whether symlink {} is dangling: {error}",
                path.display()
            ))
        })?;
        nodes.push(TopologyNode {
            relative_path,
            kind: TopologyNodeKind::Symlink,
            sha256: None,
            byte_len: None,
            link_target: Some(target.to_owned()),
            dangling: Some(dangling),
        });
        return Ok(());
    }
    if file_type.is_file() {
        let bytes = fs::read(path).map_err(|error| {
            ToolError::io(format!("read topology file {}: {error}", path.display()))
        })?;
        nodes.push(TopologyNode {
            relative_path,
            kind: TopologyNodeKind::File,
            sha256: Some(sha256_hex(&bytes)),
            byte_len: Some(bytes.len() as u64),
            link_target: None,
            dangling: None,
        });
        return Ok(());
    }
    if file_type.is_dir() {
        nodes.push(TopologyNode {
            relative_path,
            kind: TopologyNodeKind::Directory,
            sha256: None,
            byte_len: None,
            link_target: None,
            dangling: None,
        });
        let mut children = fs::read_dir(path)
            .map_err(|error| {
                ToolError::io(format!(
                    "read topology directory {}: {error}",
                    path.display()
                ))
            })?
            .map(|entry| {
                entry.map(|entry| entry.path()).map_err(|error| {
                    ToolError::io(format!(
                        "read topology entry in {}: {error}",
                        path.display()
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            collect_topology(root, &child, nodes)?;
        }
        return Ok(());
    }
    Err(ToolError::invalid(format!(
        "unsupported filesystem node kind at {}",
        path.display()
    )))
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, ToolError> {
    let relative = path.strip_prefix(root).map_err(|error| {
        ToolError::internal(format!(
            "topology node {} is outside root {}: {error}",
            path.display(),
            root.display()
        ))
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let part = component.as_os_str().to_str().ok_or_else(|| {
            ToolError::invalid(format!("path {} is not valid UTF-8", path.display()))
        })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn topology_digest(nodes: &[TopologyNode]) -> Result<String, ToolError> {
    let mut hasher = Sha256::new();
    hasher.update(format!("{TOPOLOGY_ALGORITHM}\0").as_bytes());
    for node in nodes {
        let path = node.relative_path.as_bytes();
        let kind = match node.kind {
            TopologyNodeKind::File => 0_u8,
            TopologyNodeKind::Directory => 1,
            TopologyNodeKind::Symlink => 2,
        };
        hasher.update([kind]);
        hasher.update((path.len() as u64).to_le_bytes());
        hasher.update(path);
        match node.kind {
            TopologyNodeKind::File => {
                let byte_len = node
                    .byte_len
                    .ok_or_else(|| ToolError::internal("file topology node has no byte_len"))?;
                let sha256 = node
                    .sha256
                    .as_deref()
                    .ok_or_else(|| ToolError::internal("file topology node has no sha256"))?;
                hasher.update(byte_len.to_le_bytes());
                hasher.update(sha256.as_bytes());
            }
            TopologyNodeKind::Directory => {}
            TopologyNodeKind::Symlink => {
                let target = node.link_target.as_deref().ok_or_else(|| {
                    ToolError::internal("symlink topology node has no link_target")
                })?;
                hasher.update((target.len() as u64).to_le_bytes());
                hasher.update(target.as_bytes());
                hasher.update([u8::from(node.dangling.unwrap_or(false))]);
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Expand a compact, typed baseline/candidate corpus into an executable plan.
///
/// Each declared case becomes two pinned invocations, an independent fixture
/// validator for the candidate, and a second-implementation parity validator.
/// The declared semantic negative control becomes a separate candidate
/// invocation whose fixture validator must refute the nearby wrong behavior.
/// The expanded plan is passed through the same validation gate as a
/// hand-authored plan before it is returned.
///
/// # Errors
///
/// Returns [`ToolError`] when the corpus schema, identities, subject bindings,
/// commands, fixture references, or expanded execution plan are invalid.
pub fn expand_parity_corpus(spec: &ParityCorpusSpec) -> Result<ExecutionPlan, ToolError> {
    validate_parity_spec(spec)?;
    let fixture_evidence = |json_pointer: &str| GitFixtureEvidence {
        subject_id: spec.fixture.subject_id.clone(),
        commit: spec.fixture.commit.clone(),
        path: spec.fixture.path.clone(),
        sha256: spec.fixture.sha256.clone(),
        origin: AuthorityEvidenceOrigin::ProspectiveBaseline,
        json_pointer: Some(json_pointer.to_owned()),
    };
    let mut invocations = Vec::with_capacity(spec.cases.len() * 2 + 1);
    let mut validators = Vec::with_capacity(spec.cases.len() * 2 + 1);
    let mut validator_ids = Vec::with_capacity(spec.cases.len() * 2);

    for case in &spec.cases {
        let baseline_id = format!("baseline-{}", case.id);
        let candidate_id = format!("candidate-{}", case.id);
        let fixture_id = format!("fixture-{}", case.id);
        let parity_id = format!("parity-{}", case.id);
        invocations.push(expanded_invocation(
            &baseline_id,
            &case.id,
            &spec.baseline_subject_id,
            &case.baseline,
        ));
        invocations.push(expanded_invocation(
            &candidate_id,
            &case.id,
            &spec.candidate_subject_id,
            &case.candidate,
        ));
        validators.push(ValidatorPlan {
            id: fixture_id.clone(),
            kind: spec.validator_kind,
            authority: ValidatorAuthority::IndependentFixture,
            observed_invocation_id: candidate_id.clone(),
            expected: ExpectedObservation::Literal {
                value: case.expected_literal.clone(),
            },
            authority_evidence: Some(fixture_evidence(&case.fixture_json_pointer)),
        });
        validators.push(ValidatorPlan {
            id: parity_id.clone(),
            kind: spec.validator_kind,
            authority: ValidatorAuthority::SecondImplementation,
            observed_invocation_id: candidate_id,
            expected: ExpectedObservation::Invocation {
                invocation_id: baseline_id,
            },
            authority_evidence: None,
        });
        validator_ids.extend([fixture_id, parity_id]);
    }

    invocations.push(expanded_invocation(
        &spec.negative_control.invocation_id,
        &spec.negative_control.case_id,
        &spec.candidate_subject_id,
        &spec.negative_control.command,
    ));
    validators.push(ValidatorPlan {
        id: spec.negative_control.validator_id.clone(),
        kind: spec.validator_kind,
        authority: ValidatorAuthority::IndependentFixture,
        observed_invocation_id: spec.negative_control.invocation_id.clone(),
        expected: ExpectedObservation::Literal {
            value: spec.negative_control.expected_literal.clone(),
        },
        authority_evidence: Some(fixture_evidence(
            &spec.negative_control.fixture_json_pointer,
        )),
    });

    let plan = expanded_plan(spec, invocations, validators, validator_ids);
    validate_plan(&plan)?;
    Ok(plan)
}

fn validate_parity_spec(spec: &ParityCorpusSpec) -> Result<(), ToolError> {
    if spec.schema_version != PARITY_CORPUS_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {PARITY_CORPUS_SCHEMA:?}",
            spec.schema_version
        )));
    }
    if spec.baseline_subject_id == spec.candidate_subject_id {
        return Err(ToolError::invalid(
            "baseline_subject_id and candidate_subject_id must be distinct",
        ));
    }
    let subject_ids = unique_ids("subjects", spec.subjects.iter().map(|item| &item.id))?;
    for (label, subject_id) in [
        ("baseline_subject_id", &spec.baseline_subject_id),
        ("candidate_subject_id", &spec.candidate_subject_id),
        ("fixture.subject_id", &spec.fixture.subject_id),
    ] {
        if !subject_ids.contains(subject_id.as_str()) {
            return Err(ToolError::invalid(format!(
                "{label} references unknown subject {subject_id:?}"
            )));
        }
    }
    if spec.cases.is_empty() {
        return Err(ToolError::invalid(
            "parity corpus must declare at least one positive case",
        ));
    }
    unique_ids("parity cases", spec.cases.iter().map(|item| &item.id))?;
    Ok(())
}

fn expanded_plan(
    spec: &ParityCorpusSpec,
    invocations: Vec<InvocationPlan>,
    validators: Vec<ValidatorPlan>,
    validator_ids: Vec<String>,
) -> ExecutionPlan {
    ExecutionPlan {
        schema_version: EXECUTION_PLAN_SCHEMA.to_owned(),
        manifest_id: spec.manifest_id.clone(),
        decision_id: spec.decision_id.clone(),
        closure: spec.closure,
        git_program: spec.git_program.clone(),
        subjects: spec.subjects.clone(),
        invocations,
        validators,
        claims: vec![ExecutedClaimPlan {
            id: spec.claim.id.clone(),
            description: spec.claim.description.clone(),
            subject: ExecutedClaimSubject {
                coordinate: ExecutedClaimCoordinate::BoundExecutable,
                subject_ids: vec![
                    spec.baseline_subject_id.clone(),
                    spec.candidate_subject_id.clone(),
                ],
            },
            kind: spec.claim.kind,
            epistemic_class: spec.claim.epistemic_class,
            closure: spec.closure,
            validator_ids,
            negative_control_id: spec.negative_control.id.clone(),
        }],
        negative_controls: vec![NegativeControlPlan {
            id: spec.negative_control.id.clone(),
            claim_id: spec.claim.id.clone(),
            validator_id: spec.negative_control.validator_id.clone(),
        }],
        nonclaims: spec.nonclaims.clone(),
    }
}

fn expanded_invocation(
    id: &str,
    case_id: &str,
    subject_id: &str,
    command: &InvocationCommand,
) -> InvocationPlan {
    InvocationPlan {
        id: id.to_owned(),
        case_id: case_id.to_owned(),
        subject_id: subject_id.to_owned(),
        argv: command.argv.clone(),
        cwd: command.cwd.clone(),
        environment: command.environment.clone(),
        stdin_hex: command.stdin_hex.clone(),
        input_topologies: command.input_topologies.clone(),
        output_root: command.output_root.clone(),
    }
}

/// Execute a fully pinned evidence plan and return its receipts.
///
/// The runner clears inherited environment variables, executes programs
/// directly without a shell, and captures stdout/stderr in memory. It does not
/// sandbox, impose a timeout/output cap, eliminate filesystem races, or prove
/// that a pinned executable was built from a pinned source object.
///
/// # Errors
///
/// Returns [`ToolError`] for malformed schemas and operating-system failures
/// that prevent the requested observation. Expected binding or semantic
/// disagreement is represented in the returned manifest status and witness.
#[allow(clippy::too_many_lines)]
pub fn execute_plan(plan: &ExecutionPlan) -> Result<ExecutedEvidenceManifest, ToolError> {
    validate_plan(plan)?;
    let plan_sha256 = canonical_sha256(plan, "execution plan")?;
    let mut manifest = empty_manifest(plan, plan_sha256);

    if let Some(witness) = verify_bound_file("git_program", &plan.git_program)? {
        return Ok(reject_binding(manifest, witness));
    }

    let mut subjects = BTreeMap::new();
    for subject in &plan.subjects {
        match verify_subject(&plan.git_program, subject)? {
            Ok(receipt) => {
                subjects.insert(subject.id.as_str(), subject);
                manifest.subject_receipts.push(receipt);
            }
            Err(witness) => return Ok(reject_binding(manifest, witness)),
        }
    }

    for invocation in &plan.invocations {
        let subject = subjects[invocation.subject_id.as_str()];
        match execute_invocation(subject, invocation)? {
            Ok(receipt) => manifest.invocation_receipts.push(receipt),
            Err(witness) => return Ok(reject_binding(manifest, witness)),
        }
    }

    let receipt_index: BTreeMap<_, _> = manifest
        .invocation_receipts
        .iter()
        .map(|receipt| (receipt.id.as_str(), receipt))
        .collect();
    for validator in &plan.validators {
        let authority_evidence = match verify_validator_authority(
            &plan.git_program,
            validator,
            &subjects,
            &receipt_index,
        )? {
            Ok(receipt) => receipt,
            Err(witness) => return Ok(reject_binding(manifest, witness)),
        };
        manifest.validator_receipts.push(run_validator(
            validator,
            &receipt_index,
            authority_evidence,
        )?);
    }

    let validator_index: BTreeMap<_, _> = manifest
        .validator_receipts
        .iter()
        .map(|receipt| (receipt.id.as_str(), receipt))
        .collect();
    for control in &plan.negative_controls {
        let observed = validator_index[control.validator_id.as_str()].status;
        manifest
            .negative_control_receipts
            .push(NegativeControlReceipt {
                id: control.id.clone(),
                claim_id: control.claim_id.clone(),
                validator_id: control.validator_id.clone(),
                observed_status: observed,
                passed: observed == ValidatorStatus::Refuted,
            });
    }

    let control_index: BTreeMap<_, _> = manifest
        .negative_control_receipts
        .iter()
        .map(|receipt| (receipt.id.as_str(), receipt))
        .collect();
    for claim in &plan.claims {
        let validators = claim
            .validator_ids
            .iter()
            .map(|id| validator_index[id.as_str()])
            .collect::<Vec<_>>();
        let control = control_index[claim.negative_control_id.as_str()];
        let authority_error = claim_authority_error(plan, claim, &validators);
        if let Some(message) = authority_error {
            manifest
                .claim_receipts
                .push(claim_receipt(claim, ClaimVerdict::Unsupported, false));
            return Ok(reject_binding(
                manifest,
                ManifestWitness {
                    kind: "claim_authority_ceiling".to_owned(),
                    subject: claim.id.clone(),
                    expected: None,
                    observed: Some(message),
                },
            ));
        }
        let verdict = if validators
            .iter()
            .any(|receipt| receipt.status == ValidatorStatus::Refuted)
        {
            ClaimVerdict::Refuted
        } else if validators
            .iter()
            .all(|receipt| receipt.status == ValidatorStatus::Accepted)
            && control.passed
        {
            ClaimVerdict::Supported
        } else {
            ClaimVerdict::Unknown
        };
        if verdict == ClaimVerdict::Refuted {
            for validator in validators
                .iter()
                .filter(|receipt| receipt.status == ValidatorStatus::Refuted)
            {
                manifest.counterexamples.push(Counterexample {
                    claim_id: claim.id.clone(),
                    validator_id: validator.id.clone(),
                    witness: validator
                        .witness
                        .clone()
                        .unwrap_or_else(|| "validator refuted the claim".to_owned()),
                });
            }
        }
        manifest
            .claim_receipts
            .push(claim_receipt(claim, verdict, control.passed));
    }

    manifest.status = if manifest
        .negative_control_receipts
        .iter()
        .any(|receipt| !receipt.passed)
    {
        ManifestStatus::NegativeControlFailed
    } else if manifest
        .claim_receipts
        .iter()
        .any(|receipt| receipt.verdict == ClaimVerdict::Refuted)
    {
        ManifestStatus::Refuted
    } else if manifest
        .claim_receipts
        .iter()
        .all(|receipt| receipt.verdict == ClaimVerdict::Supported)
    {
        ManifestStatus::Accepted
    } else {
        ManifestStatus::Unknown
    };
    if manifest.status == ManifestStatus::Accepted
        && audit_manifest(plan, &manifest)? != ManifestStatus::Accepted
    {
        return Err(ToolError::internal(
            "execute_plan produced an accepted manifest that failed immediate audit",
        ));
    }
    Ok(manifest)
}

/// Re-audit a serialized manifest before convergence evaluation.
///
/// This catches post-execution tampering such as changing `executed` to false
/// while retaining an accepted status. It cannot replay commands or reconstruct
/// stdout/stderr from their hashes.
///
/// # Errors
///
/// Returns [`ToolError`] when the manifest schema or internal references are
/// malformed.
#[allow(clippy::too_many_lines)]
pub fn audit_manifest(
    plan: &ExecutionPlan,
    manifest: &ExecutedEvidenceManifest,
) -> Result<ManifestStatus, ToolError> {
    validate_plan(plan)?;
    if manifest.schema_version != EXECUTED_MANIFEST_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported manifest schema {:?}; expected {EXECUTED_MANIFEST_SCHEMA:?}",
            manifest.schema_version
        )));
    }
    require_nonempty("manifest_id", &manifest.manifest_id)?;
    require_sha256("plan_sha256", &manifest.plan_sha256)?;
    let expected_plan_sha256 = canonical_sha256(plan, "execution plan")?;
    if manifest.plan_sha256 != expected_plan_sha256
        || manifest.manifest_id != plan.manifest_id
        || manifest.decision_id != plan.decision_id
        || manifest.closure != plan.closure
        || !manifest_matches_plan(plan, manifest)
    {
        return Ok(ManifestStatus::BindingRejected);
    }
    if manifest.nonclaims.is_empty() || manifest.nonclaims.iter().any(String::is_empty) {
        return Err(ToolError::invalid(
            "executed manifest must retain at least one non-empty nonclaim",
        ));
    }
    if manifest.status == ManifestStatus::BindingRejected {
        return Ok(ManifestStatus::BindingRejected);
    }
    if manifest.subject_receipts.len() < 2
        || manifest.invocation_receipts.is_empty()
        || manifest.validator_receipts.is_empty()
        || manifest.claim_receipts.is_empty()
        || manifest.negative_control_receipts.is_empty()
    {
        return Ok(ManifestStatus::BindingRejected);
    }
    if manifest
        .invocation_receipts
        .iter()
        .any(|receipt| !receipt.executed)
    {
        return Ok(ManifestStatus::BindingRejected);
    }
    let subjects = audit_subjects(&manifest.subject_receipts)?;
    if subjects.is_empty() {
        return Ok(ManifestStatus::BindingRejected);
    }
    let invocations = audit_invocations(&manifest.invocation_receipts, &subjects)?;
    if invocations.is_empty() {
        return Ok(ManifestStatus::BindingRejected);
    }
    let validators = audit_validators(
        &manifest.validator_receipts,
        &invocations,
        &manifest.subject_receipts,
    )?;
    if validators.is_empty() {
        return Ok(ManifestStatus::BindingRejected);
    }
    let controls = audit_negative_controls(&manifest.negative_control_receipts, &validators);
    if controls.is_empty() {
        return Ok(ManifestStatus::BindingRejected);
    }
    for claim in &manifest.claim_receipts {
        let Some(control) = controls.get(claim.negative_control_id.as_str()) else {
            return Ok(ManifestStatus::BindingRejected);
        };
        if control.claim_id != claim.id || claim.negative_control_passed != control.passed {
            return Ok(ManifestStatus::BindingRejected);
        }
        let mut claim_validators = Vec::new();
        for id in &claim.validator_ids {
            let Some(validator) = validators.get(id.as_str()) else {
                return Ok(ManifestStatus::BindingRejected);
            };
            claim_validators.push(*validator);
        }
        if claim_validators.is_empty()
            || audited_claim_authority_error(claim, &claim_validators).is_some()
        {
            return Ok(ManifestStatus::BindingRejected);
        }
        let expected_verdict = if claim_validators
            .iter()
            .any(|validator| validator.status == ValidatorStatus::Refuted)
        {
            ClaimVerdict::Refuted
        } else if claim_validators
            .iter()
            .all(|validator| validator.status == ValidatorStatus::Accepted)
            && control.passed
        {
            ClaimVerdict::Supported
        } else {
            ClaimVerdict::Unknown
        };
        if claim.verdict != expected_verdict {
            return Ok(ManifestStatus::BindingRejected);
        }
    }
    let expected_status = if controls.values().any(|receipt| !receipt.passed) {
        ManifestStatus::NegativeControlFailed
    } else if manifest
        .claim_receipts
        .iter()
        .any(|claim| claim.verdict == ClaimVerdict::Refuted)
    {
        ManifestStatus::Refuted
    } else if manifest
        .claim_receipts
        .iter()
        .all(|claim| claim.verdict == ClaimVerdict::Supported)
    {
        ManifestStatus::Accepted
    } else {
        ManifestStatus::Unknown
    };
    Ok(if manifest.status == expected_status {
        manifest.status
    } else {
        ManifestStatus::BindingRejected
    })
}

#[allow(clippy::too_many_lines)]
fn manifest_matches_plan(plan: &ExecutionPlan, manifest: &ExecutedEvidenceManifest) -> bool {
    let subject_plans: BTreeMap<_, _> = plan
        .subjects
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let invocation_plans: BTreeMap<_, _> = plan
        .invocations
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let validator_plans: BTreeMap<_, _> = plan
        .validators
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let claim_plans: BTreeMap<_, _> = plan
        .claims
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let control_plans: BTreeMap<_, _> = plan
        .negative_controls
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    if manifest.subject_receipts.len() != subject_plans.len()
        || manifest.invocation_receipts.len() != invocation_plans.len()
        || manifest.validator_receipts.len() != validator_plans.len()
        || manifest.claim_receipts.len() != claim_plans.len()
        || manifest.negative_control_receipts.len() != control_plans.len()
    {
        return false;
    }
    for receipt in &manifest.subject_receipts {
        let Some(expected) = subject_plans.get(receipt.id.as_str()) else {
            return false;
        };
        let dirty = match &expected.dirty {
            DirtyBinding::Clean { patch_sha256 } | DirtyBinding::Dirty { patch_sha256 } => {
                Some(patch_sha256)
            }
            DirtyBinding::Unattested => None,
        };
        if receipt.repository != expected.repository
            || receipt.commit != expected.git.commit
            || receipt.tree != expected.git.tree
            || receipt.dirty_patch_sha256.as_ref() != dirty
            || receipt.executable_sha256 != expected.executable.sha256
            || receipt.lock_sha256s
                != expected
                    .locks
                    .iter()
                    .map(|item| item.sha256.clone())
                    .collect::<Vec<_>>()
            || receipt.toolchain_stdout_sha256 != expected.toolchain.stdout_sha256
            || receipt.build_attestation != expected.build_attestation
        {
            return false;
        }
    }
    for receipt in &manifest.invocation_receipts {
        let Some(expected) = invocation_plans.get(receipt.id.as_str()) else {
            return false;
        };
        if receipt.case_id != expected.case_id
            || receipt.subject_id != expected.subject_id
            || receipt.argv != expected.argv
            || receipt.cwd != expected.cwd
            || receipt.environment != expected.environment
            || receipt.stdin_sha256
                != sha256_hex(&decode_hex(&expected.stdin_hex, "stdin").unwrap_or_default())
            || receipt.input_topologies.len() != expected.input_topologies.len()
            || receipt
                .input_topologies
                .iter()
                .zip(&expected.input_topologies)
                .any(|(actual, planned)| {
                    actual.root != planned.root || actual.digest != planned.digest
                })
            || receipt
                .output_topology
                .as_ref()
                .map(|item| item.root.as_str())
                != expected.output_root.as_deref()
        {
            return false;
        }
    }
    for receipt in &manifest.validator_receipts {
        let Some(expected) = validator_plans.get(receipt.id.as_str()) else {
            return false;
        };
        if receipt.kind != expected.kind
            || receipt.authority != expected.authority
            || receipt.observed_invocation_id != expected.observed_invocation_id
            || receipt.expected_source != expected.expected
            || receipt.authority_evidence.as_ref().map(|item| {
                (
                    &item.subject_id,
                    &item.commit,
                    &item.path,
                    &item.sha256,
                    item.origin,
                    &item.json_pointer,
                )
            }) != expected.authority_evidence.as_ref().map(|item| {
                (
                    &item.subject_id,
                    &item.commit,
                    &item.path,
                    &item.sha256,
                    item.origin,
                    &item.json_pointer,
                )
            })
        {
            return false;
        }
    }
    for receipt in &manifest.claim_receipts {
        let Some(expected) = claim_plans.get(receipt.id.as_str()) else {
            return false;
        };
        if receipt.subject != expected.subject
            || receipt.kind != expected.kind
            || receipt.epistemic_class != expected.epistemic_class
            || receipt.closure != expected.closure
            || receipt.validator_ids != expected.validator_ids
            || receipt.negative_control_id != expected.negative_control_id
        {
            return false;
        }
    }
    manifest.negative_control_receipts.iter().all(|receipt| {
        control_plans
            .get(receipt.id.as_str())
            .is_some_and(|expected| {
                receipt.claim_id == expected.claim_id
                    && receipt.validator_id == expected.validator_id
            })
    })
}

#[allow(clippy::too_many_lines)]
fn validate_plan(plan: &ExecutionPlan) -> Result<(), ToolError> {
    if plan.schema_version != EXECUTION_PLAN_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {EXECUTION_PLAN_SCHEMA:?}",
            plan.schema_version
        )));
    }
    require_nonempty("manifest_id", &plan.manifest_id)?;
    require_nonempty("decision_id", &plan.decision_id)?;
    if plan.nonclaims.is_empty() || plan.nonclaims.iter().any(String::is_empty) {
        return Err(ToolError::invalid(
            "nonclaims must contain at least one non-empty statement",
        ));
    }
    if plan.subjects.len() < 2 {
        return Err(ToolError::invalid(
            "subjects must bind at least baseline and candidate",
        ));
    }
    if plan.invocations.is_empty() || plan.validators.is_empty() || plan.claims.is_empty() {
        return Err(ToolError::invalid(
            "invocations, validators, and claims must not be empty",
        ));
    }
    let subject_ids = unique_ids("subjects", plan.subjects.iter().map(|item| &item.id))?;
    let subject_plans: BTreeMap<_, _> = plan
        .subjects
        .iter()
        .map(|subject| (subject.id.as_str(), subject))
        .collect();
    let invocation_ids = unique_ids("invocations", plan.invocations.iter().map(|item| &item.id))?;
    let validator_ids = unique_ids("validators", plan.validators.iter().map(|item| &item.id))?;
    let claim_ids = unique_ids("claims", plan.claims.iter().map(|item| &item.id))?;
    let control_ids = unique_ids(
        "negative_controls",
        plan.negative_controls.iter().map(|item| &item.id),
    )?;
    validate_bound_file("git_program", &plan.git_program)?;
    for subject in &plan.subjects {
        let repository = Path::new(&subject.repository);
        if !repository.is_absolute() || !repository.is_dir() {
            return Err(ToolError::invalid(format!(
                "subject {:?} repository must be an existing absolute directory",
                subject.id
            )));
        }
        require_git_oid("subject.git.commit", &subject.git.commit)?;
        require_git_oid("subject.git.tree", &subject.git.tree)?;
        validate_bound_file("subject.executable", &subject.executable)?;
        for lock in &subject.locks {
            validate_bound_file("subject.lock", lock)?;
        }
        validate_bound_file("subject.toolchain.program", &subject.toolchain.program)?;
        require_sha256(
            "subject.toolchain.stdout_sha256",
            &subject.toolchain.stdout_sha256,
        )?;
        require_nonempty("subject.toolchain.target", &subject.toolchain.target)?;
        require_nonempty("subject.toolchain.profile", &subject.toolchain.profile)?;
        match &subject.dirty {
            DirtyBinding::Clean { patch_sha256 } | DirtyBinding::Dirty { patch_sha256 } => {
                require_sha256("subject.dirty.patch_sha256", patch_sha256)?;
            }
            DirtyBinding::Unattested => {}
        }
        if subject.checkout == CheckoutBinding::ObjectOnly
            && !matches!(subject.dirty, DirtyBinding::Unattested)
        {
            return Err(ToolError::invalid(format!(
                "object-only subject {:?} must use dirty.kind=unattested",
                subject.id
            )));
        }
        if subject.checkout == CheckoutBinding::CurrentCheckout
            && matches!(subject.dirty, DirtyBinding::Unattested)
        {
            return Err(ToolError::invalid(format!(
                "current-checkout subject {:?} must attest clean or dirty state",
                subject.id
            )));
        }
    }
    for invocation in &plan.invocations {
        if !subject_ids.contains(invocation.subject_id.as_str()) {
            return Err(ToolError::invalid(format!(
                "invocation {:?} references unknown subject {:?}",
                invocation.id, invocation.subject_id
            )));
        }
        if invocation.case_id.is_empty() || invocation.argv.is_empty() {
            return Err(ToolError::invalid(format!(
                "invocation {:?} case_id and argv must not be empty",
                invocation.id
            )));
        }
        let cwd = Path::new(&invocation.cwd);
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(ToolError::invalid(format!(
                "invocation {:?} cwd must be an existing absolute directory",
                invocation.id
            )));
        }
        decode_hex(&invocation.stdin_hex, "invocation.stdin_hex")?;
        for topology in &invocation.input_topologies {
            validate_topology_pin(topology)?;
        }
        if let Some(output_root) = &invocation.output_root
            && !Path::new(output_root).is_absolute()
        {
            return Err(ToolError::invalid(format!(
                "invocation {:?} output_root must be absolute",
                invocation.id
            )));
        }
    }
    for validator in &plan.validators {
        if !invocation_ids.contains(validator.observed_invocation_id.as_str()) {
            return Err(ToolError::invalid(format!(
                "validator {:?} references unknown observed invocation",
                validator.id
            )));
        }
        let observed = plan
            .invocations
            .iter()
            .find(|item| item.id == validator.observed_invocation_id)
            .expect("validated observed invocation");
        let observed_subject = subject_plans[observed.subject_id.as_str()];
        if let ExpectedObservation::Invocation { invocation_id } = &validator.expected
            && !invocation_ids.contains(invocation_id.as_str())
        {
            return Err(ToolError::invalid(format!(
                "validator {:?} references unknown expected invocation",
                validator.id
            )));
        }
        if validator.authority == ValidatorAuthority::SecondImplementation
            && !matches!(validator.expected, ExpectedObservation::Invocation { .. })
        {
            return Err(ToolError::invalid(format!(
                "second_implementation validator {:?} must compare two invocations",
                validator.id
            )));
        }
        if validator.authority == ValidatorAuthority::SecondImplementation {
            let ExpectedObservation::Invocation { invocation_id } = &validator.expected else {
                unreachable!("second implementation shape checked above");
            };
            let expected = plan
                .invocations
                .iter()
                .find(|item| item.id == *invocation_id)
                .expect("validated expected invocation");
            let expected_subject = subject_plans[expected.subject_id.as_str()];
            if observed.id == expected.id
                || observed.subject_id == expected.subject_id
                || observed_subject.executable.sha256 == expected_subject.executable.sha256
            {
                return Err(ToolError::invalid(format!(
                    "second_implementation validator {:?} requires distinct invocations, subjects, and executable identities",
                    validator.id
                )));
            }
        }
        match validator.authority {
            ValidatorAuthority::SelfProduced | ValidatorAuthority::SecondImplementation => {
                if validator.authority_evidence.is_some() {
                    return Err(ToolError::invalid(format!(
                        "validator {:?} authority must not declare Git fixture evidence",
                        validator.id
                    )));
                }
            }
            ValidatorAuthority::IndependentFixture => {
                let Some(evidence) = &validator.authority_evidence else {
                    return Err(ToolError::invalid(format!(
                        "independent_fixture validator {:?} needs immutable Git fixture evidence and a literal expectation",
                        validator.id
                    )));
                };
                if !matches!(validator.expected, ExpectedObservation::Literal { .. })
                    || evidence.origin != AuthorityEvidenceOrigin::ProspectiveBaseline
                {
                    return Err(ToolError::invalid(format!(
                        "independent_fixture validator {:?} needs a prospective-baseline Git fixture and a literal expectation",
                        validator.id
                    )));
                }
                let Some(fixture_subject) = subject_plans.get(evidence.subject_id.as_str()) else {
                    return Err(ToolError::invalid(format!(
                        "independent_fixture validator {:?} references unknown fixture subject {:?}",
                        validator.id, evidence.subject_id
                    )));
                };
                if fixture_subject.id == observed_subject.id
                    || fixture_subject.repository != observed_subject.repository
                    || fixture_subject.git.commit == observed_subject.git.commit
                {
                    return Err(ToolError::invalid(format!(
                        "independent_fixture validator {:?} must bind a distinct earlier subject in the observed subject's repository",
                        validator.id
                    )));
                }
            }
            ValidatorAuthority::TrustedOracle | ValidatorAuthority::ReproducibleBuild => {
                if validator
                    .authority_evidence
                    .as_ref()
                    .is_none_or(|evidence| {
                        evidence.origin != AuthorityEvidenceOrigin::ExternalReceipt
                    })
                {
                    return Err(ToolError::invalid(format!(
                        "validator {:?} must bind evidence with external_receipt origin",
                        validator.id
                    )));
                }
            }
        }
        if let Some(evidence) = &validator.authority_evidence {
            if !subject_ids.contains(evidence.subject_id.as_str()) {
                return Err(ToolError::invalid(format!(
                    "validator {:?} fixture evidence references unknown subject {:?}",
                    validator.id, evidence.subject_id
                )));
            }
            require_git_oid("validator.authority_evidence.commit", &evidence.commit)?;
            require_sha256("validator.authority_evidence.sha256", &evidence.sha256)?;
            validate_git_relative_path(&evidence.path)?;
            if evidence
                .json_pointer
                .as_ref()
                .is_some_and(|pointer| !pointer.is_empty() && !pointer.starts_with('/'))
            {
                return Err(ToolError::invalid(
                    "authority evidence json_pointer must be empty or begin with '/'",
                ));
            }
        }
    }
    let consumed_invocations = plan
        .validators
        .iter()
        .flat_map(|validator| {
            std::iter::once(validator.observed_invocation_id.as_str()).chain(
                match &validator.expected {
                    ExpectedObservation::Invocation { invocation_id } => {
                        Some(invocation_id.as_str())
                    }
                    ExpectedObservation::Literal { .. } => None,
                },
            )
        })
        .collect::<BTreeSet<_>>();
    if consumed_invocations != invocation_ids {
        let unconsumed = invocation_ids
            .difference(&consumed_invocations)
            .copied()
            .collect::<Vec<_>>();
        return Err(ToolError::invalid(format!(
            "every invocation must be consumed by a validator; unconsumed={unconsumed:?}"
        )));
    }
    let mut claimed_control_ids = BTreeSet::new();
    let mut decision_validator_ids = BTreeSet::new();
    for claim in &plan.claims {
        if claim.description.is_empty()
            || claim.subject.subject_ids.is_empty()
            || claim.validator_ids.is_empty()
            || claim.negative_control_id.is_empty()
        {
            return Err(ToolError::invalid(format!(
                "claim {:?} needs a description, validators, and negative control",
                claim.id
            )));
        }
        let claim_subjects = unique_ids(
            "claim executable subjects",
            claim.subject.subject_ids.iter(),
        )?;
        if !claim_subjects.is_subset(&subject_ids) {
            return Err(ToolError::invalid(format!(
                "claim {:?} references an unknown executable subject",
                claim.id
            )));
        }
        if !claimed_control_ids.insert(claim.negative_control_id.as_str()) {
            return Err(ToolError::invalid(
                "every claim must own a distinct negative control",
            ));
        }
        if claim.closure != plan.closure {
            return Err(ToolError::invalid(format!(
                "claim {:?} closure must equal the execution plan closure in v1",
                claim.id
            )));
        }
        for validator in &claim.validator_ids {
            if !validator_ids.contains(validator.as_str()) {
                return Err(ToolError::invalid(format!(
                    "claim {:?} references unknown validator {validator:?}",
                    claim.id
                )));
            }
            decision_validator_ids.insert(validator.as_str());
        }
        if !control_ids.contains(claim.negative_control_id.as_str()) {
            return Err(ToolError::invalid(format!(
                "claim {:?} references unknown negative control {:?}",
                claim.id, claim.negative_control_id
            )));
        }
    }
    for control in &plan.negative_controls {
        if !claim_ids.contains(control.claim_id.as_str())
            || !validator_ids.contains(control.validator_id.as_str())
        {
            return Err(ToolError::invalid(format!(
                "negative control {:?} references an unknown claim or validator",
                control.id
            )));
        }
        let claim = plan
            .claims
            .iter()
            .find(|claim| claim.id == control.claim_id)
            .expect("validated claim reference");
        if claim.negative_control_id != control.id
            || claim.validator_ids.contains(&control.validator_id)
        {
            return Err(ToolError::invalid(
                "a negative control must link exactly to its claim and use a distinct validator",
            ));
        }
        decision_validator_ids.insert(control.validator_id.as_str());
    }
    if claimed_control_ids != control_ids {
        return Err(ToolError::invalid(
            "negative controls and claims must have bijective ownership",
        ));
    }
    if decision_validator_ids != validator_ids {
        let orphaned = validator_ids
            .difference(&decision_validator_ids)
            .copied()
            .collect::<Vec<_>>();
        return Err(ToolError::invalid(format!(
            "every validator must support a claim or its strict negative control; orphaned={orphaned:?}"
        )));
    }
    validate_claim_subject_reachability(plan)?;
    Ok(())
}

fn validate_claim_subject_reachability(plan: &ExecutionPlan) -> Result<(), ToolError> {
    let invocations: BTreeMap<_, _> = plan
        .invocations
        .iter()
        .map(|invocation| (invocation.id.as_str(), invocation))
        .collect();
    let validators: BTreeMap<_, _> = plan
        .validators
        .iter()
        .map(|validator| (validator.id.as_str(), validator))
        .collect();
    let controls: BTreeMap<_, _> = plan
        .negative_controls
        .iter()
        .map(|control| (control.id.as_str(), control))
        .collect();
    for claim in &plan.claims {
        let control = controls[claim.negative_control_id.as_str()];
        let validator_ids = claim
            .validator_ids
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(control.validator_id.as_str()));
        let mut reachable = BTreeSet::new();
        for id in validator_ids {
            let validator = validators[id];
            reachable.insert(
                invocations[validator.observed_invocation_id.as_str()]
                    .subject_id
                    .as_str(),
            );
            if let ExpectedObservation::Invocation { invocation_id } = &validator.expected {
                reachable.insert(invocations[invocation_id.as_str()].subject_id.as_str());
            }
        }
        let declared = claim
            .subject
            .subject_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if reachable != declared {
            return Err(ToolError::invalid(format!(
                "claim {:?} executable subjects must exactly equal subjects reachable from its support and control validators; declared={declared:?}, reachable={reachable:?}",
                claim.id
            )));
        }
    }
    Ok(())
}

fn empty_manifest(plan: &ExecutionPlan, plan_sha256: String) -> ExecutedEvidenceManifest {
    let mut nonclaims = plan.nonclaims.clone();
    nonclaims.extend([
        "Pinned Git objects, lockfiles, toolchain output, and executable bytes do not prove that the executable was built from the pinned source.".to_owned(),
        "Environment clearing does not bind the kernel, loader, filesystem implementation, hardware, time, network, or concurrently mutable external state.".to_owned(),
        "Typed topology v1 excludes permissions, ownership, timestamps, extended attributes, mount identity, and Windows reparse subtype.".to_owned(),
        "An accepted self-produced validator can support scoped repeatability only; it is not an independent correctness or external-runtime oracle.".to_owned(),
    ]);
    nonclaims.sort();
    nonclaims.dedup();
    ExecutedEvidenceManifest {
        schema_version: EXECUTED_MANIFEST_SCHEMA.to_owned(),
        manifest_id: plan.manifest_id.clone(),
        decision_id: plan.decision_id.clone(),
        plan_sha256,
        closure: plan.closure,
        status: ManifestStatus::Unknown,
        subject_receipts: Vec::new(),
        invocation_receipts: Vec::new(),
        validator_receipts: Vec::new(),
        claim_receipts: Vec::new(),
        negative_control_receipts: Vec::new(),
        counterexamples: Vec::new(),
        witness: None,
        nonclaims,
    }
}

fn reject_binding(
    mut manifest: ExecutedEvidenceManifest,
    witness: ManifestWitness,
) -> ExecutedEvidenceManifest {
    manifest.status = ManifestStatus::BindingRejected;
    manifest.witness = Some(witness);
    manifest
}

#[allow(clippy::too_many_lines)]
fn verify_subject(
    git_program: &BoundFile,
    subject: &SubjectPin,
) -> Result<Result<SubjectReceipt, ManifestWitness>, ToolError> {
    for (label, bound) in std::iter::once(("executable", &subject.executable))
        .chain(subject.locks.iter().map(|bound| ("lock", bound)))
        .chain(std::iter::once((
            "toolchain_program",
            &subject.toolchain.program,
        )))
    {
        if let Some(mut witness) = verify_bound_file(label, bound)? {
            witness.subject.clone_from(&subject.id);
            return Ok(Err(witness));
        }
    }
    let commit = run_git(
        git_program,
        &subject.repository,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", subject.git.commit),
        ],
    )?;
    if commit != subject.git.commit {
        return Ok(Err(binding_witness(
            "git_commit_mismatch",
            &subject.id,
            Some(&subject.git.commit),
            Some(&commit),
        )));
    }
    let tree = run_git(
        git_program,
        &subject.repository,
        &["rev-parse", &format!("{}^{{tree}}", subject.git.commit)],
    )?;
    if tree != subject.git.tree {
        return Ok(Err(binding_witness(
            "git_tree_mismatch",
            &subject.id,
            Some(&subject.git.tree),
            Some(&tree),
        )));
    }
    let dirty_patch_sha256 = if subject.checkout == CheckoutBinding::CurrentCheckout {
        let head = run_git(git_program, &subject.repository, &["rev-parse", "HEAD"])?;
        if head != subject.git.commit {
            return Ok(Err(binding_witness(
                "checkout_head_mismatch",
                &subject.id,
                Some(&subject.git.commit),
                Some(&head),
            )));
        }
        let (dirty, digest) = git_dirty_digest(git_program, &subject.repository)?;
        match &subject.dirty {
            DirtyBinding::Clean { patch_sha256 } => {
                if dirty || &digest != patch_sha256 {
                    return Ok(Err(binding_witness(
                        "clean_checkout_binding_mismatch",
                        &subject.id,
                        Some(patch_sha256),
                        Some(&digest),
                    )));
                }
                Some(digest)
            }
            DirtyBinding::Dirty { patch_sha256 } => {
                if !dirty || &digest != patch_sha256 {
                    return Ok(Err(binding_witness(
                        "dirty_checkout_binding_mismatch",
                        &subject.id,
                        Some(patch_sha256),
                        Some(&digest),
                    )));
                }
                Some(digest)
            }
            DirtyBinding::Unattested => None,
        }
    } else {
        None
    };
    let toolchain = run_pinned_command(
        &subject.toolchain.program,
        &subject.toolchain.args,
        Path::new(&subject.repository),
    )?;
    let toolchain_sha256 = sha256_hex(&toolchain.stdout);
    if !toolchain.status.success() || toolchain_sha256 != subject.toolchain.stdout_sha256 {
        return Ok(Err(binding_witness(
            "toolchain_identity_mismatch",
            &subject.id,
            Some(&subject.toolchain.stdout_sha256),
            Some(&toolchain_sha256),
        )));
    }
    Ok(Ok(SubjectReceipt {
        id: subject.id.clone(),
        repository: subject.repository.clone(),
        commit,
        tree,
        dirty_patch_sha256,
        executable_sha256: subject.executable.sha256.clone(),
        lock_sha256s: subject
            .locks
            .iter()
            .map(|bound| bound.sha256.clone())
            .collect(),
        toolchain_stdout_sha256: toolchain_sha256,
        build_attestation: subject.build_attestation,
    }))
}

fn execute_invocation(
    subject: &SubjectPin,
    invocation: &InvocationPlan,
) -> Result<Result<InvocationReceipt, ManifestWitness>, ToolError> {
    if invocation.argv.first() != Some(&subject.executable.path) {
        return Ok(Err(binding_witness(
            "argv_executable_mismatch",
            &invocation.id,
            Some(&subject.executable.path),
            invocation.argv.first().map(String::as_str),
        )));
    }
    let program_sha256_pre = sha256_file(Path::new(&subject.executable.path))?;
    if program_sha256_pre != subject.executable.sha256 {
        return Ok(Err(binding_witness(
            "executable_hash_mismatch",
            &invocation.id,
            Some(&subject.executable.sha256),
            Some(&program_sha256_pre),
        )));
    }
    let mut input_topologies = Vec::new();
    for pin in &invocation.input_topologies {
        let observed = snapshot_topology(Path::new(&pin.root))?;
        if observed.digest != pin.digest {
            return Ok(Err(binding_witness(
                "input_topology_mismatch",
                &invocation.id,
                Some(&pin.digest),
                Some(&observed.digest),
            )));
        }
        input_topologies.push(observed);
    }
    let stdin = decode_hex(&invocation.stdin_hex, "invocation.stdin_hex")?;
    let program_sha256_pre_spawn = sha256_file(Path::new(&subject.executable.path))?;
    if program_sha256_pre_spawn != program_sha256_pre {
        return Ok(Err(binding_witness(
            "executable_replaced_before_spawn",
            &invocation.id,
            Some(&program_sha256_pre),
            Some(&program_sha256_pre_spawn),
        )));
    }
    // ubs:ignore — operator-authored executable is SHA-256 pinned immediately before spawn; arbitrary execution is this evidence harness's explicit contract.
    let mut command = Command::new(&subject.executable.path);
    command
        .args(&invocation.argv[1..])
        .current_dir(&invocation.cwd)
        .env_clear()
        .envs(&invocation.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        ToolError::io(format!(
            "launch invocation {:?} with {}: {error}",
            invocation.id, subject.executable.path
        ))
    })?;
    if let Some(mut child_stdin) = child.stdin.take() {
        child_stdin.write_all(&stdin).map_err(|error| {
            ToolError::io(format!(
                "write invocation {:?} stdin: {error}",
                invocation.id
            ))
        })?;
    }
    let output = child.wait_with_output().map_err(|error| {
        ToolError::io(format!("wait for invocation {:?}: {error}", invocation.id))
    })?;
    let output_topology = invocation
        .output_root
        .as_ref()
        .map(|root| snapshot_topology(Path::new(root)))
        .transpose()?;
    Ok(Ok(InvocationReceipt {
        id: invocation.id.clone(),
        case_id: invocation.case_id.clone(),
        subject_id: invocation.subject_id.clone(),
        argv: invocation.argv.clone(),
        cwd: invocation.cwd.clone(),
        environment: invocation.environment.clone(),
        stdin_sha256: sha256_hex(&stdin),
        input_topologies,
        program_sha256_pre,
        program_sha256_pre_spawn,
        executed: true,
        exit_code: output.status.code(),
        stdout_sha256: sha256_hex(&output.stdout),
        stdout_len: output.stdout.len() as u64,
        stderr_sha256: sha256_hex(&output.stderr),
        stderr_len: output.stderr.len() as u64,
        output_topology,
    }))
}

fn run_validator(
    plan: &ValidatorPlan,
    receipts: &BTreeMap<&str, &InvocationReceipt>,
    authority_evidence: Option<GitFixtureEvidenceReceipt>,
) -> Result<ValidatorReceipt, ToolError> {
    let observed_receipt = receipts[plan.observed_invocation_id.as_str()];
    let observed = observation_value(plan.kind, observed_receipt)?;
    let expected = match &plan.expected {
        ExpectedObservation::Invocation { invocation_id } => {
            observation_value(plan.kind, receipts[invocation_id.as_str()])?
        }
        ExpectedObservation::Literal { value } => value.clone(),
    };
    let status = if observed == expected {
        ValidatorStatus::Accepted
    } else {
        ValidatorStatus::Refuted
    };
    Ok(ValidatorReceipt {
        id: plan.id.clone(),
        kind: plan.kind,
        authority: plan.authority,
        observed_invocation_id: plan.observed_invocation_id.clone(),
        expected_source: plan.expected.clone(),
        authority_evidence,
        observed: observed.clone(),
        expected: expected.clone(),
        status,
        witness: (status == ValidatorStatus::Refuted)
            .then(|| format!("observed {observed:?}, expected {expected:?}")),
    })
}

fn verify_validator_authority(
    git_program: &BoundFile,
    validator: &ValidatorPlan,
    subjects: &BTreeMap<&str, &SubjectPin>,
    receipts: &BTreeMap<&str, &InvocationReceipt>,
) -> Result<Result<Option<GitFixtureEvidenceReceipt>, ManifestWitness>, ToolError> {
    let Some(evidence) = &validator.authority_evidence else {
        return Ok(Ok(None));
    };
    let subject = subjects[evidence.subject_id.as_str()];
    if let Some(witness) = verify_prospective_fixture_origin(
        git_program,
        validator,
        subject,
        evidence,
        subjects,
        receipts,
    )? {
        return Ok(Err(witness));
    }
    if subject.git.commit != evidence.commit {
        return Ok(Err(binding_witness(
            "authority_evidence_commit_mismatch",
            &validator.id,
            Some(&evidence.commit),
            Some(&subject.git.commit),
        )));
    }
    let bytes = git_show_blob(
        git_program,
        &subject.repository,
        &evidence.commit,
        &evidence.path,
    )?;
    let observed_sha256 = sha256_hex(&bytes);
    if observed_sha256 != evidence.sha256 {
        return Ok(Err(binding_witness(
            "authority_evidence_hash_mismatch",
            &validator.id,
            Some(&evidence.sha256),
            Some(&observed_sha256),
        )));
    }
    let observed_value = if let Some(pointer) = &evidence.json_pointer {
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
            ToolError::invalid(format!(
                "authority evidence {} at {} is not JSON: {error}",
                evidence.path, evidence.commit
            ))
        })?;
        let selected = value.pointer(pointer).ok_or_else(|| {
            ToolError::invalid(format!(
                "authority evidence pointer {pointer:?} does not exist in {}",
                evidence.path
            ))
        })?;
        match selected {
            serde_json::Value::String(value) => value.clone(),
            serde_json::Value::Number(value) => value.to_string(),
            serde_json::Value::Bool(value) => value.to_string(),
            serde_json::Value::Null => "null".to_owned(),
            _ => serde_json::to_string(selected).map_err(|error| {
                ToolError::internal(format!("serialize authority evidence value: {error}"))
            })?,
        }
    } else {
        String::from_utf8(bytes).map_err(|error| {
            ToolError::invalid(format!(
                "authority evidence {} is not UTF-8: {error}",
                evidence.path
            ))
        })?
    };
    let expected = match &validator.expected {
        ExpectedObservation::Literal { value } => value,
        ExpectedObservation::Invocation { invocation_id } => {
            let receipt = receipts[invocation_id.as_str()];
            return Ok(Err(binding_witness(
                "authority_evidence_not_literal",
                &validator.id,
                None,
                Some(&receipt.id),
            )));
        }
    };
    if &observed_value != expected {
        return Ok(Err(binding_witness(
            "authority_evidence_value_mismatch",
            &validator.id,
            Some(expected),
            Some(&observed_value),
        )));
    }
    Ok(Ok(Some(GitFixtureEvidenceReceipt {
        subject_id: evidence.subject_id.clone(),
        commit: evidence.commit.clone(),
        path: evidence.path.clone(),
        sha256: observed_sha256,
        origin: evidence.origin,
        json_pointer: evidence.json_pointer.clone(),
        observed_value,
    })))
}

fn verify_prospective_fixture_origin(
    git_program: &BoundFile,
    validator: &ValidatorPlan,
    fixture_subject: &SubjectPin,
    evidence: &GitFixtureEvidence,
    subjects: &BTreeMap<&str, &SubjectPin>,
    receipts: &BTreeMap<&str, &InvocationReceipt>,
) -> Result<Option<ManifestWitness>, ToolError> {
    if evidence.origin != AuthorityEvidenceOrigin::ProspectiveBaseline {
        return Ok(None);
    }
    let observed = receipts[validator.observed_invocation_id.as_str()];
    let observed_subject = subjects[observed.subject_id.as_str()];
    if git_is_strict_ancestor(
        git_program,
        &fixture_subject.repository,
        &evidence.commit,
        &observed_subject.git.commit,
    )? {
        Ok(None)
    } else {
        Ok(Some(binding_witness(
            "authority_evidence_not_prospective",
            &validator.id,
            Some("strict Git ancestor of the observed subject"),
            Some(&evidence.commit),
        )))
    }
}

fn observation_value(
    kind: ValidatorKind,
    receipt: &InvocationReceipt,
) -> Result<String, ToolError> {
    if !receipt.executed {
        return Err(ToolError::invalid(format!(
            "invocation receipt {:?} was not executed",
            receipt.id
        )));
    }
    match kind {
        ValidatorKind::ExactExit => Ok(receipt
            .exit_code
            .map_or_else(|| "signal".to_owned(), |code| code.to_string())),
        ValidatorKind::ExactStdout => Ok(receipt.stdout_sha256.clone()),
        ValidatorKind::ExactStderr => Ok(receipt.stderr_sha256.clone()),
        ValidatorKind::ExactTopology => receipt
            .output_topology
            .as_ref()
            .map(|topology| topology.digest.clone())
            .ok_or_else(|| {
                ToolError::invalid(format!(
                    "validator requires output topology from invocation {:?}",
                    receipt.id
                ))
            }),
    }
}

fn claim_authority_error(
    plan: &ExecutionPlan,
    claim: &ExecutedClaimPlan,
    validators: &[&ValidatorReceipt],
) -> Option<String> {
    let all_self = validators
        .iter()
        .all(|receipt| receipt.authority == ValidatorAuthority::SelfProduced);
    if all_self {
        return Some(
            "self-produced validators may measure repeatability but cannot by themselves support a positive semantic claim".to_owned(),
        );
    }
    if matches!(
        claim.epistemic_class,
        EpistemicClass::TheoremInstance | EpistemicClass::ExactCheckedModel
    ) {
        return Some(
            "executed empirical validators cannot establish theorem_instance or exact_checked_model"
                .to_owned(),
        );
    }
    if matches!(
        claim.closure,
        ClosureKind::ExhaustiveFinite | ClosureKind::SoundOverapproximation
    ) {
        return Some(
            "executed empirical validators cannot establish exhaustive_finite or sound_overapproximation closure"
                .to_owned(),
        );
    }
    if claim.kind == ClaimKind::SourceCorrespondence {
        let _ = plan;
        return Some(
            "executed-evidence v1 records build pins but cannot establish source_correspondence"
                .to_owned(),
        );
    }
    None
}

fn audited_claim_authority_error(
    claim: &ExecutedClaimReceipt,
    validators: &[&ValidatorReceipt],
) -> Option<String> {
    let all_self = validators
        .iter()
        .all(|receipt| receipt.authority == ValidatorAuthority::SelfProduced);
    if all_self {
        return Some(
            "self-produced validators cannot support a positive semantic claim".to_owned(),
        );
    }
    if matches!(
        claim.epistemic_class,
        EpistemicClass::TheoremInstance | EpistemicClass::ExactCheckedModel
    ) || matches!(
        claim.closure,
        ClosureKind::ExhaustiveFinite | ClosureKind::SoundOverapproximation
    ) || claim.kind == ClaimKind::SourceCorrespondence
    {
        return Some("empirical executed-manifest authority ceiling exceeded".to_owned());
    }
    None
}

fn audit_subjects(
    receipts: &[SubjectReceipt],
) -> Result<BTreeMap<&str, &SubjectReceipt>, ToolError> {
    let mut subjects = BTreeMap::new();
    for receipt in receipts {
        if receipt.id.is_empty()
            || receipt.repository.is_empty()
            || subjects.insert(receipt.id.as_str(), receipt).is_some()
        {
            return Ok(BTreeMap::new());
        }
        require_git_oid("subject_receipt.commit", &receipt.commit)?;
        require_git_oid("subject_receipt.tree", &receipt.tree)?;
        require_sha256(
            "subject_receipt.executable_sha256",
            &receipt.executable_sha256,
        )?;
        require_sha256(
            "subject_receipt.toolchain_stdout_sha256",
            &receipt.toolchain_stdout_sha256,
        )?;
        for lock in &receipt.lock_sha256s {
            require_sha256("subject_receipt.lock_sha256", lock)?;
        }
        if let Some(dirty) = &receipt.dirty_patch_sha256 {
            require_sha256("subject_receipt.dirty_patch_sha256", dirty)?;
        }
    }
    Ok(subjects)
}

fn audit_invocations<'a>(
    receipts: &'a [InvocationReceipt],
    subjects: &BTreeMap<&str, &SubjectReceipt>,
) -> Result<BTreeMap<&'a str, &'a InvocationReceipt>, ToolError> {
    if subjects.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut invocations = BTreeMap::new();
    for receipt in receipts {
        let Some(subject) = subjects.get(receipt.subject_id.as_str()) else {
            return Ok(BTreeMap::new());
        };
        if receipt.id.is_empty()
            || receipt.argv.is_empty()
            || invocations.insert(receipt.id.as_str(), receipt).is_some()
            || receipt.program_sha256_pre != receipt.program_sha256_pre_spawn
            || receipt.program_sha256_pre != subject.executable_sha256
        {
            return Ok(BTreeMap::new());
        }
        for value in [
            &receipt.stdin_sha256,
            &receipt.program_sha256_pre,
            &receipt.program_sha256_pre_spawn,
            &receipt.stdout_sha256,
            &receipt.stderr_sha256,
        ] {
            require_sha256("invocation receipt digest", value)?;
        }
        for topology in receipt
            .input_topologies
            .iter()
            .chain(receipt.output_topology.iter())
        {
            audit_topology(topology)?;
        }
    }
    Ok(invocations)
}

fn audit_topology(topology: &TopologySnapshot) -> Result<(), ToolError> {
    if topology.algorithm != TOPOLOGY_ALGORITHM || !Path::new(&topology.root).is_absolute() {
        return Err(ToolError::invalid(
            "topology receipt has an unsupported algorithm or non-absolute root",
        ));
    }
    require_sha256("topology.digest", &topology.digest)?;
    let mut paths = BTreeSet::new();
    for node in &topology.nodes {
        if !paths.insert(node.relative_path.as_str()) {
            return Err(ToolError::invalid(
                "topology receipt contains duplicate relative paths",
            ));
        }
        match node.kind {
            TopologyNodeKind::File => {
                if node.byte_len.is_none() || node.link_target.is_some() || node.dangling.is_some()
                {
                    return Err(ToolError::invalid("malformed file topology node"));
                }
                require_sha256(
                    "topology file sha256",
                    node.sha256.as_deref().unwrap_or_default(),
                )?;
            }
            TopologyNodeKind::Directory => {
                if node.sha256.is_some()
                    || node.byte_len.is_some()
                    || node.link_target.is_some()
                    || node.dangling.is_some()
                {
                    return Err(ToolError::invalid("malformed directory topology node"));
                }
            }
            TopologyNodeKind::Symlink => {
                if node.sha256.is_some()
                    || node.byte_len.is_some()
                    || node.link_target.is_none()
                    || node.dangling.is_none()
                {
                    return Err(ToolError::invalid("malformed symlink topology node"));
                }
            }
        }
    }
    let expected = topology_digest(&topology.nodes)?;
    if topology.digest != expected {
        return Err(ToolError::invalid(
            "topology receipt digest does not match its typed nodes",
        ));
    }
    Ok(())
}

fn audit_validators<'a>(
    receipts: &'a [ValidatorReceipt],
    invocations: &BTreeMap<&str, &InvocationReceipt>,
    subjects: &[SubjectReceipt],
) -> Result<BTreeMap<&'a str, &'a ValidatorReceipt>, ToolError> {
    let mut validators = BTreeMap::new();
    for receipt in receipts {
        if receipt.id.is_empty() || validators.insert(receipt.id.as_str(), receipt).is_some() {
            return Ok(BTreeMap::new());
        }
        let Some(observed_invocation) = invocations.get(receipt.observed_invocation_id.as_str())
        else {
            return Ok(BTreeMap::new());
        };
        match receipt.authority {
            ValidatorAuthority::SelfProduced | ValidatorAuthority::SecondImplementation => {
                if receipt.authority_evidence.is_some() {
                    return Ok(BTreeMap::new());
                }
            }
            ValidatorAuthority::IndependentFixture
            | ValidatorAuthority::TrustedOracle
            | ValidatorAuthority::ReproducibleBuild => {
                let Some(evidence) = &receipt.authority_evidence else {
                    return Ok(BTreeMap::new());
                };
                let Some(subject) = subjects.iter().find(|item| item.id == evidence.subject_id)
                else {
                    return Ok(BTreeMap::new());
                };
                if subject.commit != evidence.commit || evidence.observed_value != receipt.expected
                {
                    return Ok(BTreeMap::new());
                }
                match receipt.authority {
                    ValidatorAuthority::IndependentFixture => {
                        let Some(observed_subject) = subjects
                            .iter()
                            .find(|item| item.id == observed_invocation.subject_id)
                        else {
                            return Ok(BTreeMap::new());
                        };
                        if evidence.origin != AuthorityEvidenceOrigin::ProspectiveBaseline
                            || subject.id == observed_subject.id
                            || subject.repository != observed_subject.repository
                            || subject.commit == observed_subject.commit
                        {
                            return Ok(BTreeMap::new());
                        }
                    }
                    ValidatorAuthority::TrustedOracle | ValidatorAuthority::ReproducibleBuild => {
                        if evidence.origin != AuthorityEvidenceOrigin::ExternalReceipt {
                            return Ok(BTreeMap::new());
                        }
                    }
                    ValidatorAuthority::SelfProduced | ValidatorAuthority::SecondImplementation => {
                        unreachable!()
                    }
                }
                require_git_oid("validator authority receipt commit", &evidence.commit)?;
                require_sha256("validator authority receipt sha256", &evidence.sha256)?;
                validate_git_relative_path(&evidence.path)?;
            }
        }
        let observed = observation_value(receipt.kind, observed_invocation)?;
        let expected = match &receipt.expected_source {
            ExpectedObservation::Invocation { invocation_id } => {
                let Some(expected_invocation) = invocations.get(invocation_id.as_str()) else {
                    return Ok(BTreeMap::new());
                };
                observation_value(receipt.kind, expected_invocation)?
            }
            ExpectedObservation::Literal { value } => value.clone(),
        };
        if receipt.observed != observed || receipt.expected != expected {
            return Ok(BTreeMap::new());
        }
        let expected_status = if observed == expected {
            ValidatorStatus::Accepted
        } else {
            ValidatorStatus::Refuted
        };
        if !matches!(
            receipt.status,
            ValidatorStatus::Unknown | ValidatorStatus::NotRun
        ) && receipt.status != expected_status
        {
            return Ok(BTreeMap::new());
        }
    }
    Ok(validators)
}

fn audit_negative_controls<'a>(
    receipts: &'a [NegativeControlReceipt],
    validators: &BTreeMap<&str, &ValidatorReceipt>,
) -> BTreeMap<&'a str, &'a NegativeControlReceipt> {
    if validators.is_empty() {
        return BTreeMap::new();
    }
    let mut controls = BTreeMap::new();
    for receipt in receipts {
        let Some(validator) = validators.get(receipt.validator_id.as_str()) else {
            return BTreeMap::new();
        };
        if receipt.id.is_empty()
            || receipt.claim_id.is_empty()
            || controls.insert(receipt.id.as_str(), receipt).is_some()
            || receipt.observed_status != validator.status
            || receipt.passed != (receipt.observed_status == ValidatorStatus::Refuted)
        {
            return BTreeMap::new();
        }
    }
    controls
}

fn claim_receipt(
    claim: &ExecutedClaimPlan,
    verdict: ClaimVerdict,
    negative_control_passed: bool,
) -> ExecutedClaimReceipt {
    ExecutedClaimReceipt {
        id: claim.id.clone(),
        subject: claim.subject.clone(),
        kind: claim.kind,
        epistemic_class: claim.epistemic_class,
        closure: claim.closure,
        verdict,
        validator_ids: claim.validator_ids.clone(),
        negative_control_id: claim.negative_control_id.clone(),
        negative_control_passed,
    }
}

fn run_git(git_program: &BoundFile, repository: &str, args: &[&str]) -> Result<String, ToolError> {
    let mut command_args = vec!["-C", repository];
    command_args.extend_from_slice(args);
    let output = run_pinned_command(git_program, &command_args, Path::new(repository))?;
    if !output.status.success() {
        return Err(ToolError::invalid(format!(
            "git command {:?} failed for {}: {}",
            args,
            repository,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| ToolError::invalid(format!("git output was not UTF-8: {error}")))
}

fn git_is_strict_ancestor(
    git_program: &BoundFile,
    repository: &str,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, ToolError> {
    if ancestor == descendant {
        return Ok(false);
    }
    let output = run_pinned_command(
        git_program,
        &[
            "-C",
            repository,
            "merge-base",
            "--is-ancestor",
            ancestor,
            descendant,
        ],
        Path::new(repository),
    )?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(ToolError::invalid(format!(
            "git merge-base --is-ancestor failed for {ancestor} and {descendant}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
    }
}

fn git_show_blob(
    git_program: &BoundFile,
    repository: &str,
    commit: &str,
    path: &str,
) -> Result<Vec<u8>, ToolError> {
    validate_git_relative_path(path)?;
    let object = format!("{commit}:{path}");
    let output = run_pinned_command(
        git_program,
        &["-C", repository, "show", object.as_str()],
        Path::new(repository),
    )?;
    if !output.status.success() {
        return Err(ToolError::invalid(format!(
            "git show failed for {object}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn validate_git_relative_path(path: &str) -> Result<(), ToolError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(ToolError::invalid(
            "Git fixture evidence path must be a non-empty repository-relative path without parent traversal",
        ));
    }
    Ok(())
}

fn run_pinned_command(
    program: &BoundFile,
    args: &[impl AsRef<std::ffi::OsStr>],
    cwd: &Path,
) -> Result<std::process::Output, ToolError> {
    let observed = sha256_file(Path::new(&program.path))?;
    if observed != program.sha256 {
        return Err(ToolError::invalid(format!(
            "pinned program {} hash mismatch: expected {}, observed {observed}",
            program.path, program.sha256
        )));
    }
    // ubs:ignore — operator-authored tool is SHA-256 pinned immediately before spawn; arbitrary execution is this evidence harness's explicit contract.
    Command::new(&program.path)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .output()
        .map_err(|error| {
            ToolError::io(format!(
                "execute pinned program {} in {}: {error}",
                program.path,
                cwd.display()
            ))
        })
}

fn git_dirty_digest(
    git_program: &BoundFile,
    repository: &str,
) -> Result<(bool, String), ToolError> {
    let diff = run_pinned_command(
        git_program,
        &[
            "-C",
            repository,
            "diff",
            "--binary",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=none",
            "HEAD",
        ],
        Path::new(repository),
    )?;
    if !diff.status.success() {
        return Err(ToolError::invalid(format!(
            "git diff failed for {repository}: {}",
            String::from_utf8_lossy(&diff.stderr).trim()
        )));
    }
    let untracked = run_pinned_command(
        git_program,
        &[
            "-C",
            repository,
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        Path::new(repository),
    )?;
    if !untracked.status.success() {
        return Err(ToolError::invalid(format!(
            "git ls-files failed for {repository}: {}",
            String::from_utf8_lossy(&untracked.stderr).trim()
        )));
    }
    let mut hasher = Sha256::new();
    hasher.update(b"kerf.git-dirty.v1\0");
    hasher.update((diff.stdout.len() as u64).to_le_bytes());
    hasher.update(&diff.stdout);
    let mut untracked_paths = untracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .map(str::to_owned)
                .map_err(|error| {
                    ToolError::invalid(format!("untracked Git path was not UTF-8: {error}"))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    untracked_paths.sort();
    for relative in &untracked_paths {
        let path = Path::new(repository).join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            ToolError::io(format!(
                "inspect untracked path {}: {error}",
                path.display()
            ))
        })?;
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        if metadata.file_type().is_symlink() {
            hasher.update([2]);
            let target = fs::read_link(&path).map_err(|error| {
                ToolError::io(format!(
                    "read untracked symlink {}: {error}",
                    path.display()
                ))
            })?;
            let target = target.to_str().ok_or_else(|| {
                ToolError::invalid(format!(
                    "untracked symlink target {} is not UTF-8",
                    path.display()
                ))
            })?;
            hasher.update((target.len() as u64).to_le_bytes());
            hasher.update(target.as_bytes());
        } else if metadata.is_file() {
            hasher.update([0]);
            let bytes = fs::read(&path).map_err(|error| {
                ToolError::io(format!("read untracked file {}: {error}", path.display()))
            })?;
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        } else {
            hasher.update([1]);
        }
    }
    let dirty = !diff.stdout.is_empty() || !untracked_paths.is_empty();
    Ok((dirty, format!("{:x}", hasher.finalize())))
}

fn verify_bound_file(label: &str, bound: &BoundFile) -> Result<Option<ManifestWitness>, ToolError> {
    let observed = sha256_file(Path::new(&bound.path))?;
    Ok((observed != bound.sha256).then(|| {
        binding_witness(
            &format!("{label}_hash_mismatch"),
            label,
            Some(&bound.sha256),
            Some(&observed),
        )
    }))
}

fn validate_bound_file(label: &str, bound: &BoundFile) -> Result<(), ToolError> {
    let path = Path::new(&bound.path);
    if !path.is_absolute() || !path.is_file() {
        return Err(ToolError::invalid(format!(
            "{label}.path {:?} must be an existing absolute file",
            bound.path
        )));
    }
    require_sha256(&format!("{label}.sha256"), &bound.sha256)
}

fn validate_topology_pin(pin: &TopologyPin) -> Result<(), ToolError> {
    if !Path::new(&pin.root).is_absolute() {
        return Err(ToolError::invalid("input topology root must be absolute"));
    }
    require_sha256("input topology digest", &pin.digest)
}

fn sha256_file(path: &Path) -> Result<String, ToolError> {
    let file = File::open(path)
        .map_err(|error| ToolError::io(format!("open {}: {error}", path.display())))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| ToolError::io(format!("read {}: {error}", path.display())))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn canonical_sha256<T: Serialize>(value: &T, label: &str) -> Result<String, ToolError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| ToolError::internal(format!("serialize {label}: {error}")))
}

fn unique_ids<'a>(
    field: &str,
    ids: impl Iterator<Item = &'a String>,
) -> Result<BTreeSet<&'a str>, ToolError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if id.is_empty() {
            return Err(ToolError::invalid(format!("{field} IDs must not be empty")));
        }
        if !unique.insert(id.as_str()) {
            return Err(ToolError::invalid(format!(
                "{field} contains duplicate ID {id:?}"
            )));
        }
    }
    Ok(unique)
}

fn require_nonempty(field: &str, value: &str) -> Result<(), ToolError> {
    if value.is_empty() {
        return Err(ToolError::invalid(format!("{field} must not be empty")));
    }
    Ok(())
}

fn require_git_oid(field: &str, value: &str) -> Result<(), ToolError> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ToolError::invalid(format!(
            "{field} must be exactly 40 hexadecimal characters"
        )));
    }
    Ok(())
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

fn binding_witness(
    kind: &str,
    subject: &str,
    expected: Option<&str>,
    observed: Option<&str>,
) -> ManifestWitness {
    ManifestWitness {
        kind: kind.to_owned(),
        subject: subject.to_owned(),
        expected: expected.map(str::to_owned),
        observed: observed.map(str::to_owned),
    }
}

fn decode_hex(value: &str, field: &str) -> Result<Vec<u8>, ToolError> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ToolError::invalid(format!(
            "{field} must be an even-length hexadecimal string"
        )));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)
                .map_err(|error| ToolError::internal(format!("decode {field}: {error}")))?;
            u8::from_str_radix(pair, 16)
                .map_err(|error| ToolError::invalid(format!("decode {field}: {error}")))
        })
        .collect()
}
