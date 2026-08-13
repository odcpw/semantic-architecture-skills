//! Deterministic, proof-obligation-carrying whole-system transformation generation.

mod source;

use crate::cycle::PurposeEnvelope;
use crate::semantic::ClosureKind;
use kerf::candidates::Direction;
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::json;
use source::{SourceInventory, anchors, duplicate_blocks, scan_repository};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const TRANSFORMATION_REQUEST_SCHEMA: &str = "kerf.transformation-request.v1";
pub const TRANSFORMATION_BATCH_SCHEMA: &str = "kerf.transformation-batch.v1";
pub const GENERATOR_REGISTRY_SCHEMA: &str = "kerf.generator-registry.v1";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TransformationFamily {
    CapabilityConstruction,
    BehavioralFactorRealization,
    AuthorityConsolidation,
    IsomorphicSimplification,
    SplitFuseRelocate,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricRole {
    CapabilityCoverage,
    DescriptionCost,
    DuplicateAuthority,
    BoundaryInformation,
    RuntimeCost,
    MemoryCost,
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricIntent {
    pub id: String,
    pub unit: String,
    pub direction: Direction,
    pub role: MetricRole,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityIntent {
    pub id: String,
    pub responsibility: String,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticFactIntent {
    pub id: String,
    pub description: String,
    pub writers: Vec<String>,
    #[serde(default)]
    pub readers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedObservation {
    pub id: String,
    pub statement: String,
    pub capability_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PurposeModel {
    pub purpose_id: String,
    pub capabilities: Vec<CapabilityIntent>,
    #[serde(default)]
    pub facts: Vec<SemanticFactIntent>,
    pub protected_observations: Vec<ProtectedObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentModel {
    pub environment_id: String,
    pub contexts: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HardConstraint {
    pub id: String,
    pub statement: String,
    pub affected_capability_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationPolicy {
    pub allowed_families: Vec<TransformationFamily>,
    pub source_extensions: Vec<String>,
    pub planned_source_root: String,
    pub max_files: usize,
    pub max_bytes_per_file: u64,
    pub max_depth: usize,
    pub minimum_duplicate_lines: usize,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratorSubject {
    Greenfield,
    Brownfield {
        repository_root: String,
        revision: String,
        source_tree_sha256: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationRequest {
    pub schema_version: String,
    pub request_id: String,
    pub search_basis_sha256: String,
    pub subject: GeneratorSubject,
    pub envelope: PurposeEnvelope,
    pub purpose: PurposeModel,
    pub environment: EnvironmentModel,
    pub hard_constraints: Vec<HardConstraint>,
    pub metrics: Vec<MetricIntent>,
    pub policy: TransformationPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemanticWitness {
    PurposeCapability {
        capability_ids: Vec<String>,
        observation_ids: Vec<String>,
    },
    AuthorityConflict {
        fact_id: String,
        writer_ids: Vec<String>,
    },
    BehavioralOverlap {
        capability_ids: Vec<String>,
        observation_ids: Vec<String>,
        fact_ids: Vec<String>,
    },
    RepeatedSource {
        normalized_block_sha256: String,
        normalized_line_count: usize,
        locations: Vec<String>,
    },
    SemanticScattering {
        capability_ids: Vec<String>,
        anchor_tokens: Vec<String>,
        locations: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestructureMode {
    Split,
    Fuse,
    Relocate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransformationOperation {
    CreateModule {
        suggested_path: String,
        capability_id: String,
        contracts: Vec<String>,
    },
    ConsolidateAuthority {
        fact_id: String,
        writer_ids: Vec<String>,
        suggested_path: String,
    },
    RealizeBehavioralFactor {
        capability_ids: Vec<String>,
        shared_observation_ids: Vec<String>,
        suggested_path: String,
    },
    ExtractSharedImplementation {
        normalized_block_sha256: String,
        source_locations: Vec<String>,
        suggested_path: String,
    },
    RestructureModules {
        mode: RestructureMode,
        capability_ids: Vec<String>,
        source_paths: Vec<String>,
        suggested_path: String,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PredictedDirection {
    Better,
    Worse,
    Unchanged,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricPrediction {
    pub direction: PredictedDirection,
    pub mechanism: String,
    pub falsifier: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofObligationKind {
    BuildAndObserve,
    AuthorityUniqueness,
    KerfFactorAndRecompose,
    IsomorphicParity,
    SeamFeasibility,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProofObligation {
    pub id: String,
    pub kind: ProofObligationKind,
    pub checker: String,
    pub required_closure: ClosureKind,
    pub protected_observation_ids: Vec<String>,
    pub falsifier: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationCandidate {
    pub id: String,
    pub family: TransformationFamily,
    pub parent_search_basis_sha256: String,
    pub registry_sha256: String,
    pub summary: String,
    pub witnesses: Vec<SemanticWitness>,
    pub operations: Vec<TransformationOperation>,
    pub predicted_metric_deltas: BTreeMap<String, MetricPrediction>,
    pub proof_obligations: Vec<ProofObligation>,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorDescriptor {
    pub id: String,
    pub family: TransformationFamily,
    pub version_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorRegistry {
    pub schema_version: String,
    pub registry_sha256: String,
    pub generators: Vec<GeneratorDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCoverage {
    pub complete: bool,
    pub truncated: bool,
    pub scanned_files: usize,
    pub skipped_files: Vec<String>,
    pub source_content_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationBatch {
    pub schema_version: String,
    pub request_id: String,
    pub search_basis_sha256: String,
    pub registry: GeneratorRegistry,
    pub coverage: DiscoveryCoverage,
    pub candidates: Vec<TransformationCandidate>,
    pub family_counts: BTreeMap<TransformationFamily, usize>,
    pub nonclaims: Vec<String>,
}

/// Return the content-bound built-in transformation registry.
#[must_use]
pub fn generator_registry() -> GeneratorRegistry {
    let mut source = include_bytes!("generate.rs").to_vec();
    source.extend_from_slice(include_bytes!("generate/source.rs"));
    let registry_sha256 = sha256_hex(&source);
    let generators = all_families()
        .into_iter()
        .map(|family| {
            let identity = format!("kerf.generator-version.v1\0{registry_sha256}\0{family:?}");
            GeneratorDescriptor {
                id: family_id(family).to_owned(),
                family,
                version_sha256: sha256_hex(identity.as_bytes()),
            }
        })
        .collect();
    GeneratorRegistry {
        schema_version: GENERATOR_REGISTRY_SCHEMA.to_owned(),
        registry_sha256,
        generators,
    }
}

/// Hash the order-normalized purpose model for `PurposeEnvelope` authoring.
///
/// # Errors
///
/// Returns [`ToolError`] if serialization fails.
pub fn purpose_sha256(purpose: &PurposeModel) -> Result<String, ToolError> {
    canonical_sha256(&normalized_purpose(purpose), "purpose model")
}

/// Hash the order-normalized environment model for envelope authoring.
///
/// # Errors
///
/// Returns [`ToolError`] if serialization fails.
pub fn environment_sha256(environment: &EnvironmentModel) -> Result<String, ToolError> {
    canonical_sha256(&normalized_environment(environment), "environment model")
}

/// Hash the order-normalized hard constraints for envelope authoring.
///
/// # Errors
///
/// Returns [`ToolError`] if serialization fails.
pub fn hard_constraints_sha256(constraints: &[HardConstraint]) -> Result<String, ToolError> {
    let normalized = constraints
        .iter()
        .map(|constraint| (constraint.id.as_str(), constraint))
        .collect::<BTreeMap<_, _>>();
    canonical_sha256(&normalized, "hard constraints")
}

/// Hash the order-normalized metric policy for envelope authoring.
///
/// # Errors
///
/// Returns [`ToolError`] if serialization fails.
pub fn metric_policy_sha256(metrics: &[MetricIntent]) -> Result<String, ToolError> {
    let normalized = metrics
        .iter()
        .map(|metric| (metric.id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    canonical_sha256(&normalized, "metric policy")
}

/// Hash the order-normalized transformation policy for envelope authoring.
///
/// # Errors
///
/// Returns [`ToolError`] if serialization fails.
pub fn transformation_policy_sha256(policy: &TransformationPolicy) -> Result<String, ToolError> {
    let mut normalized = policy.clone();
    normalized.allowed_families.sort();
    normalized.allowed_families.dedup();
    normalized.source_extensions.sort();
    normalized.source_extensions.dedup();
    canonical_sha256(&normalized, "transformation policy")
}

/// Produce deterministic semantic transformation candidates without requiring
/// a caller-named seam.
///
/// # Errors
///
/// Returns [`ToolError`] when the envelope does not bind its supplied content,
/// the purpose graph is malformed, repository discovery exceeds a hard bound,
/// or a generated candidate lacks a semantic witness or proof obligation.
pub fn generate_transformations(
    request: &TransformationRequest,
) -> Result<TransformationBatch, ToolError> {
    validate_request(request)?;
    let registry = generator_registry();
    let allowed = request
        .policy
        .allowed_families
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let inventory = match &request.subject {
        GeneratorSubject::Greenfield => None,
        GeneratorSubject::Brownfield {
            repository_root, ..
        } => Some(scan_repository(
            Path::new(repository_root),
            &request.policy,
        )?),
    };
    let mut candidates = collect_candidates(
        request,
        inventory.as_ref(),
        &registry.registry_sha256,
        &allowed,
    )?;
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    candidates.dedup_by(|left, right| left.id == right.id);
    let truncated = candidates.len() > request.policy.max_candidates;
    candidates.truncate(request.policy.max_candidates);
    for candidate in &candidates {
        validate_candidate(request, &registry.registry_sha256, candidate)?;
    }
    let family_counts = candidates
        .iter()
        .fold(BTreeMap::new(), |mut counts, candidate| {
            *counts.entry(candidate.family).or_insert(0) += 1;
            counts
        });
    let greenfield_content_sha256 = purpose_sha256(&request.purpose)?;
    let (scanned_files, skipped_files, source_content_sha256) = inventory.as_ref().map_or_else(
        || (0, Vec::new(), greenfield_content_sha256),
        |inventory| {
            (
                inventory.files.len(),
                inventory.skipped.clone(),
                inventory.content_sha256.clone(),
            )
        },
    );
    let complete = !truncated && skipped_files.is_empty();
    Ok(TransformationBatch {
        schema_version: TRANSFORMATION_BATCH_SCHEMA.to_owned(),
        request_id: request.request_id.clone(),
        search_basis_sha256: request.search_basis_sha256.clone(),
        registry,
        coverage: DiscoveryCoverage {
            complete,
            truncated,
            scanned_files,
            skipped_files,
            source_content_sha256,
        },
        candidates,
        family_counts,
        nonclaims: vec![
            "Generated transformations are heuristic candidates carrying proof obligations, not accepted improvements.".to_owned(),
            "Lexical source correspondence and repeated normalized blocks do not establish semantic equivalence.".to_owned(),
            "Discovery completeness is bounded by the supplied purpose model, transformation families, source extensions, and resource policy.".to_owned(),
            "Qualitative metric deltas are prospective predictions; measurement and independent verification decide acceptance.".to_owned(),
        ],
    })
}

/// Revalidate a persisted transformation batch against its exact request and
/// the currently executing generator registry.
///
/// # Errors
///
/// Returns [`ToolError`] when identities, counts, candidates, witnesses,
/// metric predictions, proof obligations, or nonclaims are inconsistent.
pub fn validate_transformation_batch(
    request: &TransformationRequest,
    batch: &TransformationBatch,
) -> Result<(), ToolError> {
    validate_request(request)?;
    let registry = generator_registry();
    if batch.schema_version != TRANSFORMATION_BATCH_SCHEMA
        || batch.request_id != request.request_id
        || batch.search_basis_sha256 != request.search_basis_sha256
        || batch.registry.schema_version != registry.schema_version
        || batch.registry.registry_sha256 != registry.registry_sha256
        || canonical_sha256(&batch.registry, "persisted generator registry")?
            != canonical_sha256(&registry, "current generator registry")?
    {
        return Err(ToolError::invalid(
            "transformation batch identities do not match the request and current registry",
        ));
    }
    if batch.nonclaims.is_empty() || batch.nonclaims.iter().any(String::is_empty) {
        return Err(ToolError::invalid(
            "transformation batch requires non-empty nonclaims",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut family_counts = BTreeMap::new();
    for candidate in &batch.candidates {
        if !ids.insert(candidate.id.as_str()) {
            return Err(ToolError::invalid(
                "transformation candidate IDs must be unique",
            ));
        }
        validate_candidate(request, &registry.registry_sha256, candidate)?;
        *family_counts.entry(candidate.family).or_insert(0) += 1;
    }
    if family_counts != batch.family_counts {
        return Err(ToolError::invalid(
            "transformation batch family_counts do not match its candidates",
        ));
    }
    let replayed = generate_transformations(request)?;
    if canonical_sha256(batch, "persisted transformation batch")?
        != canonical_sha256(&replayed, "replayed transformation batch")?
    {
        return Err(ToolError::invalid(
            "transformation batch does not match deterministic generation against the current request, registry, and source inventory",
        ));
    }
    Ok(())
}

fn collect_candidates(
    request: &TransformationRequest,
    inventory: Option<&SourceInventory>,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
) -> Result<Vec<TransformationCandidate>, ToolError> {
    let capability_locations = inventory.map_or_else(BTreeMap::new, |inventory| {
        locate_capabilities(&request.purpose, inventory)
    });
    let mut candidates = Vec::new();
    generate_capabilities(
        request,
        inventory,
        &capability_locations,
        registry_sha256,
        allowed,
        &mut candidates,
    )?;
    generate_authorities(request, registry_sha256, allowed, &mut candidates)?;
    generate_behavioral_factors(request, registry_sha256, allowed, &mut candidates)?;
    if let Some(inventory) = inventory {
        generate_duplicate_extractions(
            request,
            inventory,
            registry_sha256,
            allowed,
            &mut candidates,
        )?;
        generate_restructures(
            request,
            inventory,
            &capability_locations,
            registry_sha256,
            allowed,
            &mut candidates,
        )?;
    }
    Ok(candidates)
}

#[allow(clippy::too_many_lines)]
fn validate_request(request: &TransformationRequest) -> Result<(), ToolError> {
    if request.schema_version != TRANSFORMATION_REQUEST_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {TRANSFORMATION_REQUEST_SCHEMA:?}",
            request.schema_version
        )));
    }
    require_id("request", &request.request_id)?;
    require_sha256("search_basis_sha256", &request.search_basis_sha256)?;
    require_id("purpose", &request.purpose.purpose_id)?;
    require_id("environment", &request.environment.environment_id)?;
    if request.envelope.purpose_sha256 != purpose_sha256(&request.purpose)?
        || request.envelope.environment_sha256 != environment_sha256(&request.environment)?
        || request.envelope.hard_constraints_sha256
            != hard_constraints_sha256(&request.hard_constraints)?
        || request.envelope.metric_policy_sha256 != metric_policy_sha256(&request.metrics)?
        || request.envelope.transformation_policy_sha256
            != transformation_policy_sha256(&request.policy)?
    {
        return Err(ToolError::invalid(
            "purpose envelope hashes do not bind the supplied purpose, environment, constraints, metrics, and transformation policy",
        ));
    }
    if let GeneratorSubject::Brownfield {
        repository_root,
        revision,
        source_tree_sha256,
    } = &request.subject
    {
        if repository_root.is_empty() || revision.is_empty() {
            return Err(ToolError::invalid(
                "brownfield repository_root and revision must not be empty",
            ));
        }
        require_sha256("subject.source_tree_sha256", source_tree_sha256)?;
    }
    if request.purpose.capabilities.is_empty()
        || request.purpose.protected_observations.is_empty()
        || request.environment.contexts.is_empty()
        || request.hard_constraints.is_empty()
        || request.metrics.is_empty()
    {
        return Err(ToolError::invalid(
            "purpose capabilities, protected observations, environment contexts, hard constraints, and metrics must not be empty",
        ));
    }
    let capability_ids = unique_ids(
        "capability",
        request
            .purpose
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str()),
    )?;
    let fact_ids = unique_ids(
        "fact",
        request.purpose.facts.iter().map(|fact| fact.id.as_str()),
    )?;
    for capability in &request.purpose.capabilities {
        if capability.responsibility.is_empty() {
            return Err(ToolError::invalid(format!(
                "capability {:?} needs a responsibility",
                capability.id
            )));
        }
        for fact in capability.reads.iter().chain(&capability.writes) {
            if !fact_ids.contains(fact.as_str()) {
                return Err(ToolError::invalid(format!(
                    "capability {:?} references unknown fact {fact:?}",
                    capability.id
                )));
            }
        }
    }
    for fact in &request.purpose.facts {
        if fact.description.is_empty() || fact.writers.is_empty() {
            return Err(ToolError::invalid(format!(
                "fact {:?} needs a description and at least one writer",
                fact.id
            )));
        }
        for capability in fact.writers.iter().chain(&fact.readers) {
            if !capability_ids.contains(capability.as_str()) {
                return Err(ToolError::invalid(format!(
                    "fact {:?} references unknown capability {capability:?}",
                    fact.id
                )));
            }
        }
    }
    let observation_ids = unique_ids(
        "protected observation",
        request
            .purpose
            .protected_observations
            .iter()
            .map(|observation| observation.id.as_str()),
    )?;
    for observation in &request.purpose.protected_observations {
        if observation.statement.is_empty() || observation.capability_ids.is_empty() {
            return Err(ToolError::invalid(format!(
                "protected observation {:?} needs a statement and capabilities",
                observation.id
            )));
        }
        require_known_capabilities(&capability_ids, &observation.capability_ids)?;
    }
    let constraint_ids = unique_ids(
        "hard constraint",
        request
            .hard_constraints
            .iter()
            .map(|constraint| constraint.id.as_str()),
    )?;
    for constraint in &request.hard_constraints {
        if constraint.statement.is_empty() || constraint.affected_capability_ids.is_empty() {
            return Err(ToolError::invalid(format!(
                "hard constraint {:?} needs a statement and affected capabilities",
                constraint.id
            )));
        }
        require_known_capabilities(&capability_ids, &constraint.affected_capability_ids)?;
    }
    let _ = (observation_ids, constraint_ids);
    let metric_ids = unique_ids(
        "metric",
        request.metrics.iter().map(|metric| metric.id.as_str()),
    )?;
    if request.metrics.iter().any(|metric| metric.unit.is_empty()) || metric_ids.is_empty() {
        return Err(ToolError::invalid(
            "metrics need unique IDs and non-empty units",
        ));
    }
    let families = request
        .policy
        .allowed_families
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if families.is_empty() || families.len() != request.policy.allowed_families.len() {
        return Err(ToolError::invalid(
            "allowed_families must be non-empty and unique",
        ));
    }
    let extensions = request
        .policy
        .source_extensions
        .iter()
        .map(|extension| extension.trim_start_matches('.'))
        .collect::<BTreeSet<_>>();
    if request.policy.planned_source_root.is_empty()
        || extensions.is_empty()
        || extensions.contains("")
        || request.policy.max_files == 0
        || request.policy.max_bytes_per_file == 0
        || request.policy.max_depth == 0
        || request.policy.minimum_duplicate_lines < 2
        || request.policy.max_candidates == 0
    {
        return Err(ToolError::invalid(
            "transformation policy needs a source root, extensions, positive bounds, duplicate_lines >= 2, and max_candidates > 0",
        ));
    }
    Ok(())
}

fn validate_candidate(
    request: &TransformationRequest,
    registry_sha256: &str,
    candidate: &TransformationCandidate,
) -> Result<(), ToolError> {
    require_id("candidate", &candidate.id)?;
    if candidate.parent_search_basis_sha256 != request.search_basis_sha256
        || candidate.registry_sha256 != registry_sha256
    {
        return Err(ToolError::invalid(format!(
            "candidate {:?} is not bound to the request and registry",
            candidate.id
        )));
    }
    if candidate.summary.is_empty()
        || candidate.witnesses.is_empty()
        || candidate.operations.is_empty()
        || candidate.proof_obligations.is_empty()
        || candidate.nonclaims.is_empty()
    {
        return Err(ToolError::invalid(format!(
            "candidate {:?} requires a summary, semantic witness, operation, proof obligation, and nonclaim",
            candidate.id
        )));
    }
    let expected_metrics = request
        .metrics
        .iter()
        .map(|metric| metric.id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_metrics = candidate
        .predicted_metric_deltas
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if expected_metrics != observed_metrics {
        return Err(ToolError::invalid(format!(
            "candidate {:?} must predict every declared metric",
            candidate.id
        )));
    }
    if candidate
        .predicted_metric_deltas
        .values()
        .any(|prediction| prediction.mechanism.is_empty() || prediction.falsifier.is_empty())
    {
        return Err(ToolError::invalid(format!(
            "candidate {:?} has a metric prediction without mechanism or falsifier",
            candidate.id
        )));
    }
    let protected = request
        .purpose
        .protected_observations
        .iter()
        .map(|observation| observation.id.as_str())
        .collect::<BTreeSet<_>>();
    for obligation in &candidate.proof_obligations {
        require_id("proof obligation", &obligation.id)?;
        if obligation.checker.is_empty()
            || obligation.falsifier.is_empty()
            || obligation.protected_observation_ids.is_empty()
            || obligation
                .protected_observation_ids
                .iter()
                .any(|id| !protected.contains(id.as_str()))
        {
            return Err(ToolError::invalid(format!(
                "candidate {:?} proof obligations must bind a checker, falsifier, and known protected observations",
                candidate.id
            )));
        }
    }
    validate_witnesses(candidate)?;
    let mut normalized = candidate.clone();
    normalized.id.clear();
    let identity = canonical_sha256(&normalized, "transformation candidate")?;
    let expected_id = format!("{}-{}", family_id(candidate.family), &identity[..16]);
    if candidate.id != expected_id {
        return Err(ToolError::invalid(format!(
            "candidate {:?} does not match its content-derived ID {expected_id:?}",
            candidate.id
        )));
    }
    Ok(())
}

fn validate_witnesses(candidate: &TransformationCandidate) -> Result<(), ToolError> {
    for witness in &candidate.witnesses {
        let valid = match witness {
            SemanticWitness::PurposeCapability {
                capability_ids,
                observation_ids,
            } => !capability_ids.is_empty() && !observation_ids.is_empty(),
            SemanticWitness::AuthorityConflict {
                fact_id,
                writer_ids,
            } => !fact_id.is_empty() && writer_ids.len() >= 2,
            SemanticWitness::BehavioralOverlap {
                capability_ids,
                observation_ids,
                fact_ids,
            } => capability_ids.len() >= 2 && (!observation_ids.is_empty() || !fact_ids.is_empty()),
            SemanticWitness::RepeatedSource {
                normalized_block_sha256,
                normalized_line_count,
                locations,
            } => {
                require_sha256("normalized_block_sha256", normalized_block_sha256)?;
                *normalized_line_count >= 2 && locations.len() >= 2
            }
            SemanticWitness::SemanticScattering {
                capability_ids,
                anchor_tokens,
                locations,
            } => !capability_ids.is_empty() && !anchor_tokens.is_empty() && locations.len() >= 2,
        };
        if !valid {
            return Err(ToolError::invalid(format!(
                "candidate {:?} has a non-semantic or empty witness; file size alone is never admissible",
                candidate.id
            )));
        }
    }
    Ok(())
}

fn generate_capabilities(
    request: &TransformationRequest,
    inventory: Option<&SourceInventory>,
    locations: &BTreeMap<String, BTreeSet<String>>,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
    output: &mut Vec<TransformationCandidate>,
) -> Result<(), ToolError> {
    if !allowed.contains(&TransformationFamily::CapabilityConstruction) {
        return Ok(());
    }
    for capability in &request.purpose.capabilities {
        if inventory.is_some()
            && locations
                .get(&capability.id)
                .is_some_and(|paths| !paths.is_empty())
        {
            continue;
        }
        let observation_ids = observations_for(request, &[capability.id.as_str()]);
        let operation = TransformationOperation::CreateModule {
            suggested_path: suggested_path(
                &request.policy,
                &format!("{}.rs", safe_component(&capability.id)),
            ),
            capability_id: capability.id.clone(),
            contracts: observation_ids.clone(),
        };
        output.push(build_candidate(
            request,
            registry_sha256,
            TransformationFamily::CapabilityConstruction,
            format!(
                "Construct capability {} as an observable module",
                capability.id
            ),
            vec![SemanticWitness::PurposeCapability {
                capability_ids: vec![capability.id.clone()],
                observation_ids,
            }],
            vec![operation],
        )?);
    }
    Ok(())
}

fn generate_authorities(
    request: &TransformationRequest,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
    output: &mut Vec<TransformationCandidate>,
) -> Result<(), ToolError> {
    if !allowed.contains(&TransformationFamily::AuthorityConsolidation) {
        return Ok(());
    }
    for fact in &request.purpose.facts {
        let writers = fact.writers.iter().cloned().collect::<BTreeSet<_>>();
        if writers.len() < 2 {
            continue;
        }
        let writer_ids = writers.into_iter().collect::<Vec<_>>();
        output.push(build_candidate(
            request,
            registry_sha256,
            TransformationFamily::AuthorityConsolidation,
            format!(
                "Consolidate {} writers behind one authority protocol",
                fact.id
            ),
            vec![SemanticWitness::AuthorityConflict {
                fact_id: fact.id.clone(),
                writer_ids: writer_ids.clone(),
            }],
            vec![TransformationOperation::ConsolidateAuthority {
                fact_id: fact.id.clone(),
                writer_ids,
                suggested_path: suggested_path(
                    &request.policy,
                    &format!("{}_authority.rs", safe_component(&fact.id)),
                ),
            }],
        )?);
    }
    Ok(())
}

fn generate_behavioral_factors(
    request: &TransformationRequest,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
    output: &mut Vec<TransformationCandidate>,
) -> Result<(), ToolError> {
    if !allowed.contains(&TransformationFamily::BehavioralFactorRealization) {
        return Ok(());
    }
    for (index, left) in request.purpose.capabilities.iter().enumerate() {
        for right in request.purpose.capabilities.iter().skip(index + 1) {
            let capability_ids = vec![left.id.clone(), right.id.clone()];
            let observation_ids = observations_for(request, &[&left.id, &right.id]);
            let fact_ids = shared_facts(left, right);
            if observation_ids.is_empty() && fact_ids.is_empty() {
                continue;
            }
            output.push(build_candidate(
                request,
                registry_sha256,
                TransformationFamily::BehavioralFactorRealization,
                format!(
                    "Realize the shared behavior of {} and {} as an explicit factor",
                    left.id, right.id
                ),
                vec![SemanticWitness::BehavioralOverlap {
                    capability_ids: capability_ids.clone(),
                    observation_ids: observation_ids.clone(),
                    fact_ids,
                }],
                vec![TransformationOperation::RealizeBehavioralFactor {
                    capability_ids,
                    shared_observation_ids: observation_ids,
                    suggested_path: suggested_path(
                        &request.policy,
                        &format!(
                            "{}_{}_factor.rs",
                            safe_component(&left.id),
                            safe_component(&right.id)
                        ),
                    ),
                }],
            )?);
        }
    }
    Ok(())
}

fn generate_duplicate_extractions(
    request: &TransformationRequest,
    inventory: &SourceInventory,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
    output: &mut Vec<TransformationCandidate>,
) -> Result<(), ToolError> {
    if !allowed.contains(&TransformationFamily::IsomorphicSimplification) {
        return Ok(());
    }
    for block in duplicate_blocks(inventory, request.policy.minimum_duplicate_lines) {
        let suffix = block.sha256.get(..12).unwrap_or(&block.sha256).to_owned();
        let extension = if block.extension.is_empty() {
            "txt".to_owned()
        } else {
            block.extension.clone()
        };
        output.push(build_candidate(
            request,
            registry_sha256,
            TransformationFamily::IsomorphicSimplification,
            format!("Extract repeated normalized implementation block {suffix}"),
            vec![SemanticWitness::RepeatedSource {
                normalized_block_sha256: block.sha256.clone(),
                normalized_line_count: block.normalized_lines.len(),
                locations: block.locations.clone(),
            }],
            vec![TransformationOperation::ExtractSharedImplementation {
                normalized_block_sha256: block.sha256,
                source_locations: block.locations,
                suggested_path: suggested_path(
                    &request.policy,
                    &format!("shared_{suffix}.{extension}"),
                ),
            }],
        )?);
    }
    Ok(())
}

fn generate_restructures(
    request: &TransformationRequest,
    inventory: &SourceInventory,
    locations: &BTreeMap<String, BTreeSet<String>>,
    registry_sha256: &str,
    allowed: &BTreeSet<TransformationFamily>,
    output: &mut Vec<TransformationCandidate>,
) -> Result<(), ToolError> {
    if !allowed.contains(&TransformationFamily::SplitFuseRelocate) {
        return Ok(());
    }
    let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (capability, paths) in locations {
        for path in paths {
            by_file
                .entry(path.clone())
                .or_default()
                .push(capability.clone());
        }
    }
    for (path, mut capability_ids) in by_file {
        capability_ids.sort();
        capability_ids.dedup();
        if capability_ids.len() < 2 {
            continue;
        }
        let file = inventory.files.iter().find(|file| file.path == path);
        let anchor_tokens = capability_ids
            .iter()
            .filter_map(|id| {
                request
                    .purpose
                    .capabilities
                    .iter()
                    .find(|capability| &capability.id == id)
            })
            .flat_map(|capability| {
                anchors(&format!("{} {}", capability.id, capability.responsibility))
            })
            .filter(|token| file.is_some_and(|source| source.tokens.contains(token)))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if anchor_tokens.is_empty() {
            continue;
        }
        output.push(build_candidate(
            request,
            registry_sha256,
            TransformationFamily::SplitFuseRelocate,
            format!("Split semantically distinct capabilities currently co-located in {path}"),
            vec![SemanticWitness::SemanticScattering {
                capability_ids: capability_ids.clone(),
                anchor_tokens,
                locations: vec![path.clone(), "purpose-model".to_owned()],
            }],
            vec![TransformationOperation::RestructureModules {
                mode: RestructureMode::Split,
                capability_ids,
                source_paths: vec![path],
                suggested_path: suggested_path(&request.policy, "split_modules"),
            }],
        )?);
    }
    for (capability_id, paths) in locations {
        if paths.len() < 2 {
            continue;
        }
        let capability = request
            .purpose
            .capabilities
            .iter()
            .find(|capability| capability.id == *capability_id)
            .ok_or_else(|| ToolError::internal("located capability is absent from purpose"))?;
        let anchor_tokens = anchors(&format!("{} {}", capability.id, capability.responsibility))
            .into_iter()
            .collect::<Vec<_>>();
        output.push(build_candidate(
            request,
            registry_sha256,
            TransformationFamily::SplitFuseRelocate,
            format!("Fuse scattered responsibility for capability {capability_id}"),
            vec![SemanticWitness::SemanticScattering {
                capability_ids: vec![capability_id.clone()],
                anchor_tokens,
                locations: paths.iter().cloned().collect(),
            }],
            vec![TransformationOperation::RestructureModules {
                mode: RestructureMode::Fuse,
                capability_ids: vec![capability_id.clone()],
                source_paths: paths.iter().cloned().collect(),
                suggested_path: suggested_path(
                    &request.policy,
                    &format!("{}.rs", safe_component(capability_id)),
                ),
            }],
        )?);
    }
    Ok(())
}

fn locate_capabilities(
    purpose: &PurposeModel,
    inventory: &SourceInventory,
) -> BTreeMap<String, BTreeSet<String>> {
    purpose
        .capabilities
        .iter()
        .map(|capability| {
            let capability_anchors =
                anchors(&format!("{} {}", capability.id, capability.responsibility));
            let paths = inventory
                .files
                .iter()
                .filter(|file| !file.tokens.is_disjoint(&capability_anchors))
                .map(|file| file.path.clone())
                .collect();
            (capability.id.clone(), paths)
        })
        .collect()
}

fn build_candidate(
    request: &TransformationRequest,
    registry_sha256: &str,
    family: TransformationFamily,
    summary: String,
    witnesses: Vec<SemanticWitness>,
    operations: Vec<TransformationOperation>,
) -> Result<TransformationCandidate, ToolError> {
    let observation_ids = request
        .purpose
        .protected_observations
        .iter()
        .map(|observation| observation.id.clone())
        .collect::<Vec<_>>();
    let proof_obligations = proof_obligations(family, &observation_ids);
    let predicted_metric_deltas = request
        .metrics
        .iter()
        .map(|metric| (metric.id.clone(), metric_prediction(family, metric.role)))
        .collect();
    let mut candidate = TransformationCandidate {
        id: String::new(),
        family,
        parent_search_basis_sha256: request.search_basis_sha256.clone(),
        registry_sha256: registry_sha256.to_owned(),
        summary,
        witnesses,
        operations,
        predicted_metric_deltas,
        proof_obligations,
        nonclaims: vec![
            "candidate generation does not establish that the predicted metric direction will occur"
                .to_owned(),
            "application is forbidden until every required proof obligation has an accepted receipt"
                .to_owned(),
        ],
    };
    let identity = canonical_sha256(&candidate, "transformation candidate")?;
    candidate.id = format!("{}-{}", family_id(family), &identity[..16]);
    Ok(candidate)
}

fn proof_obligations(
    family: TransformationFamily,
    observation_ids: &[String],
) -> Vec<ProofObligation> {
    let (kind, checker, closure, falsifier) = match family {
        TransformationFamily::CapabilityConstruction => (
            ProofObligationKind::BuildAndObserve,
            "kerf-evidence execute",
            ClosureKind::CompleteForDeclaredCorpus,
            "the capability fails to build or any protected observation disagrees",
        ),
        TransformationFamily::BehavioralFactorRealization => (
            ProofObligationKind::KerfFactorAndRecompose,
            "kerf factor && kerf recompose",
            ClosureKind::ExhaustiveFinite,
            "factor laws fail or recomposition exposes a mismatch or hole",
        ),
        TransformationFamily::AuthorityConsolidation => (
            ProofObligationKind::AuthorityUniqueness,
            "kerf-model analyze && kerf-evidence execute",
            ClosureKind::CompleteForDeclaredCorpus,
            "more than one writer remains or a protected observation changes",
        ),
        TransformationFamily::IsomorphicSimplification => (
            ProofObligationKind::IsomorphicParity,
            "kerf-evidence expand-parity-corpus && kerf-evidence execute",
            ClosureKind::CompleteForDeclaredCorpus,
            "a protected output, state transition, effect, or negative control differs",
        ),
        TransformationFamily::SplitFuseRelocate => (
            ProofObligationKind::SeamFeasibility,
            "fold_graph.py score && kerf-evidence execute",
            ClosureKind::CompleteForDeclaredCorpus,
            "the boundary is infeasible, duplicates authority, or changes a protected observation",
        ),
    };
    vec![ProofObligation {
        id: format!("verify-{}", family_id(family)),
        kind,
        checker: checker.to_owned(),
        required_closure: closure,
        protected_observation_ids: observation_ids.to_vec(),
        falsifier: falsifier.to_owned(),
    }]
}

fn metric_prediction(family: TransformationFamily, role: MetricRole) -> MetricPrediction {
    let direction = match (family, role) {
        (TransformationFamily::CapabilityConstruction, MetricRole::CapabilityCoverage)
        | (
            TransformationFamily::BehavioralFactorRealization
            | TransformationFamily::IsomorphicSimplification,
            MetricRole::DescriptionCost,
        )
        | (
            TransformationFamily::BehavioralFactorRealization
            | TransformationFamily::SplitFuseRelocate,
            MetricRole::BoundaryInformation,
        )
        | (TransformationFamily::AuthorityConsolidation, MetricRole::DuplicateAuthority) => {
            PredictedDirection::Better
        }
        (TransformationFamily::CapabilityConstruction, MetricRole::DescriptionCost) => {
            PredictedDirection::Worse
        }
        _ => PredictedDirection::Unknown,
    };
    MetricPrediction {
        direction,
        mechanism: match direction {
            PredictedDirection::Better => {
                format!("the {family:?} mechanism directly targets the declared metric role")
            }
            PredictedDirection::Worse => {
                "new capability realization necessarily introduces implementation description"
                    .to_owned()
            }
            PredictedDirection::Unchanged => "the operation does not touch this coordinate".to_owned(),
            PredictedDirection::Unknown => {
                "no sound directional mechanism is known before measurement".to_owned()
            }
        },
        falsifier: "the independently measured candidate metric does not have the predicted product-order relation to the parent"
            .to_owned(),
    }
}

fn observations_for(request: &TransformationRequest, capabilities: &[&str]) -> Vec<String> {
    request
        .purpose
        .protected_observations
        .iter()
        .filter(|observation| {
            capabilities
                .iter()
                .all(|capability| observation.capability_ids.iter().any(|id| id == capability))
        })
        .map(|observation| observation.id.clone())
        .collect()
}

fn shared_facts(left: &CapabilityIntent, right: &CapabilityIntent) -> Vec<String> {
    let left_facts = left
        .reads
        .iter()
        .chain(&left.writes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let right_facts = right
        .reads
        .iter()
        .chain(&right.writes)
        .cloned()
        .collect::<BTreeSet<_>>();
    left_facts.intersection(&right_facts).cloned().collect()
}

fn normalized_purpose(purpose: &PurposeModel) -> serde_json::Value {
    json!({
        "purpose_id": purpose.purpose_id,
        "capabilities": purpose.capabilities.iter().map(|capability| (capability.id.as_str(), capability)).collect::<BTreeMap<_, _>>(),
        "facts": purpose.facts.iter().map(|fact| (fact.id.as_str(), fact)).collect::<BTreeMap<_, _>>(),
        "protected_observations": purpose.protected_observations.iter().map(|observation| (observation.id.as_str(), observation)).collect::<BTreeMap<_, _>>(),
    })
}

fn normalized_environment(environment: &EnvironmentModel) -> serde_json::Value {
    let mut contexts = environment.contexts.clone();
    contexts.sort();
    let mut assumptions = environment.assumptions.clone();
    assumptions.sort();
    json!({
        "environment_id": environment.environment_id,
        "contexts": contexts,
        "assumptions": assumptions,
    })
}

fn require_known_capabilities(
    known: &BTreeSet<&str>,
    supplied: &[String],
) -> Result<(), ToolError> {
    if supplied.is_empty() || supplied.iter().any(|id| !known.contains(id.as_str())) {
        return Err(ToolError::invalid(
            "capability reference set must be non-empty and contain only known capabilities",
        ));
    }
    Ok(())
}

fn unique_ids<'a>(
    kind: &str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeSet<&'a str>, ToolError> {
    let mut result = BTreeSet::new();
    for value in values {
        require_id(kind, value)?;
        if !result.insert(value) {
            return Err(ToolError::invalid(format!("duplicate {kind} ID {value:?}")));
        }
    }
    Ok(result)
}

fn suggested_path(policy: &TransformationPolicy, leaf: &str) -> String {
    format!(
        "{}/{}",
        policy.planned_source_root.trim_end_matches('/'),
        leaf
    )
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

const fn family_id(family: TransformationFamily) -> &'static str {
    match family {
        TransformationFamily::CapabilityConstruction => "capability-construction",
        TransformationFamily::BehavioralFactorRealization => "behavioral-factor",
        TransformationFamily::AuthorityConsolidation => "authority-consolidation",
        TransformationFamily::IsomorphicSimplification => "isomorphic-simplification",
        TransformationFamily::SplitFuseRelocate => "split-fuse-relocate",
    }
}

fn all_families() -> [TransformationFamily; 5] {
    [
        TransformationFamily::CapabilityConstruction,
        TransformationFamily::BehavioralFactorRealization,
        TransformationFamily::AuthorityConsolidation,
        TransformationFamily::IsomorphicSimplification,
        TransformationFamily::SplitFuseRelocate,
    ]
}

fn canonical_sha256<T: Serialize>(value: &T, label: &str) -> Result<String, ToolError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| ToolError::internal(format!("serialize {label}: {error}")))
}

fn require_id(kind: &str, value: &str) -> Result<(), ToolError> {
    if value.is_empty() {
        Err(ToolError::invalid(format!("{kind} IDs must not be empty")))
    } else {
        Ok(())
    }
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
