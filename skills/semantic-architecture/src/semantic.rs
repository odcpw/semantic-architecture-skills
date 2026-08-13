//! Evidence-bearing semantic models above the exact finite Kerf kernel.

use kerf::candidates::{CandidateInput, MigrationGraphInput};
use kerf::cert::CertificateInput;
use kerf::factor::FactorInput;
use kerf::factor_search::FactorSearchInput;
use kerf::lts::{CompareInput, LtsInput};
use kerf::morphism::MorphismInput;
use kerf::recompose::RecomposeInput;
use kerf::report::{ToolError, sha256_hex};
use kerf::transport::TransportInput;
use serde::de::{DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Formatter;

pub const SEMANTIC_MODEL_SCHEMA: &str = "kerf.semantic-model.v1";
pub const WORK_GRAPH_SCHEMA: &str = "kerf.semantic-work-graph.v1";
pub const PROJECTION_MANIFEST_SCHEMA: &str = "kerf.projection-manifest.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticModel {
    pub schema_version: String,
    pub model_id: String,
    pub subjects: Vec<Subject>,
    pub scopes: Vec<Scope>,
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub closures: Vec<ClosureClaim>,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub work_items: Vec<DeclaredWorkItem>,
    #[serde(default)]
    pub projections: Vec<KerfProjection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub id: String,
    pub kind: SubjectKind,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub build_identity: Option<String>,
    #[serde(default)]
    pub runtime_environment: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Intent,
    Implementation,
    Candidate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub id: String,
    pub subject: String,
    #[serde(default)]
    pub contexts: Vec<String>,
    #[serde(default)]
    pub workloads: Vec<String>,
    #[serde(default)]
    pub horizon: Option<String>,
    #[serde(default)]
    pub required_reality_contact: Option<RealityContact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entity {
    pub id: String,
    pub subject: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: EntityKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntityKind {
    Requirement {
        modality: RequirementModality,
        #[serde(default)]
        acceptance_claims: Vec<String>,
    },
    Carrier {
        fact: String,
        #[serde(default)]
        authority: Option<String>,
        #[serde(default)]
        identity: Option<String>,
        #[serde(default)]
        lifetime: Option<String>,
        #[serde(default)]
        reset: Option<String>,
    },
    Authority {
        protocol: AuthorityProtocol,
        writers: Vec<String>,
        #[serde(default)]
        consistency_contract: Option<String>,
    },
    DerivedView {
        source: String,
        #[serde(default)]
        validity: Option<String>,
        #[serde(default)]
        invalidated_by: Vec<String>,
    },
    Action,
    Transition,
    Observation {
        #[serde(default)]
        oracle_evidence: Vec<String>,
    },
    Effect {
        #[serde(default)]
        intent_carrier: Option<String>,
        #[serde(default)]
        receipt_observation: Option<String>,
        #[serde(default)]
        reconciliation_transition: Option<String>,
    },
    Port {
        direction: PortDirection,
        semantic_type: String,
    },
    Contract {
        contract_kind: ContractKind,
    },
    Actor {
        actor_kind: String,
    },
    Implementation {
        locator: String,
    },
    Invariant,
    Resource {
        resource_kind: String,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequirementModality {
    Must,
    Should,
    May,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityProtocol {
    SingleWriter,
    ReplicatedConsensus,
    TransactionalMultiWriter,
    EventSourced,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    Api,
    Abi,
    Schema,
    Persistence,
    Security,
    Timing,
    Failure,
    Consistency,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub id: String,
    pub kind: RelationKind,
    pub tails: Vec<String>,
    pub heads: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub subject: String,
    pub kind: BindingKind,
    pub artifact_sha256: String,
    pub locator: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub alphabet: Vec<AlphabetBinding>,
    #[serde(default)]
    pub normalization: Vec<String>,
    #[serde(default)]
    pub session: Option<SessionProtocol>,
    #[serde(default)]
    pub negative_controls: Vec<NegativeControl>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    SourceArtifact,
    ExecutableProtocol,
    RuntimeProtocol,
    DataSchema,
    ManualAssertion,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlphabetBinding {
    pub role: AlphabetRole,
    pub model_symbol: String,
    pub concrete_predicate: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AlphabetRole {
    Action,
    Observation,
    Emission,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionProtocol {
    pub open: String,
    pub reset: String,
    pub close: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControl {
    pub id: String,
    pub intervention: String,
    pub expected_rejection: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    RequiresObservation,
    RealizedBy,
    Reads,
    Writes,
    Emits,
    HappensBefore,
    DerivesFrom,
    Invalidates,
    Implements,
    CorrespondsTo,
    Decomposes,
    Refines,
    Calls,
    Imports,
    BuildInvalidates,
    FlowsOver,
    ContendsOn,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub subject: String,
    pub binding: String,
    pub kind: EvidenceKind,
    pub acquisition: Acquisition,
    pub sha256: String,
    pub source: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub derived_from: Vec<String>,
    #[serde(default)]
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Source,
    Runtime,
    Test,
    Specification,
    History,
    Build,
    Manual,
    Generated,
    Proof,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Acquisition {
    Observed,
    Intervened,
    Declared,
    Inferred,
    Derived,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RealityContact {
    None,
    DeclaredArtifact,
    Observed,
    Intervened,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Coverage {
    pub contexts: Vec<String>,
    pub workloads: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureClaim {
    pub id: String,
    pub subject: String,
    pub scope: String,
    pub domain: String,
    pub kind: ClosureKind,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClosureKind {
    ExhaustiveFinite,
    SoundOverapproximation,
    CompleteForDeclaredCorpus,
    ObservedOnly,
    Sampled,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,
    pub subject: String,
    pub scope: String,
    pub kind: ClaimKind,
    pub statement: String,
    pub epistemic_class: EpistemicClass,
    pub verdict: ClaimVerdict,
    #[serde(default)]
    pub subjects: Vec<String>,
    #[serde(default)]
    pub support: Vec<String>,
    #[serde(default)]
    pub refutation: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub checker: Option<CheckerReceipt>,
    #[serde(default)]
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    RequirementSatisfied,
    Equivalent,
    Refines,
    Factor,
    Morphism,
    Isomorphism,
    Consistent,
    Invariant,
    TraceConformant,
    SourceCorrespondence,
    TopologyRewrite,
    SemanticRewrite,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicClass {
    TheoremInstance,
    ExactCheckedModel,
    BoundedCheck,
    EvidenceInference,
    Heuristic,
    Metaphor,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerdict {
    Supported,
    Refuted,
    Conflicted,
    Unknown,
    Unsupported,
    Stale,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckerReceipt {
    pub tool: String,
    pub operation: String,
    pub input_sha256: String,
    pub result_sha256: String,
    pub status: CheckerStatus,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckerStatus {
    Accepted,
    Refuted,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredWorkItem {
    pub id: String,
    pub intent: String,
    #[serde(default)]
    pub changes: Vec<String>,
    #[serde(default)]
    pub satisfies: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub evidence_gates: Vec<String>,
    #[serde(default)]
    pub cleanup_obligation: Option<String>,
    #[serde(default)]
    pub external_binding: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KerfProjection {
    pub id: String,
    pub subject: String,
    pub scope: String,
    pub operation: KerfOperation,
    pub lowering_version: String,
    pub target_input: Value,
    pub scope_entities: Vec<String>,
    #[serde(default)]
    pub entity_map: Vec<ProjectionMap>,
    #[serde(default)]
    pub relation_map: Vec<ProjectionMap>,
    #[serde(default)]
    pub omitted_entities: Vec<Omission>,
    #[serde(default)]
    pub omitted_relations: Vec<Omission>,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub closure_ids: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    pub source_claim_ceiling: SourceClaimCeiling,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionMap {
    pub source: String,
    pub target_pointer: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Omission {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KerfOperation {
    ValidateLts,
    Quotient,
    Distinguish,
    Factor,
    SearchFactors,
    Recompose,
    CheckMorphism,
    VerifyCertificate,
    ValidateCandidates,
    ValidateMigration,
    Pareto,
    ScoreRoutes,
}

impl KerfOperation {
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            Self::ValidateLts => "validate-lts",
            Self::Quotient => "quotient",
            Self::Distinguish => "distinguish",
            Self::Factor => "factor",
            Self::SearchFactors => "search-factors",
            Self::Recompose => "recompose",
            Self::CheckMorphism => "check-morphism",
            Self::VerifyCertificate => "verify-certificate",
            Self::ValidateCandidates => "validate-candidates",
            Self::ValidateMigration => "validate-migration",
            Self::Pareto => "pareto",
            Self::ScoreRoutes => "score-routes",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceClaimCeiling {
    BoundedCheck,
    EvidenceInference,
    Heuristic,
}

#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub algorithm: &'static str,
    pub model_sha256: String,
    pub structurally_valid: bool,
    pub subject_count: usize,
    pub entity_count: usize,
    pub relation_count: usize,
    pub binding_count: usize,
    pub claim_count: usize,
    pub finding_counts: BTreeMap<&'static str, usize>,
    pub findings: Vec<Finding>,
    pub claim_groundings: Vec<ClaimGrounding>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimGrounding {
    pub claim_id: String,
    pub formal_class: EpistemicClass,
    pub reality_contact: RealityContact,
    pub support_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub code: &'static str,
    pub subjects: Vec<String>,
    pub basis: FindingBasis,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocker,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingBasis {
    ExactStructural,
    DeclaredModel,
    EvidencePolicy,
}

#[derive(Debug, Serialize)]
pub struct WorkGraphResult {
    pub schema_version: &'static str,
    pub algorithm: &'static str,
    pub model_sha256: String,
    pub acyclic: bool,
    pub ready_for_implementation: bool,
    pub external_apply_performed: bool,
    pub items: Vec<WorkItemOutput>,
    pub edges: Vec<WorkEdge>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkItemOutput {
    pub id: String,
    pub origin: WorkOrigin,
    pub kind: WorkKind,
    pub priority: u8,
    pub title: String,
    pub intent: String,
    pub changes: Vec<String>,
    pub satisfies: Vec<String>,
    pub evidence_gates: Vec<String>,
    pub resolves_findings: Vec<String>,
    pub cleanup_obligation: Option<String>,
    pub external_binding: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrigin {
    Declared,
    GeneratedObligation,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    Evidence,
    ModelRefinement,
    ContradictionResolution,
    Proof,
    Implementation,
    Migration,
    Cleanup,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkEdge {
    pub work: String,
    pub depends_on: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectionResult {
    pub algorithm: &'static str,
    pub model_sha256: String,
    pub projection_id: String,
    pub operation: &'static str,
    pub target_schema: String,
    pub target_input_json: String,
    pub target_input_sha256: String,
    pub manifest: ProjectionManifest,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ProjectionManifest {
    pub schema_version: &'static str,
    pub semantic_model_sha256: String,
    pub subject: String,
    pub scope: String,
    pub target_operation: &'static str,
    pub target_schema: String,
    pub target_input_sha256: String,
    pub lowering_version: String,
    pub claim_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub closure_ids: Vec<String>,
    pub assumptions: Vec<String>,
    pub mapped_entities: usize,
    pub omitted_entities: usize,
    pub mapped_relations: usize,
    pub omitted_relations: usize,
    pub source_claim_ceiling: SourceClaimCeiling,
}

struct Prepared<'a> {
    subjects: BTreeMap<&'a str, &'a Subject>,
    scopes: BTreeMap<&'a str, &'a Scope>,
    entities: BTreeMap<&'a str, &'a Entity>,
    evidence: BTreeMap<&'a str, &'a Evidence>,
    claims: BTreeMap<&'a str, &'a Claim>,
    projections: BTreeMap<&'a str, &'a KerfProjection>,
}

/// Parse JSON while rejecting duplicate object members at every depth.
///
/// # Errors
///
/// Returns [`ToolError`] for malformed JSON, duplicate members, or a value that
/// does not conform to the requested strict Serde type.
pub fn parse_no_duplicates<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ToolError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = NoDuplicateValue::deserialize(&mut deserializer)
        .map_err(|error| ToolError::invalid(format!("invalid JSON input: {error}")))?;
    deserializer
        .end()
        .map_err(|error| ToolError::invalid(format!("invalid JSON input: {error}")))?;
    serde_json::from_value(value.0)
        .map_err(|error| ToolError::invalid(format!("invalid semantic model: {error}")))
}

/// Validate and analyze one supplied semantic model.
///
/// # Errors
///
/// Returns [`ToolError`] when the schema, references, typed relations, hashes,
/// projection coverage, or embedded Kerf input are structurally invalid.
pub fn analyze(model: &SemanticModel) -> Result<AnalysisResult, ToolError> {
    let prepared = prepare(model)?;
    let model_sha256 = canonical_model_sha256(model)?;
    let mut findings = collect_findings(model, &prepared);
    sort_findings(&mut findings);
    let claim_groundings = claim_groundings(model, &prepared);
    let mut finding_counts = BTreeMap::new();
    finding_counts.insert(
        "blocker",
        findings
            .iter()
            .filter(|finding| finding.severity == Severity::Blocker)
            .count(),
    );
    finding_counts.insert(
        "error",
        findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count(),
    );
    finding_counts.insert(
        "warning",
        findings
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count(),
    );
    finding_counts.insert(
        "info",
        findings
            .iter()
            .filter(|finding| finding.severity == Severity::Info)
            .count(),
    );
    Ok(AnalysisResult {
        algorithm: "typed_semantic_model_analysis_v1",
        model_sha256,
        structurally_valid: true,
        subject_count: model.subjects.len(),
        entity_count: model.entities.len(),
        relation_count: model.relations.len(),
        binding_count: model.bindings.len(),
        claim_count: model.claims.len(),
        finding_counts,
        findings,
        claim_groundings,
        nonclaims: analysis_nonclaims(),
    })
}

/// Compile declared work and unresolved model obligations into a pure work graph.
///
/// # Errors
///
/// Returns [`ToolError`] when the semantic model is structurally invalid.
pub fn derive_work(model: &SemanticModel) -> Result<WorkGraphResult, ToolError> {
    let analysis = analyze(model)?;
    let mut items = Vec::new();
    let mut edges = BTreeSet::new();
    for declared in &model.work_items {
        let kind = if declared.cleanup_obligation.is_some() {
            WorkKind::Cleanup
        } else {
            WorkKind::Implementation
        };
        items.push(WorkItemOutput {
            id: declared.id.clone(),
            origin: WorkOrigin::Declared,
            kind,
            priority: 1,
            title: declared.intent.clone(),
            intent: declared.intent.clone(),
            changes: sorted_unique(declared.changes.clone()),
            satisfies: sorted_unique(declared.satisfies.clone()),
            evidence_gates: sorted_unique(declared.evidence_gates.clone()),
            resolves_findings: Vec::new(),
            cleanup_obligation: declared.cleanup_obligation.clone(),
            external_binding: declared.external_binding.clone(),
        });
        for dependency in &declared.depends_on {
            edges.insert(WorkEdge {
                work: declared.id.clone(),
                depends_on: dependency.clone(),
            });
        }
    }

    let mut generated_by_finding = BTreeMap::new();
    for finding in &analysis.findings {
        if finding.severity == Severity::Info {
            continue;
        }
        let id = obligation_id(finding);
        generated_by_finding.insert(finding.id.clone(), id.clone());
        items.push(WorkItemOutput {
            id,
            origin: WorkOrigin::GeneratedObligation,
            kind: finding_work_kind(finding),
            priority: finding_priority(finding),
            title: finding.next_action.clone(),
            intent: finding.message.clone(),
            changes: finding.subjects.clone(),
            satisfies: requirement_subjects(model, &finding.subjects),
            evidence_gates: Vec::new(),
            resolves_findings: vec![finding.id.clone()],
            cleanup_obligation: None,
            external_binding: None,
        });
    }

    add_requirement_dependencies(model, &analysis.findings, &generated_by_finding, &mut edges);
    add_declared_work_dependencies(model, &analysis.findings, &generated_by_finding, &mut edges);
    items.sort_by(|left, right| left.id.cmp(&right.id));
    let item_ids: BTreeSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
    edges.retain(|edge| {
        item_ids.contains(edge.work.as_str()) && item_ids.contains(edge.depends_on.as_str())
    });
    let edges: Vec<WorkEdge> = edges.into_iter().collect();
    let acyclic = work_graph_is_acyclic(&items, &edges);
    let ready_for_implementation = acyclic
        && analysis
            .findings
            .iter()
            .all(|finding| !matches!(finding.severity, Severity::Blocker | Severity::Error));
    Ok(WorkGraphResult {
        schema_version: WORK_GRAPH_SCHEMA,
        algorithm: "semantic_obligation_compilation_v1",
        model_sha256: analysis.model_sha256,
        acyclic,
        ready_for_implementation,
        external_apply_performed: false,
        items,
        edges,
        nonclaims: vec![
            "the work graph is a deterministic preview and does not mutate Beads or any repository",
            "work dependencies do not prove semantic claims or implementation correctness",
            "tracker status is not semantic evidence",
        ],
    })
}

/// Materialize one declared Kerf projection and its traceability manifest.
///
/// # Errors
///
/// Returns [`ToolError`] when the model or projection is invalid, the projection
/// ID is absent, or the embedded target is not a valid input for the named
/// existing Kerf operation.
pub fn project(model: &SemanticModel, projection_id: &str) -> Result<ProjectionResult, ToolError> {
    let prepared = prepare(model)?;
    let projection = prepared
        .projections
        .get(projection_id)
        .ok_or_else(|| ToolError::invalid(format!("unknown projection ID {projection_id:?}")))?;
    validate_target_input(projection)?;
    let target_input_json = serde_json::to_string(&projection.target_input)
        .map_err(|error| ToolError::internal(format!("could not encode target input: {error}")))?;
    let target_input_sha256 = sha256_hex(target_input_json.as_bytes());
    let target_schema = projection
        .target_input
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::invalid("projection target_input needs schema_version"))?
        .to_owned();
    let model_sha256 = canonical_model_sha256(model)?;
    let manifest = ProjectionManifest {
        schema_version: PROJECTION_MANIFEST_SCHEMA,
        semantic_model_sha256: model_sha256.clone(),
        subject: projection.subject.clone(),
        scope: projection.scope.clone(),
        target_operation: projection.operation.command(),
        target_schema: target_schema.clone(),
        target_input_sha256: target_input_sha256.clone(),
        lowering_version: projection.lowering_version.clone(),
        claim_ids: sorted_unique(projection.claim_ids.clone()),
        evidence_ids: sorted_unique(projection.evidence_ids.clone()),
        closure_ids: sorted_unique(projection.closure_ids.clone()),
        assumptions: sorted_unique(projection.assumptions.clone()),
        mapped_entities: projection.entity_map.len(),
        omitted_entities: projection.omitted_entities.len(),
        mapped_relations: projection.relation_map.len(),
        omitted_relations: projection.omitted_relations.len(),
        source_claim_ceiling: projection.source_claim_ceiling,
    };
    Ok(ProjectionResult {
        algorithm: "explicit_semantic_to_kerf_projection_v1",
        model_sha256,
        projection_id: projection.id.clone(),
        operation: projection.operation.command(),
        target_schema,
        target_input_json,
        target_input_sha256,
        manifest,
        nonclaims: vec![
            "the manifest proves deterministic traceability to the emitted finite input, not source-to-model correspondence",
            "a Kerf result can be exact for the emitted finite input while the implementation binding remains bounded, inferred, or unknown",
            "mapped and omitted coordinates are supplied declarations and are not inferred by this operation",
        ],
    })
}

#[allow(clippy::too_many_lines)]
fn prepare(model: &SemanticModel) -> Result<Prepared<'_>, ToolError> {
    if model.schema_version != SEMANTIC_MODEL_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {:?}",
            model.schema_version, SEMANTIC_MODEL_SCHEMA
        )));
    }
    require_nonempty("model_id", &model.model_id)?;
    let subjects = index_by_id("subject", &model.subjects, |item| &item.id)?;
    if subjects.is_empty() {
        return Err(ToolError::invalid("subjects must not be empty"));
    }
    for subject in &model.subjects {
        if let Some(parent) = &subject.parent {
            require_reference("subject.parent", parent, &subjects)?;
            if parent == &subject.id {
                return Err(ToolError::invalid(format!(
                    "subject {:?} cannot parent itself",
                    subject.id
                )));
            }
        }
    }
    require_acyclic(
        "subject.parent",
        &model
            .subjects
            .iter()
            .map(|item| {
                (
                    item.id.as_str(),
                    item.parent.iter().map(String::as_str).collect(),
                )
            })
            .collect(),
    )?;
    let scopes = index_by_id("scope", &model.scopes, |item| &item.id)?;
    for scope in &model.scopes {
        require_reference("scope.subject", &scope.subject, &subjects)?;
        require_unique_strings("scope.contexts", &scope.contexts, true)?;
        require_unique_strings("scope.workloads", &scope.workloads, true)?;
    }
    let entities = index_by_id("entity", &model.entities, |item| &item.id)?;
    let bindings = index_by_id("binding", &model.bindings, |item| &item.id)?;
    let evidence = index_by_id("evidence", &model.evidence, |item| &item.id)?;
    let closures = index_by_id("closure", &model.closures, |item| &item.id)?;
    let claims = index_by_id("claim", &model.claims, |item| &item.id)?;
    let work = index_by_id("work item", &model.work_items, |item| &item.id)?;
    let projections = index_by_id("projection", &model.projections, |item| &item.id)?;

    for binding in &model.bindings {
        require_reference("binding.subject", &binding.subject, &subjects)?;
        require_sha256("binding.artifact_sha256", &binding.artifact_sha256)?;
        require_nonempty("binding.locator", &binding.locator)?;
        require_unique_strings("binding.normalization", &binding.normalization, true)?;
        if let (Some(binding_revision), Some(subject_revision)) = (
            binding.revision.as_deref(),
            subjects[binding.subject.as_str()].revision.as_deref(),
        ) && binding_revision != subject_revision
        {
            return Err(ToolError::invalid(format!(
                "binding {:?} revision {:?} disagrees with subject {:?} revision {:?}",
                binding.id, binding_revision, binding.subject, subject_revision
            )));
        }
        let mut alphabet = BTreeSet::new();
        for coordinate in &binding.alphabet {
            require_nonempty("binding.alphabet.model_symbol", &coordinate.model_symbol)?;
            require_nonempty(
                "binding.alphabet.concrete_predicate",
                &coordinate.concrete_predicate,
            )?;
            if !alphabet.insert((coordinate.role, coordinate.model_symbol.as_str())) {
                return Err(ToolError::invalid(format!(
                    "binding {:?} repeats alphabet coordinate {:?}/{:?}",
                    binding.id, coordinate.role, coordinate.model_symbol
                )));
            }
        }
        if let Some(session) = &binding.session {
            require_nonempty("binding.session.open", &session.open)?;
            require_nonempty("binding.session.reset", &session.reset)?;
            require_nonempty("binding.session.close", &session.close)?;
        }
        let mut controls = BTreeSet::new();
        for control in &binding.negative_controls {
            require_nonempty("binding.negative_control.id", &control.id)?;
            require_nonempty(
                "binding.negative_control.intervention",
                &control.intervention,
            )?;
            require_nonempty(
                "binding.negative_control.expected_rejection",
                &control.expected_rejection,
            )?;
            if !controls.insert(control.id.as_str()) {
                return Err(ToolError::invalid(format!(
                    "binding {:?} repeats negative control {:?}",
                    binding.id, control.id
                )));
            }
        }
    }

    for item in &model.evidence {
        require_reference("evidence.subject", &item.subject, &subjects)?;
        require_reference("evidence.binding", &item.binding, &bindings)?;
        if bindings[item.binding.as_str()].subject != item.subject {
            return Err(ToolError::invalid(format!(
                "evidence {:?} subject {:?} disagrees with binding {:?} subject {:?}",
                item.id,
                item.subject,
                item.binding,
                bindings[item.binding.as_str()].subject
            )));
        }
        if item.kind == EvidenceKind::Source
            && bindings[item.binding.as_str()].kind == BindingKind::SourceArtifact
            && item.sha256 != bindings[item.binding.as_str()].artifact_sha256
        {
            return Err(ToolError::invalid(format!(
                "source evidence {:?} hash disagrees with source binding {:?}",
                item.id, item.binding
            )));
        }
        require_nonempty("evidence.source", &item.source)?;
        require_sha256("evidence.sha256", &item.sha256)?;
        require_references("evidence.derived_from", &item.derived_from, &evidence)?;
        require_unique_strings("evidence.coverage.contexts", &item.coverage.contexts, true)?;
        require_unique_strings(
            "evidence.coverage.workloads",
            &item.coverage.workloads,
            true,
        )?;
        require_unique_strings(
            "evidence.coverage.limitations",
            &item.coverage.limitations,
            false,
        )?;
    }
    require_acyclic(
        "evidence.derived_from",
        &model
            .evidence
            .iter()
            .map(|item| {
                (
                    item.id.as_str(),
                    item.derived_from.iter().map(String::as_str).collect(),
                )
            })
            .collect(),
    )?;

    for entity in &model.entities {
        require_reference("entity.subject", &entity.subject, &subjects)?;
        require_nonempty("entity.label", &entity.label)?;
        validate_entity(entity, &entities, &evidence, &claims)?;
    }

    let mut relation_ids = BTreeSet::new();
    for relation in &model.relations {
        require_nonempty("relation.id", &relation.id)?;
        if !relation_ids.insert(relation.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate relation ID {:?}",
                relation.id
            )));
        }
        require_unique_strings("relation.tails", &relation.tails, false)?;
        require_unique_strings("relation.heads", &relation.heads, false)?;
        if relation.tails.is_empty() || relation.heads.is_empty() {
            return Err(ToolError::invalid(format!(
                "relation {:?} needs non-empty tails and heads",
                relation.id
            )));
        }
        require_references("relation.tails", &relation.tails, &entities)?;
        require_references("relation.heads", &relation.heads, &entities)?;
        require_references("relation.evidence", &relation.evidence, &evidence)?;
        validate_relation_types(relation, &entities)?;
        validate_relation_subjects(relation, &entities)?;
    }

    for closure in &model.closures {
        require_reference("closure.subject", &closure.subject, &subjects)?;
        validate_scope_binding(&closure.subject, &closure.scope, &scopes)?;
        require_nonempty("closure.domain", &closure.domain)?;
        require_references("closure.evidence", &closure.evidence, &evidence)?;
        for evidence_id in &closure.evidence {
            if evidence[evidence_id.as_str()].subject != closure.subject {
                return Err(ToolError::invalid(format!(
                    "closure {:?} evidence {:?} belongs to subject {:?}, not {:?}",
                    closure.id,
                    evidence_id,
                    evidence[evidence_id.as_str()].subject,
                    closure.subject
                )));
            }
        }
    }

    for claim in &model.claims {
        require_reference("claim.subject", &claim.subject, &subjects)?;
        validate_scope_binding(&claim.subject, &claim.scope, &scopes)?;
        require_nonempty("claim.statement", &claim.statement)?;
        require_references("claim.subjects", &claim.subjects, &entities)?;
        require_references("claim.support", &claim.support, &evidence)?;
        require_references("claim.refutation", &claim.refutation, &evidence)?;
        require_references("claim.assumptions", &claim.assumptions, &claims)?;
        require_references("claim.conflicts_with", &claim.conflicts_with, &claims)?;
        let involved_subjects: BTreeSet<&str> = std::iter::once(claim.subject.as_str())
            .chain(
                claim
                    .subjects
                    .iter()
                    .map(|id| entities[id.as_str()].subject.as_str()),
            )
            .collect();
        for evidence_id in claim.support.iter().chain(&claim.refutation) {
            let evidence_subject = evidence[evidence_id.as_str()].subject.as_str();
            if !involved_subjects.contains(evidence_subject) {
                return Err(ToolError::invalid(format!(
                    "claim {:?} uses evidence {:?} from unrelated subject {:?}",
                    claim.id, evidence_id, evidence_subject
                )));
            }
        }
        if claim.conflicts_with.iter().any(|id| id == &claim.id) {
            return Err(ToolError::invalid(format!(
                "claim {:?} cannot conflict with itself",
                claim.id
            )));
        }
        if let Some(checker) = &claim.checker {
            require_nonempty("checker.tool", &checker.tool)?;
            require_nonempty("checker.operation", &checker.operation)?;
            require_sha256("checker.input_sha256", &checker.input_sha256)?;
            require_sha256("checker.result_sha256", &checker.result_sha256)?;
        }
    }
    require_acyclic(
        "claim.assumptions",
        &model
            .claims
            .iter()
            .map(|item| {
                (
                    item.id.as_str(),
                    item.assumptions.iter().map(String::as_str).collect(),
                )
            })
            .collect(),
    )?;

    for item in &model.work_items {
        if item.id.starts_with("obligation::") {
            return Err(ToolError::invalid(format!(
                "declared work item {:?} uses reserved prefix \"obligation::\"",
                item.id
            )));
        }
        require_nonempty("work.intent", &item.intent)?;
        require_references("work.changes", &item.changes, &entities)?;
        require_entity_kind_references(
            "work.satisfies",
            &item.satisfies,
            &entities,
            is_requirement,
            "requirement",
        )?;
        require_references("work.depends_on", &item.depends_on, &work)?;
        require_references("work.evidence_gates", &item.evidence_gates, &claims)?;
        if item.depends_on.iter().any(|id| id == &item.id) {
            return Err(ToolError::invalid(format!(
                "work item {:?} cannot depend on itself",
                item.id
            )));
        }
    }

    for projection in &model.projections {
        validate_projection(
            projection,
            &subjects,
            &scopes,
            &entities,
            &model.relations,
            &evidence,
            &closures,
            &claims,
        )?;
    }

    Ok(Prepared {
        subjects,
        scopes,
        entities,
        evidence,
        claims,
        projections,
    })
}

#[allow(clippy::too_many_lines)]
fn validate_entity(
    entity: &Entity,
    entities: &BTreeMap<&str, &Entity>,
    evidence: &BTreeMap<&str, &Evidence>,
    claims: &BTreeMap<&str, &Claim>,
) -> Result<(), ToolError> {
    match &entity.kind {
        EntityKind::Requirement {
            acceptance_claims, ..
        } => {
            require_references("requirement.acceptance_claims", acceptance_claims, claims)?;
            for claim_id in acceptance_claims {
                if claims[claim_id.as_str()].subject != entity.subject {
                    return Err(ToolError::invalid(format!(
                        "requirement {:?} acceptance claim {:?} belongs to subject {:?}, not {:?}",
                        entity.id,
                        claim_id,
                        claims[claim_id.as_str()].subject,
                        entity.subject
                    )));
                }
            }
            Ok(())
        }
        EntityKind::Carrier {
            fact, authority, ..
        } => {
            require_nonempty("carrier.fact", fact)?;
            if let Some(authority) = authority {
                require_entity_kind_reference(
                    "carrier.authority",
                    authority,
                    entities,
                    is_authority,
                    "authority",
                )?;
                require_same_subject("carrier.authority", entity, authority, entities)?;
            }
            Ok(())
        }
        EntityKind::Authority {
            writers,
            consistency_contract,
            ..
        } => {
            require_unique_strings("authority.writers", writers, false)?;
            require_references("authority.writers", writers, entities)?;
            for writer in writers {
                require_same_subject("authority.writers", entity, writer, entities)?;
            }
            if let Some(contract) = consistency_contract {
                require_entity_kind_reference(
                    "authority.consistency_contract",
                    contract,
                    entities,
                    is_contract,
                    "contract",
                )?;
                require_same_subject("authority.consistency_contract", entity, contract, entities)?;
            }
            Ok(())
        }
        EntityKind::DerivedView {
            source,
            invalidated_by,
            ..
        } => {
            require_entity_kind_reference(
                "derived_view.source",
                source,
                entities,
                is_state,
                "carrier or derived_view",
            )?;
            require_same_subject("derived_view.source", entity, source, entities)?;
            require_entity_kind_references(
                "derived_view.invalidated_by",
                invalidated_by,
                entities,
                is_transition_like,
                "action or transition",
            )?;
            for transition in invalidated_by {
                require_same_subject("derived_view.invalidated_by", entity, transition, entities)?;
            }
            Ok(())
        }
        EntityKind::Observation { oracle_evidence } => {
            require_references("observation.oracle_evidence", oracle_evidence, evidence)?;
            for evidence_id in oracle_evidence {
                if evidence[evidence_id.as_str()].subject != entity.subject {
                    return Err(ToolError::invalid(format!(
                        "observation {:?} evidence {:?} belongs to subject {:?}, not {:?}",
                        entity.id,
                        evidence_id,
                        evidence[evidence_id.as_str()].subject,
                        entity.subject
                    )));
                }
            }
            Ok(())
        }
        EntityKind::Effect {
            intent_carrier,
            receipt_observation,
            reconciliation_transition,
        } => {
            if let Some(id) = intent_carrier {
                require_entity_kind_reference(
                    "effect.intent_carrier",
                    id,
                    entities,
                    is_carrier,
                    "carrier",
                )?;
                require_same_subject("effect.intent_carrier", entity, id, entities)?;
            }
            if let Some(id) = receipt_observation {
                require_entity_kind_reference(
                    "effect.receipt_observation",
                    id,
                    entities,
                    is_observation,
                    "observation",
                )?;
                require_same_subject("effect.receipt_observation", entity, id, entities)?;
            }
            if let Some(id) = reconciliation_transition {
                require_entity_kind_reference(
                    "effect.reconciliation_transition",
                    id,
                    entities,
                    is_transition_like,
                    "action or transition",
                )?;
                require_same_subject("effect.reconciliation_transition", entity, id, entities)?;
            }
            Ok(())
        }
        EntityKind::Port { semantic_type, .. } => {
            require_nonempty("port.semantic_type", semantic_type)
        }
        EntityKind::Actor { actor_kind } => require_nonempty("actor.actor_kind", actor_kind),
        EntityKind::Implementation { locator } => {
            require_nonempty("implementation.locator", locator)
        }
        EntityKind::Resource { resource_kind } => {
            require_nonempty("resource.resource_kind", resource_kind)
        }
        EntityKind::Action
        | EntityKind::Transition
        | EntityKind::Contract { .. }
        | EntityKind::Invariant => Ok(()),
    }
}

fn validate_relation_types(
    relation: &Relation,
    entities: &BTreeMap<&str, &Entity>,
) -> Result<(), ToolError> {
    let check = |ids: &[String], predicate: fn(&Entity) -> bool, expected: &str| {
        for id in ids {
            let entity = entities[id.as_str()];
            if !predicate(entity) {
                return Err(ToolError::invalid(format!(
                    "relation {:?} expects {expected} at {:?}",
                    relation.id, id
                )));
            }
        }
        Ok(())
    };
    match relation.kind {
        RelationKind::RequiresObservation => {
            check(&relation.tails, is_requirement, "requirement tails")?;
            check(&relation.heads, is_observation, "observation heads")
        }
        RelationKind::RealizedBy => {
            check(&relation.tails, is_requirement, "requirement tails")?;
            check(
                &relation.heads,
                is_semantic_realization,
                "semantic realization heads",
            )
        }
        RelationKind::Reads | RelationKind::Writes => {
            check(
                &relation.tails,
                is_behavior_actor,
                "actor/action/transition/implementation tails",
            )?;
            check(&relation.heads, is_state, "carrier or derived_view heads")
        }
        RelationKind::Emits => {
            check(
                &relation.tails,
                is_transition_like,
                "action or transition tails",
            )?;
            check(
                &relation.heads,
                is_effect_or_observation,
                "effect or observation heads",
            )
        }
        RelationKind::HappensBefore => {
            check(
                &relation.tails,
                is_transition_like,
                "action or transition tails",
            )?;
            check(
                &relation.heads,
                is_transition_like,
                "action or transition heads",
            )
        }
        RelationKind::DerivesFrom => {
            check(&relation.tails, is_derived_view, "derived_view tails")?;
            check(&relation.heads, is_state, "carrier or derived_view heads")
        }
        RelationKind::Invalidates => {
            check(
                &relation.tails,
                is_transition_like,
                "action or transition tails",
            )?;
            check(&relation.heads, is_derived_view, "derived_view heads")
        }
        RelationKind::Implements => {
            check(&relation.tails, is_implementation, "implementation tails")?;
            check(
                &relation.heads,
                is_semantic_realization,
                "semantic realization heads",
            )
        }
        RelationKind::Calls | RelationKind::Imports | RelationKind::BuildInvalidates => {
            check(&relation.tails, is_implementation, "implementation tails")?;
            check(&relation.heads, is_implementation, "implementation heads")
        }
        RelationKind::CorrespondsTo
        | RelationKind::Decomposes
        | RelationKind::Refines
        | RelationKind::FlowsOver
        | RelationKind::ContendsOn => Ok(()),
    }
}

fn validate_relation_subjects(
    relation: &Relation,
    entities: &BTreeMap<&str, &Entity>,
) -> Result<(), ToolError> {
    if matches!(
        relation.kind,
        RelationKind::Implements | RelationKind::CorrespondsTo | RelationKind::Refines
    ) {
        return Ok(());
    }
    let subjects: BTreeSet<&str> = relation
        .tails
        .iter()
        .chain(&relation.heads)
        .map(|id| entities[id.as_str()].subject.as_str())
        .collect();
    if subjects.len() != 1 {
        return Err(ToolError::invalid(format!(
            "relation {:?} of kind {:?} crosses subjects {subjects:?}; use implements, corresponds_to, or refines for cross-subject semantics",
            relation.id, relation.kind
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn validate_projection(
    projection: &KerfProjection,
    subjects: &BTreeMap<&str, &Subject>,
    scopes: &BTreeMap<&str, &Scope>,
    entities: &BTreeMap<&str, &Entity>,
    relations: &[Relation],
    evidence: &BTreeMap<&str, &Evidence>,
    closures: &BTreeMap<&str, &ClosureClaim>,
    claims: &BTreeMap<&str, &Claim>,
) -> Result<(), ToolError> {
    require_reference("projection.subject", &projection.subject, subjects)?;
    validate_scope_binding(&projection.subject, &projection.scope, scopes)?;
    require_nonempty("projection.lowering_version", &projection.lowering_version)?;
    require_unique_strings(
        "projection.scope_entities",
        &projection.scope_entities,
        false,
    )?;
    if projection.scope_entities.is_empty() {
        return Err(ToolError::invalid(format!(
            "projection {:?} scope_entities must not be empty",
            projection.id
        )));
    }
    require_references(
        "projection.scope_entities",
        &projection.scope_entities,
        entities,
    )?;
    for id in &projection.scope_entities {
        if entities[id.as_str()].subject != projection.subject {
            return Err(ToolError::invalid(format!(
                "projection {:?} entity {:?} belongs to subject {:?}, not {:?}",
                projection.id,
                id,
                entities[id.as_str()].subject,
                projection.subject
            )));
        }
    }
    let mapped_entities = validate_maps("projection.entity_map", &projection.entity_map, entities)?;
    let omitted_entities = validate_omissions(
        "projection.omitted_entities",
        &projection.omitted_entities,
        entities,
    )?;
    require_exact_partition(
        &format!("projection {:?} entity coverage", projection.id),
        &projection.scope_entities,
        &mapped_entities,
        &omitted_entities,
    )?;
    validate_target_pointers(
        &projection.id,
        "entity_map",
        &projection.entity_map,
        &projection.target_input,
    )?;

    let relation_index: BTreeMap<&str, &Relation> = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect();
    let mapped_relations = validate_maps(
        "projection.relation_map",
        &projection.relation_map,
        &relation_index,
    )?;
    let omitted_relations = validate_omissions(
        "projection.omitted_relations",
        &projection.omitted_relations,
        &relation_index,
    )?;
    let scope_set: BTreeSet<&str> = projection
        .scope_entities
        .iter()
        .map(String::as_str)
        .collect();
    let incident: Vec<String> = relations
        .iter()
        .filter(|relation| {
            relation
                .tails
                .iter()
                .chain(&relation.heads)
                .any(|id| scope_set.contains(id.as_str()))
        })
        .map(|relation| relation.id.clone())
        .collect();
    require_exact_partition(
        &format!("projection {:?} incident relation coverage", projection.id),
        &incident,
        &mapped_relations,
        &omitted_relations,
    )?;
    validate_target_pointers(
        &projection.id,
        "relation_map",
        &projection.relation_map,
        &projection.target_input,
    )?;
    require_references("projection.claim_ids", &projection.claim_ids, claims)?;
    require_references(
        "projection.evidence_ids",
        &projection.evidence_ids,
        evidence,
    )?;
    for evidence_id in &projection.evidence_ids {
        if evidence[evidence_id.as_str()].subject != projection.subject {
            return Err(ToolError::invalid(format!(
                "projection {:?} evidence {:?} belongs to subject {:?}, not {:?}",
                projection.id,
                evidence_id,
                evidence[evidence_id.as_str()].subject,
                projection.subject
            )));
        }
    }
    require_references("projection.closure_ids", &projection.closure_ids, closures)?;
    for closure_id in &projection.closure_ids {
        let closure = closures[closure_id.as_str()];
        if closure.subject != projection.subject || closure.scope != projection.scope {
            return Err(ToolError::invalid(format!(
                "projection {:?} closure {:?} belongs to subject/scope {:?}/{:?}, not {:?}/{:?}",
                projection.id,
                closure_id,
                closure.subject,
                closure.scope,
                projection.subject,
                projection.scope
            )));
        }
    }
    for claim_id in &projection.claim_ids {
        let claim = claims[claim_id.as_str()];
        if claim.subject != projection.subject || claim.scope != projection.scope {
            return Err(ToolError::invalid(format!(
                "projection {:?} claim {:?} belongs to subject/scope {:?}/{:?}, not {:?}/{:?}",
                projection.id,
                claim_id,
                claim.subject,
                claim.scope,
                projection.subject,
                projection.scope
            )));
        }
    }
    require_references("projection.assumptions", &projection.assumptions, claims)?;
    validate_target_input(projection)
}

fn validate_target_pointers(
    projection_id: &str,
    field: &str,
    mappings: &[ProjectionMap],
    target: &Value,
) -> Result<(), ToolError> {
    for mapping in mappings {
        if !mapping.target_pointer.starts_with('/')
            || target.pointer(&mapping.target_pointer).is_none()
        {
            return Err(ToolError::invalid(format!(
                "projection {projection_id:?} {field} pointer {:?} does not resolve in target_input",
                mapping.target_pointer
            )));
        }
    }
    Ok(())
}

fn validate_target_input(projection: &KerfProjection) -> Result<(), ToolError> {
    let value = projection.target_input.clone();
    let result = match projection.operation {
        KerfOperation::ValidateLts | KerfOperation::Quotient => {
            serde_json::from_value::<LtsInput>(value).map(|_| ())
        }
        KerfOperation::Distinguish => serde_json::from_value::<CompareInput>(value).map(|_| ()),
        KerfOperation::Factor => serde_json::from_value::<FactorInput>(value).map(|_| ()),
        KerfOperation::SearchFactors => {
            serde_json::from_value::<FactorSearchInput>(value).map(|_| ())
        }
        KerfOperation::Recompose => serde_json::from_value::<RecomposeInput>(value).map(|_| ()),
        KerfOperation::CheckMorphism => serde_json::from_value::<MorphismInput>(value).map(|_| ()),
        KerfOperation::VerifyCertificate => {
            serde_json::from_value::<CertificateInput>(value).map(|_| ())
        }
        KerfOperation::ValidateCandidates | KerfOperation::Pareto => {
            serde_json::from_value::<CandidateInput>(value).map(|_| ())
        }
        KerfOperation::ValidateMigration => {
            serde_json::from_value::<MigrationGraphInput>(value).map(|_| ())
        }
        KerfOperation::ScoreRoutes => serde_json::from_value::<TransportInput>(value).map(|_| ()),
    };
    result.map_err(|error| {
        ToolError::invalid(format!(
            "projection {:?} is not a valid {} input: {error}",
            projection.id,
            projection.operation.command()
        ))
    })
}

#[allow(clippy::too_many_lines)]
fn collect_findings(model: &SemanticModel, prepared: &Prepared<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for subject in &model.subjects {
        if subject.kind == SubjectKind::Implementation
            && subject.revision.as_deref().is_none_or(str::is_empty)
        {
            push_finding(
                &mut findings,
                Severity::Error,
                "implementation_revision_missing",
                vec![subject.id.clone()],
                FindingBasis::EvidencePolicy,
                "an implementation subject is not pinned to a revision",
                "Pin the implementation subject to a repository or build revision.",
            );
        }
    }

    for binding in &model.bindings {
        if matches!(
            binding.kind,
            BindingKind::ExecutableProtocol | BindingKind::RuntimeProtocol
        ) {
            if binding.alphabet.is_empty() {
                push_finding(
                    &mut findings,
                    Severity::Error,
                    "runtime_binding_alphabet_missing",
                    vec![binding.id.clone()],
                    FindingBasis::EvidencePolicy,
                    "runtime binding has no explicit action, observation, or emission alphabet",
                    "Map each modeled symbol to a concrete invocation or observation predicate.",
                );
            }
            if binding.session.is_none() {
                push_finding(
                    &mut findings,
                    Severity::Error,
                    "runtime_binding_session_missing",
                    vec![binding.id.clone()],
                    FindingBasis::EvidencePolicy,
                    "runtime binding has no open/reset/close protocol",
                    "Declare session and reset behavior so multi-step traces have a reproducible initial state.",
                );
            }
            if binding.negative_controls.is_empty() {
                push_finding(
                    &mut findings,
                    Severity::Error,
                    "runtime_binding_negative_control_missing",
                    vec![binding.id.clone()],
                    FindingBasis::EvidencePolicy,
                    "runtime binding has no nearby negative control",
                    "Add a single-variable perturbation expected to be rejected under the same binding.",
                );
            }
        }
    }

    for evidence in &model.evidence {
        if matches!(
            evidence.kind,
            EvidenceKind::Source
                | EvidenceKind::Runtime
                | EvidenceKind::History
                | EvidenceKind::Build
        ) && evidence.revision.as_deref().is_none_or(str::is_empty)
        {
            push_finding(
                &mut findings,
                Severity::Warning,
                "revision_bound_evidence_missing_revision",
                vec![evidence.id.clone()],
                FindingBasis::EvidencePolicy,
                "source, runtime, history, or build evidence is not revision-bound",
                "Bind the evidence to the exact subject revision or mark the dependent claim stale.",
            );
        }
    }

    for entity in &model.entities {
        match &entity.kind {
            EntityKind::Requirement {
                modality,
                acceptance_claims,
            } => analyze_requirement(model, entity, *modality, acceptance_claims, &mut findings),
            EntityKind::Carrier {
                fact, authority, ..
            } => {
                if authority.is_none() {
                    push_finding(
                        &mut findings,
                        Severity::Error,
                        "carrier_authority_missing",
                        vec![entity.id.clone()],
                        FindingBasis::DeclaredModel,
                        &format!("carrier for fact {fact:?} has no authority protocol"),
                        "Declare the authority protocol and its allowed writers.",
                    );
                }
            }
            EntityKind::Authority {
                protocol,
                writers,
                consistency_contract,
            } => analyze_authority(
                entity,
                *protocol,
                writers,
                consistency_contract.as_deref(),
                &mut findings,
            ),
            EntityKind::DerivedView {
                validity,
                invalidated_by,
                ..
            } => {
                if validity.as_deref().is_none_or(str::is_empty) {
                    push_finding(
                        &mut findings,
                        Severity::Warning,
                        "derived_view_validity_missing",
                        vec![entity.id.clone()],
                        FindingBasis::DeclaredModel,
                        "derived view has no declared validity rule",
                        "Declare when the view is valid and which source revision or epoch it reflects.",
                    );
                }
                if invalidated_by.is_empty() {
                    push_finding(
                        &mut findings,
                        Severity::Warning,
                        "derived_view_invalidation_missing",
                        vec![entity.id.clone()],
                        FindingBasis::DeclaredModel,
                        "derived view has no invalidation or refresh transition",
                        "Declare an invalidation, refresh, or consistency transition.",
                    );
                }
            }
            EntityKind::Effect {
                intent_carrier,
                receipt_observation,
                reconciliation_transition,
            } => {
                let missing = [
                    (intent_carrier.is_none(), "intent"),
                    (receipt_observation.is_none(), "receipt"),
                    (reconciliation_transition.is_none(), "reconciliation"),
                ]
                .into_iter()
                .filter_map(|(is_missing, name)| is_missing.then_some(name))
                .collect::<Vec<_>>();
                if !missing.is_empty() {
                    push_finding(
                        &mut findings,
                        Severity::Error,
                        "effect_recovery_protocol_incomplete",
                        vec![entity.id.clone()],
                        FindingBasis::DeclaredModel,
                        &format!("effect is missing {}", missing.join(", ")),
                        "Model durable intent, an observable receipt, and a reconciliation transition.",
                    );
                }
            }
            _ => {}
        }
    }

    analyze_fact_authorities(model, &mut findings);
    analyze_unauthorized_writers(model, prepared, &mut findings);
    analyze_implementation_bindings(model, prepared, &mut findings);
    analyze_closures(model, &mut findings);
    analyze_claims(model, prepared, &mut findings);
    analyze_projections(model, prepared, &mut findings);
    analyze_work_cycles(model, &mut findings);
    findings
}

fn claim_groundings(model: &SemanticModel, prepared: &Prepared<'_>) -> Vec<ClaimGrounding> {
    let mut result = Vec::new();
    for claim in &model.claims {
        let reality_contact = claim
            .support
            .iter()
            .map(|id| evidence_reality_contact(prepared.evidence[id.as_str()]))
            .max()
            .unwrap_or(RealityContact::None);
        result.push(ClaimGrounding {
            claim_id: claim.id.clone(),
            formal_class: claim.epistemic_class,
            reality_contact,
            support_evidence: sorted_unique(claim.support.clone()),
        });
    }
    result.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
    result
}

fn evidence_reality_contact(evidence: &Evidence) -> RealityContact {
    match evidence.acquisition {
        Acquisition::Intervened => RealityContact::Intervened,
        Acquisition::Observed => RealityContact::Observed,
        Acquisition::Declared => RealityContact::DeclaredArtifact,
        Acquisition::Inferred | Acquisition::Derived => RealityContact::None,
    }
}

fn analyze_implementation_bindings(
    model: &SemanticModel,
    prepared: &Prepared<'_>,
    findings: &mut Vec<Finding>,
) {
    let has_implementation_subject = model
        .subjects
        .iter()
        .any(|subject| subject.kind == SubjectKind::Implementation);
    if !has_implementation_subject {
        return;
    }
    let implemented: BTreeSet<&str> = model
        .relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::Implements)
        .flat_map(|relation| relation.heads.iter().map(String::as_str))
        .collect();
    for requirement in model
        .entities
        .iter()
        .filter(|entity| is_requirement(entity))
    {
        let realizations: BTreeSet<&str> = model
            .relations
            .iter()
            .filter(|relation| {
                relation.kind == RelationKind::RealizedBy
                    && relation.tails.iter().any(|id| id == &requirement.id)
            })
            .flat_map(|relation| relation.heads.iter().map(String::as_str))
            .collect();
        let missing: Vec<String> = realizations
            .difference(&implemented)
            .copied()
            .map(str::to_owned)
            .collect();
        if !missing.is_empty() {
            let mut subjects = vec![requirement.id.clone()];
            subjects.extend(missing);
            let severity = match &requirement.kind {
                EntityKind::Requirement {
                    modality: RequirementModality::Must,
                    ..
                } => Severity::Error,
                _ => Severity::Warning,
            };
            push_finding(
                findings,
                severity,
                "requirement_implementation_binding_missing",
                subjects,
                FindingBasis::DeclaredModel,
                "one or more semantic realizations of a requirement have no implementation binding",
                "Bind implementation entities to every required carrier, transition, effect, port, contract, or invariant.",
            );
        }
    }
    for entity in model
        .entities
        .iter()
        .filter(|entity| is_implementation(entity))
    {
        let bound = model.relations.iter().any(|relation| {
            relation.kind == RelationKind::Implements
                && relation.tails.iter().any(|id| id == &entity.id)
        });
        if !bound && prepared.subjects[entity.subject.as_str()].kind == SubjectKind::Implementation
        {
            push_finding(
                findings,
                Severity::Warning,
                "implementation_semantic_binding_missing",
                vec![entity.id.clone()],
                FindingBasis::DeclaredModel,
                "implementation entity is not bound to any semantic realization",
                "Bind it to a required semantic entity or classify it as removable/derived work.",
            );
        }
    }
}

fn analyze_requirement(
    model: &SemanticModel,
    requirement: &Entity,
    modality: RequirementModality,
    acceptance_claims: &[String],
    findings: &mut Vec<Finding>,
) {
    let has_observation = model.relations.iter().any(|relation| {
        relation.kind == RelationKind::RequiresObservation
            && relation.tails.iter().any(|id| id == &requirement.id)
    });
    let has_realization = model.relations.iter().any(|relation| {
        relation.kind == RelationKind::RealizedBy
            && relation.tails.iter().any(|id| id == &requirement.id)
    });
    let has_work = model
        .work_items
        .iter()
        .any(|item| item.satisfies.iter().any(|id| id == &requirement.id));
    let severity = if modality == RequirementModality::Must {
        Severity::Error
    } else {
        Severity::Warning
    };
    if !has_observation {
        push_finding(
            findings,
            severity,
            "requirement_observation_missing",
            vec![requirement.id.clone()],
            FindingBasis::DeclaredModel,
            "requirement has no protected observation",
            "Define what an exterior observer can use to accept or refute this requirement.",
        );
    }
    if !has_realization {
        push_finding(
            findings,
            severity,
            "requirement_realization_missing",
            vec![requirement.id.clone()],
            FindingBasis::DeclaredModel,
            "requirement reaches no carrier, transition, effect, port, contract, or invariant",
            "Connect the requirement to the semantic entities that realize or constrain it.",
        );
    }
    if acceptance_claims.is_empty() {
        push_finding(
            findings,
            severity,
            "requirement_acceptance_missing",
            vec![requirement.id.clone()],
            FindingBasis::DeclaredModel,
            "requirement has no acceptance claim",
            "Add a scoped claim and evidence policy that can accept or refute the requirement.",
        );
    }
    if !has_work {
        push_finding(
            findings,
            severity,
            "requirement_work_missing",
            vec![requirement.id.clone()],
            FindingBasis::DeclaredModel,
            "requirement has no implementation or evidence work item",
            "Create a work item that satisfies the requirement and names its evidence gates.",
        );
    }
}

fn analyze_authority(
    entity: &Entity,
    protocol: AuthorityProtocol,
    writers: &[String],
    consistency_contract: Option<&str>,
    findings: &mut Vec<Finding>,
) {
    let valid_writer_count = match protocol {
        AuthorityProtocol::SingleWriter => writers.len() == 1,
        AuthorityProtocol::ReplicatedConsensus | AuthorityProtocol::TransactionalMultiWriter => {
            writers.len() >= 2
        }
        AuthorityProtocol::EventSourced => !writers.is_empty(),
    };
    if !valid_writer_count {
        push_finding(
            findings,
            Severity::Blocker,
            "authority_writer_cardinality_invalid",
            vec![entity.id.clone()],
            FindingBasis::ExactStructural,
            &format!(
                "authority protocol {protocol:?} has {} writer(s)",
                writers.len()
            ),
            "Make writer cardinality match the declared authority protocol.",
        );
    }
    if protocol != AuthorityProtocol::SingleWriter && consistency_contract.is_none() {
        push_finding(
            findings,
            Severity::Error,
            "multi_writer_consistency_contract_missing",
            vec![entity.id.clone()],
            FindingBasis::DeclaredModel,
            "multi-writer authority has no consistency contract",
            "Name the consensus, transaction, event-order, or reconciliation contract.",
        );
    }
}

fn analyze_fact_authorities(model: &SemanticModel, findings: &mut Vec<Finding>) {
    let mut facts: BTreeMap<(&str, &str), BTreeSet<&str>> = BTreeMap::new();
    for entity in &model.entities {
        if let EntityKind::Carrier {
            fact,
            authority: Some(authority),
            ..
        } = &entity.kind
        {
            facts
                .entry((entity.subject.as_str(), fact.as_str()))
                .or_default()
                .insert(authority);
        }
    }
    for ((subject, fact), authorities) in facts {
        if authorities.len() > 1 {
            let mut subjects = vec![subject.to_owned(), fact.to_owned()];
            subjects.extend(authorities.into_iter().map(str::to_owned));
            push_finding(
                findings,
                Severity::Blocker,
                "fact_authority_conflict",
                subjects,
                FindingBasis::ExactStructural,
                "one semantic fact resolves to multiple authority protocols in one subject",
                "Reconcile the carriers under one explicit authority protocol or split the semantic fact.",
            );
        }
    }
}

fn analyze_unauthorized_writers(
    model: &SemanticModel,
    prepared: &Prepared<'_>,
    findings: &mut Vec<Finding>,
) {
    for relation in &model.relations {
        if relation.kind != RelationKind::Writes {
            continue;
        }
        for carrier_id in &relation.heads {
            let entity = prepared.entities[carrier_id.as_str()];
            let EntityKind::Carrier {
                authority: Some(authority_id),
                ..
            } = &entity.kind
            else {
                continue;
            };
            let authority = prepared.entities[authority_id.as_str()];
            let EntityKind::Authority { writers, .. } = &authority.kind else {
                continue;
            };
            for writer in &relation.tails {
                if !writers.contains(writer) {
                    push_finding(
                        findings,
                        Severity::Blocker,
                        "unauthorized_writer",
                        vec![writer.clone(), carrier_id.clone(), authority_id.clone()],
                        FindingBasis::ExactStructural,
                        "a declared write is not admitted by the carrier authority protocol",
                        "Add the writer through the intended protocol or remove/refute the write relation.",
                    );
                }
            }
        }
    }
}

fn analyze_closures(model: &SemanticModel, findings: &mut Vec<Finding>) {
    for closure in &model.closures {
        if closure.kind != ClosureKind::Unknown && closure.evidence.is_empty() {
            push_finding(
                findings,
                if matches!(
                    closure.kind,
                    ClosureKind::ExhaustiveFinite
                        | ClosureKind::SoundOverapproximation
                        | ClosureKind::CompleteForDeclaredCorpus
                ) {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                "closure_evidence_missing",
                vec![closure.id.clone()],
                FindingBasis::EvidencePolicy,
                "a positive closure claim has no evidence",
                "Attach the exhaustive enumeration, sound-analysis receipt, or corpus manifest.",
            );
        }
    }
}

#[allow(clippy::too_many_lines)]
fn analyze_claims(model: &SemanticModel, prepared: &Prepared<'_>, findings: &mut Vec<Finding>) {
    let mut conflict_pairs = BTreeSet::new();
    for claim in &model.claims {
        if claim.kind == ClaimKind::SourceCorrespondence
            && claim.verdict == ClaimVerdict::Supported
            && matches!(
                claim.epistemic_class,
                EpistemicClass::TheoremInstance | EpistemicClass::ExactCheckedModel
            )
        {
            push_finding(
                findings,
                Severity::Blocker,
                "source_claim_laundering",
                vec![claim.id.clone()],
                FindingBasis::EvidencePolicy,
                "an exact finite-target result is presented as exact source correspondence",
                "Limit exactness to the finite target and keep source correspondence at evidence_inference or a declared bounded check.",
            );
        }
        if claim.kind == ClaimKind::SourceCorrespondence && claim.verdict == ClaimVerdict::Supported
        {
            let involved_subjects: BTreeSet<&str> = std::iter::once(claim.subject.as_str())
                .chain(
                    claim
                        .subjects
                        .iter()
                        .map(|id| prepared.entities[id.as_str()].subject.as_str()),
                )
                .collect();
            let evidence_subjects: BTreeSet<&str> = claim
                .support
                .iter()
                .map(|id| prepared.evidence[id.as_str()].subject.as_str())
                .collect();
            let missing: Vec<_> = involved_subjects
                .difference(&evidence_subjects)
                .copied()
                .collect();
            if !missing.is_empty() {
                push_finding(
                    findings,
                    Severity::Blocker,
                    "source_correspondence_subject_evidence_missing",
                    std::iter::once(claim.id.clone())
                        .chain(missing.iter().map(|subject| (*subject).to_owned()))
                        .collect(),
                    FindingBasis::EvidencePolicy,
                    &format!(
                        "source correspondence lacks support evidence for subjects {missing:?}"
                    ),
                    "Attach bounded evidence for every subject participating in the correspondence.",
                );
            }
        }
        let reality_contact = claim
            .support
            .iter()
            .map(|id| evidence_reality_contact(prepared.evidence[id.as_str()]))
            .max()
            .unwrap_or(RealityContact::None);
        if claim.verdict == ClaimVerdict::Supported
            && let Some(required) = prepared.scopes[claim.scope.as_str()].required_reality_contact
            && reality_contact < required
        {
            push_finding(
                findings,
                Severity::Error,
                "claim_grounding_below_scope_requirement",
                vec![claim.id.clone(), claim.scope.clone()],
                FindingBasis::EvidencePolicy,
                &format!(
                    "claim reality contact {reality_contact:?} is below required {required:?}"
                ),
                "Attach evidence with the required reality contact or weaken the claim/scope requirement.",
            );
        }
        if claim.verdict == ClaimVerdict::Supported {
            match claim.epistemic_class {
                EpistemicClass::TheoremInstance | EpistemicClass::ExactCheckedModel => {
                    if let Some(checker) = claim.checker.as_ref() {
                        let has_result_evidence = claim.support.iter().any(|evidence_id| {
                            let evidence = prepared.evidence[evidence_id.as_str()];
                            evidence.sha256 == checker.result_sha256
                                && evidence.subject == claim.subject
                                && matches!(
                                    evidence.kind,
                                    EvidenceKind::Proof | EvidenceKind::Generated
                                )
                        });
                        if !has_result_evidence {
                            push_finding(
                                findings,
                                Severity::Blocker,
                                "exact_claim_result_evidence_missing",
                                vec![claim.id.clone()],
                                FindingBasis::EvidencePolicy,
                                "exact claim checker result is not content-bound as same-subject proof/generated evidence",
                                "Attach the exact result artifact as proof/generated evidence whose SHA-256 matches the checker receipt.",
                            );
                        }
                        if claim.epistemic_class == EpistemicClass::ExactCheckedModel {
                            let input_projection = model.projections.iter().find(|projection| {
                                projection.claim_ids.contains(&claim.id)
                                    && projection_target_sha256(projection) == checker.input_sha256
                            });
                            if let Some(projection) = input_projection {
                                if checker.tool != "kerf"
                                    || checker.operation != projection.operation.command()
                                {
                                    push_finding(
                                        findings,
                                        Severity::Blocker,
                                        "exact_claim_checker_identity_mismatch",
                                        vec![claim.id.clone(), projection.id.clone()],
                                        FindingBasis::EvidencePolicy,
                                        "exact claim checker tool/operation disagrees with its Kerf projection",
                                        "Use tool kerf and the exact operation named by the bound projection.",
                                    );
                                }
                            } else {
                                push_finding(
                                    findings,
                                    Severity::Blocker,
                                    "exact_claim_input_projection_missing",
                                    vec![claim.id.clone()],
                                    FindingBasis::EvidencePolicy,
                                    "exact finite-model claim is not bound to a declared projection with the checker input hash",
                                    "Attach the claim to the exact projection whose canonical target input produced the checker receipt.",
                                );
                            }
                        }
                    } else {
                        push_finding(
                            findings,
                            Severity::Blocker,
                            "exact_claim_checker_missing",
                            vec![claim.id.clone()],
                            FindingBasis::EvidencePolicy,
                            "exact or theorem-instance claim has no checker receipt",
                            "Attach a locally checkable result and the exact instantiated hypotheses.",
                        );
                    }
                }
                EpistemicClass::BoundedCheck | EpistemicClass::EvidenceInference => {
                    if claim.support.is_empty() {
                        push_finding(
                            findings,
                            Severity::Error,
                            "supported_claim_evidence_missing",
                            vec![claim.id.clone()],
                            FindingBasis::EvidencePolicy,
                            "bounded or inferred supported claim has no evidence",
                            "Attach scoped evidence or downgrade the verdict to unsupported or unknown.",
                        );
                    }
                }
                EpistemicClass::Heuristic | EpistemicClass::Metaphor => {
                    if matches!(
                        claim.kind,
                        ClaimKind::RequirementSatisfied
                            | ClaimKind::Equivalent
                            | ClaimKind::Factor
                            | ClaimKind::Isomorphism
                            | ClaimKind::Invariant
                            | ClaimKind::SourceCorrespondence
                    ) {
                        push_finding(
                            findings,
                            Severity::Blocker,
                            "heuristic_claim_laundering",
                            vec![claim.id.clone()],
                            FindingBasis::EvidencePolicy,
                            "heuristic or metaphor is presented as a positive feasibility claim",
                            "Use it only to propose a candidate, then add a bounded or exact falsifier.",
                        );
                    }
                }
            }
        }
        if let Some(checker) = &claim.checker {
            let expected = match claim.verdict {
                ClaimVerdict::Supported => Some(CheckerStatus::Accepted),
                ClaimVerdict::Refuted => Some(CheckerStatus::Refuted),
                ClaimVerdict::Unknown => Some(CheckerStatus::Unknown),
                ClaimVerdict::Conflicted | ClaimVerdict::Unsupported | ClaimVerdict::Stale => None,
            };
            if expected.is_some_and(|status| status != checker.status) {
                push_finding(
                    findings,
                    Severity::Blocker,
                    "checker_verdict_mismatch",
                    vec![claim.id.clone()],
                    FindingBasis::ExactStructural,
                    "claim verdict disagrees with its checker receipt",
                    "Use the checker status as the bounded model verdict or replace the receipt.",
                );
            }
        }
        if claim.verdict == ClaimVerdict::Refuted
            && claim.refutation.is_empty()
            && claim.checker.is_none()
        {
            push_finding(
                findings,
                Severity::Error,
                "refutation_witness_missing",
                vec![claim.id.clone()],
                FindingBasis::EvidencePolicy,
                "refuted claim has neither refutation evidence nor a checker receipt",
                "Attach the counterexample, observation, or local checker result.",
            );
        }
        for other_id in &claim.conflicts_with {
            let pair = if claim.id < *other_id {
                (claim.id.clone(), other_id.clone())
            } else {
                (other_id.clone(), claim.id.clone())
            };
            if !conflict_pairs.insert(pair.clone()) {
                continue;
            }
            let other = prepared.claims[other_id.as_str()];
            if claim.subject == other.subject && claim.scope == other.scope {
                let both_positive = claim.verdict == ClaimVerdict::Supported
                    && other.verdict == ClaimVerdict::Supported;
                push_finding(
                    findings,
                    if both_positive {
                        Severity::Blocker
                    } else {
                        Severity::Warning
                    },
                    "claim_conflict",
                    vec![pair.0, pair.1],
                    FindingBasis::DeclaredModel,
                    "explicitly incompatible claims overlap in subject and scope",
                    "Acquire a discriminating observation or revise the subject, scope, or model.",
                );
            }
        }
    }
}

fn projection_target_sha256(projection: &KerfProjection) -> String {
    let bytes = serde_json::to_vec(&projection.target_input)
        .expect("serializing a serde_json::Value cannot fail");
    sha256_hex(&bytes)
}

fn analyze_projections(
    model: &SemanticModel,
    prepared: &Prepared<'_>,
    findings: &mut Vec<Finding>,
) {
    for projection in &model.projections {
        let subject = prepared.subjects[projection.subject.as_str()];
        if subject.kind == SubjectKind::Implementation && projection.closure_ids.is_empty() {
            push_finding(
                findings,
                Severity::Error,
                "implementation_projection_closure_missing",
                vec![projection.id.clone()],
                FindingBasis::EvidencePolicy,
                "implementation projection has no closure claim for its modeled universe",
                "State whether the projected domain is exhaustive, over-approximated, corpus-complete, observed, sampled, or unknown.",
            );
        }
    }
}

fn analyze_work_cycles(model: &SemanticModel, findings: &mut Vec<Finding>) {
    let items: Vec<WorkItemOutput> = model
        .work_items
        .iter()
        .map(|item| WorkItemOutput {
            id: item.id.clone(),
            origin: WorkOrigin::Declared,
            kind: WorkKind::Implementation,
            priority: 1,
            title: item.intent.clone(),
            intent: item.intent.clone(),
            changes: Vec::new(),
            satisfies: Vec::new(),
            evidence_gates: Vec::new(),
            resolves_findings: Vec::new(),
            cleanup_obligation: None,
            external_binding: None,
        })
        .collect();
    let edges: Vec<WorkEdge> = model
        .work_items
        .iter()
        .flat_map(|item| {
            item.depends_on.iter().map(|dependency| WorkEdge {
                work: item.id.clone(),
                depends_on: dependency.clone(),
            })
        })
        .collect();
    if !work_graph_is_acyclic(&items, &edges) {
        push_finding(
            findings,
            Severity::Blocker,
            "work_dependency_cycle",
            model
                .work_items
                .iter()
                .map(|item| item.id.clone())
                .collect(),
            FindingBasis::ExactStructural,
            "declared work dependencies contain a cycle",
            "Break the cycle by exposing the missing shared prerequisite or selecting one canonical owner.",
        );
    }
}

fn add_requirement_dependencies(
    model: &SemanticModel,
    findings: &[Finding],
    generated: &BTreeMap<String, String>,
    edges: &mut BTreeSet<WorkEdge>,
) {
    for requirement in model
        .entities
        .iter()
        .filter(|entity| is_requirement(entity))
    {
        let by_code = |code: &str| {
            findings
                .iter()
                .find(|finding| finding.code == code && finding.subjects.contains(&requirement.id))
                .and_then(|finding| generated.get(&finding.id))
        };
        let observation = by_code("requirement_observation_missing");
        let realization = by_code("requirement_realization_missing");
        let acceptance = by_code("requirement_acceptance_missing");
        let work = by_code("requirement_work_missing");
        for dependent in [realization, acceptance].into_iter().flatten() {
            if let Some(prerequisite) = observation {
                edges.insert(WorkEdge {
                    work: dependent.clone(),
                    depends_on: prerequisite.clone(),
                });
            }
        }
        if let Some(work) = work {
            for prerequisite in [realization, acceptance].into_iter().flatten() {
                edges.insert(WorkEdge {
                    work: work.clone(),
                    depends_on: prerequisite.clone(),
                });
            }
        }
    }
}

fn add_declared_work_dependencies(
    model: &SemanticModel,
    findings: &[Finding],
    generated: &BTreeMap<String, String>,
    edges: &mut BTreeSet<WorkEdge>,
) {
    for item in &model.work_items {
        for finding in findings {
            let Some(obligation) = generated.get(&finding.id) else {
                continue;
            };
            let touches_change = finding
                .subjects
                .iter()
                .any(|subject| item.changes.contains(subject));
            let touches_requirement = finding
                .subjects
                .iter()
                .any(|subject| item.satisfies.contains(subject));
            let touches_gate = finding
                .subjects
                .iter()
                .any(|subject| item.evidence_gates.contains(subject));
            if touches_change || touches_requirement || touches_gate {
                edges.insert(WorkEdge {
                    work: item.id.clone(),
                    depends_on: obligation.clone(),
                });
            }
        }
    }
}

fn work_graph_is_acyclic(items: &[WorkItemOutput], edges: &[WorkEdge]) -> bool {
    let mut indegree: BTreeMap<&str, usize> = items
        .iter()
        .map(|item| (item.id.as_str(), 0_usize))
        .collect();
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        if indegree.contains_key(edge.work.as_str())
            && indegree.contains_key(edge.depends_on.as_str())
        {
            *indegree
                .get_mut(edge.work.as_str())
                .expect("checked work ID exists") += 1;
            outgoing
                .entry(edge.depends_on.as_str())
                .or_default()
                .push(edge.work.as_str());
        }
    }
    let mut ready: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    let mut visited = 0_usize;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(dependents) = outgoing.get(id) {
            for dependent in dependents {
                let degree = indegree
                    .get_mut(dependent)
                    .expect("checked dependent ID exists");
                *degree -= 1;
                if *degree == 0 {
                    ready.push_back(dependent);
                }
            }
        }
    }
    visited == items.len()
}

#[allow(clippy::too_many_lines)]
fn canonical_model_sha256(model: &SemanticModel) -> Result<String, ToolError> {
    let mut normalized = model.clone();
    normalized
        .subjects
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .scopes
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .entities
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .relations
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .bindings
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .evidence
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .closures
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .claims
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .work_items
        .sort_by(|left, right| left.id.cmp(&right.id));
    normalized
        .projections
        .sort_by(|left, right| left.id.cmp(&right.id));
    for scope in &mut normalized.scopes {
        scope.contexts.sort();
        scope.workloads.sort();
    }
    for relation in &mut normalized.relations {
        relation.tails.sort();
        relation.heads.sort();
        relation.evidence.sort();
    }
    for binding in &mut normalized.bindings {
        binding.alphabet.sort_by(|left, right| {
            left.role
                .cmp(&right.role)
                .then_with(|| left.model_symbol.cmp(&right.model_symbol))
        });
        binding.normalization.sort();
        binding
            .negative_controls
            .sort_by(|left, right| left.id.cmp(&right.id));
    }
    for evidence in &mut normalized.evidence {
        evidence.derived_from.sort();
        evidence.coverage.contexts.sort();
        evidence.coverage.workloads.sort();
        evidence.coverage.limitations.sort();
    }
    for claim in &mut normalized.claims {
        claim.subjects.sort();
        claim.support.sort();
        claim.refutation.sort();
        claim.assumptions.sort();
        claim.conflicts_with.sort();
        claim.nonclaims.sort();
    }
    for entity in &mut normalized.entities {
        match &mut entity.kind {
            EntityKind::Requirement {
                acceptance_claims, ..
            } => acceptance_claims.sort(),
            EntityKind::Authority { writers, .. } => writers.sort(),
            EntityKind::DerivedView { invalidated_by, .. } => invalidated_by.sort(),
            EntityKind::Observation { oracle_evidence } => oracle_evidence.sort(),
            EntityKind::Carrier { .. }
            | EntityKind::Action
            | EntityKind::Transition
            | EntityKind::Effect { .. }
            | EntityKind::Port { .. }
            | EntityKind::Contract { .. }
            | EntityKind::Actor { .. }
            | EntityKind::Implementation { .. }
            | EntityKind::Invariant
            | EntityKind::Resource { .. } => {}
        }
    }
    for closure in &mut normalized.closures {
        closure.evidence.sort();
    }
    for work in &mut normalized.work_items {
        work.changes.sort();
        work.satisfies.sort();
        work.depends_on.sort();
        work.evidence_gates.sort();
    }
    for projection in &mut normalized.projections {
        projection.scope_entities.sort();
        projection
            .entity_map
            .sort_by(|left, right| left.source.cmp(&right.source));
        projection
            .relation_map
            .sort_by(|left, right| left.source.cmp(&right.source));
        projection
            .omitted_entities
            .sort_by(|left, right| left.id.cmp(&right.id));
        projection
            .omitted_relations
            .sort_by(|left, right| left.id.cmp(&right.id));
        projection.claim_ids.sort();
        projection.evidence_ids.sort();
        projection.closure_ids.sort();
        projection.assumptions.sort();
    }
    let bytes = serde_json::to_vec(&normalized).map_err(|error| {
        ToolError::internal(format!("could not encode semantic model: {error}"))
    })?;
    Ok(sha256_hex(&bytes))
}

fn validate_scope_binding(
    subject: &str,
    scope_id: &str,
    scopes: &BTreeMap<&str, &Scope>,
) -> Result<(), ToolError> {
    let scope = scopes
        .get(scope_id)
        .ok_or_else(|| ToolError::invalid(format!("unknown scope reference {scope_id:?}")))?;
    if scope.subject != subject {
        return Err(ToolError::invalid(format!(
            "scope {scope_id:?} belongs to subject {:?}, not {subject:?}",
            scope.subject
        )));
    }
    Ok(())
}

fn validate_maps<'a, T>(
    field: &str,
    mappings: &[ProjectionMap],
    index: &BTreeMap<&'a str, &'a T>,
) -> Result<Vec<String>, ToolError> {
    let mut sources = BTreeSet::new();
    let mut pointers = BTreeSet::new();
    for mapping in mappings {
        require_nonempty(&format!("{field}.source"), &mapping.source)?;
        require_nonempty(&format!("{field}.target_pointer"), &mapping.target_pointer)?;
        require_reference(field, &mapping.source, index)?;
        if !sources.insert(mapping.source.as_str()) {
            return Err(ToolError::invalid(format!(
                "{field} repeats source {:?}",
                mapping.source
            )));
        }
        if !pointers.insert(mapping.target_pointer.as_str()) {
            return Err(ToolError::invalid(format!(
                "{field} repeats target_pointer {:?}",
                mapping.target_pointer
            )));
        }
    }
    Ok(sources.into_iter().map(str::to_owned).collect())
}

fn validate_omissions<'a, T>(
    field: &str,
    omissions: &[Omission],
    index: &BTreeMap<&'a str, &'a T>,
) -> Result<Vec<String>, ToolError> {
    let mut ids = BTreeSet::new();
    for omission in omissions {
        require_reference(field, &omission.id, index)?;
        require_nonempty(&format!("{field}.reason"), &omission.reason)?;
        if !ids.insert(omission.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "{field} repeats ID {:?}",
                omission.id
            )));
        }
    }
    Ok(ids.into_iter().map(str::to_owned).collect())
}

fn require_exact_partition(
    field: &str,
    universe: &[String],
    mapped: &[String],
    omitted: &[String],
) -> Result<(), ToolError> {
    let universe: BTreeSet<&str> = universe.iter().map(String::as_str).collect();
    let mapped: BTreeSet<&str> = mapped.iter().map(String::as_str).collect();
    let omitted: BTreeSet<&str> = omitted.iter().map(String::as_str).collect();
    let overlap: Vec<&str> = mapped.intersection(&omitted).copied().collect();
    if !overlap.is_empty() {
        return Err(ToolError::invalid(format!(
            "{field} maps and omits the same IDs: {overlap:?}"
        )));
    }
    let covered: BTreeSet<&str> = mapped.union(&omitted).copied().collect();
    if covered != universe {
        let missing: Vec<&str> = universe.difference(&covered).copied().collect();
        let extra: Vec<&str> = covered.difference(&universe).copied().collect();
        return Err(ToolError::invalid(format!(
            "{field} is not exact; missing={missing:?}, extra={extra:?}"
        )));
    }
    Ok(())
}

fn index_by_id<'a, T>(
    name: &str,
    values: &'a [T],
    id: impl Fn(&'a T) -> &'a str,
) -> Result<BTreeMap<&'a str, &'a T>, ToolError> {
    let mut result = BTreeMap::new();
    for value in values {
        let item_id = id(value);
        require_nonempty(&format!("{name} ID"), item_id)?;
        if result.insert(item_id, value).is_some() {
            return Err(ToolError::invalid(format!(
                "duplicate {name} ID {item_id:?}"
            )));
        }
    }
    Ok(result)
}

fn require_reference<T>(
    field: &str,
    id: &str,
    index: &BTreeMap<&str, &T>,
) -> Result<(), ToolError> {
    if !index.contains_key(id) {
        return Err(ToolError::invalid(format!(
            "{field} references unknown ID {id:?}"
        )));
    }
    Ok(())
}

fn require_references<T>(
    field: &str,
    ids: &[String],
    index: &BTreeMap<&str, &T>,
) -> Result<(), ToolError> {
    require_unique_strings(field, ids, false)?;
    for id in ids {
        require_reference(field, id, index)?;
    }
    Ok(())
}

fn require_entity_kind_reference(
    field: &str,
    id: &str,
    entities: &BTreeMap<&str, &Entity>,
    predicate: fn(&Entity) -> bool,
    expected: &str,
) -> Result<(), ToolError> {
    require_reference(field, id, entities)?;
    if !predicate(entities[id]) {
        return Err(ToolError::invalid(format!(
            "{field} expects {expected}, found entity {id:?}"
        )));
    }
    Ok(())
}

fn require_entity_kind_references(
    field: &str,
    ids: &[String],
    entities: &BTreeMap<&str, &Entity>,
    predicate: fn(&Entity) -> bool,
    expected: &str,
) -> Result<(), ToolError> {
    require_unique_strings(field, ids, false)?;
    for id in ids {
        require_entity_kind_reference(field, id, entities, predicate, expected)?;
    }
    Ok(())
}

fn require_same_subject(
    field: &str,
    owner: &Entity,
    referenced_id: &str,
    entities: &BTreeMap<&str, &Entity>,
) -> Result<(), ToolError> {
    let referenced = entities[referenced_id];
    if referenced.subject != owner.subject {
        return Err(ToolError::invalid(format!(
            "{field} on entity {:?} crosses from subject {:?} to {:?}",
            owner.id, owner.subject, referenced.subject
        )));
    }
    Ok(())
}

fn require_acyclic(field: &str, graph: &BTreeMap<&str, Vec<&str>>) -> Result<(), ToolError> {
    let mut indegree: BTreeMap<&str, usize> =
        graph.keys().copied().map(|id| (id, 0_usize)).collect();
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (node, dependencies) in graph {
        for dependency in dependencies {
            *indegree
                .get_mut(node)
                .expect("graph node was indexed before its references") += 1;
            outgoing.entry(dependency).or_default().push(node);
        }
    }
    let mut ready: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    let mut visited = 0_usize;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(dependents) = outgoing.get(id) {
            for dependent in dependents {
                let degree = indegree
                    .get_mut(dependent)
                    .expect("referential integrity was validated first");
                *degree -= 1;
                if *degree == 0 {
                    ready.push_back(dependent);
                }
            }
        }
    }
    if visited != graph.len() {
        let cyclic: Vec<&str> = indegree
            .into_iter()
            .filter_map(|(id, degree)| (degree > 0).then_some(id))
            .collect();
        return Err(ToolError::invalid(format!(
            "{field} contains a cycle involving {cyclic:?}"
        )));
    }
    Ok(())
}

fn require_nonempty(field: &str, value: &str) -> Result<(), ToolError> {
    if value.is_empty() {
        return Err(ToolError::invalid(format!("{field} must not be empty")));
    }
    Ok(())
}

fn require_unique_strings(
    field: &str,
    values: &[String],
    allow_empty_list: bool,
) -> Result<(), ToolError> {
    if !allow_empty_list && values.is_empty() {
        return Ok(());
    }
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(field, value)?;
        if !seen.insert(value) {
            return Err(ToolError::invalid(format!(
                "{field} repeats value {value:?}"
            )));
        }
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

fn push_finding(
    findings: &mut Vec<Finding>,
    severity: Severity,
    code: &'static str,
    mut subjects: Vec<String>,
    basis: FindingBasis,
    message: &str,
    next_action: &str,
) {
    subjects.sort();
    subjects.dedup();
    let suffix = if subjects.is_empty() {
        "model".to_owned()
    } else {
        subjects.join("+")
    };
    findings.push(Finding {
        id: format!("finding::{code}::{suffix}"),
        severity,
        code,
        subjects,
        basis,
        message: message.to_owned(),
        next_action: next_action.to_owned(),
    });
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.code.cmp(right.code))
            .then_with(|| left.subjects.cmp(&right.subjects))
    });
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn obligation_id(finding: &Finding) -> String {
    format!("obligation::{}", finding.id.trim_start_matches("finding::"))
}

fn finding_priority(finding: &Finding) -> u8 {
    match finding.severity {
        Severity::Blocker => 0,
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    }
}

fn finding_work_kind(finding: &Finding) -> WorkKind {
    if finding.code.contains("conflict") {
        WorkKind::ContradictionResolution
    } else if finding.code.contains("evidence")
        || finding.code.contains("revision")
        || finding.code.contains("closure")
    {
        WorkKind::Evidence
    } else if finding.code.contains("claim")
        || finding.code.contains("projection")
        || finding.code.contains("checker")
    {
        WorkKind::Proof
    } else if finding.code.contains("requirement_work") {
        WorkKind::Implementation
    } else {
        WorkKind::ModelRefinement
    }
}

fn requirement_subjects(model: &SemanticModel, subjects: &[String]) -> Vec<String> {
    sorted_unique(
        subjects
            .iter()
            .filter(|id| {
                model
                    .entities
                    .iter()
                    .find(|entity| entity.id == id.as_str())
                    .is_some_and(is_requirement)
            })
            .cloned()
            .collect(),
    )
}

fn analysis_nonclaims() -> Vec<&'static str> {
    vec![
        "structural validity does not establish that declared entities, relations, evidence, or claims correspond to a real implementation",
        "analyze validates evidence hashes and checker receipts as declarations; it does not fetch external artifacts or re-execute checker operations",
        "absence in observed or sampled evidence is not impossibility without an applicable closure claim",
        "findings are exact consequences of the supplied model and evidence policy, not universal architecture defects",
        "no scalar confidence, architecture energy, universal shortest program, or globally optimal real codebase is computed",
        "category, physics, biology, chemistry, language, and neuroscience analogies cannot establish feasibility",
    ]
}

fn is_requirement(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Requirement { .. })
}

fn is_carrier(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Carrier { .. })
}

fn is_authority(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Authority { .. })
}

fn is_derived_view(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::DerivedView { .. })
}

fn is_state(entity: &Entity) -> bool {
    is_carrier(entity) || is_derived_view(entity)
}

fn is_transition_like(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Action | EntityKind::Transition)
}

fn is_observation(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Observation { .. })
}

fn is_effect_or_observation(entity: &Entity) -> bool {
    matches!(
        entity.kind,
        EntityKind::Effect { .. } | EntityKind::Observation { .. }
    )
}

fn is_contract(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Contract { .. })
}

fn is_implementation(entity: &Entity) -> bool {
    matches!(entity.kind, EntityKind::Implementation { .. })
}

fn is_behavior_actor(entity: &Entity) -> bool {
    matches!(
        entity.kind,
        EntityKind::Actor { .. }
            | EntityKind::Action
            | EntityKind::Transition
            | EntityKind::Implementation { .. }
    )
}

fn is_semantic_realization(entity: &Entity) -> bool {
    matches!(
        entity.kind,
        EntityKind::Carrier { .. }
            | EntityKind::DerivedView { .. }
            | EntityKind::Action
            | EntityKind::Transition
            | EntityKind::Effect { .. }
            | EntityKind::Port { .. }
            | EntityKind::Contract { .. }
            | EntityKind::Invariant
    )
}

struct NoDuplicateValue(Value);

impl<'de> Deserialize<'de> for NoDuplicateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateVisitor)
    }
}

struct NoDuplicateVisitor;

impl<'de> Visitor<'de> for NoDuplicateVisitor {
    type Value = NoDuplicateValue;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(NoDuplicateValue)
            .ok_or_else(|| E::custom("JSON number must be finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(NoDuplicateValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        NoDuplicateValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = access.next_element::<NoDuplicateValue>()? {
            values.push(value.0);
        }
        Ok(NoDuplicateValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some((key, value)) = access.next_entry::<String, NoDuplicateValue>()? {
            if values.insert(key.clone(), value.0).is_some() {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object member {key:?}"
                )));
            }
        }
        Ok(NoDuplicateValue(Value::Object(values)))
    }
}
