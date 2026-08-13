//! Content-addressed campaign state and computed whole-system stopping states.

use crate::converge::{EvaluatorIdentity, current_evaluator_identity};
use kerf::candidates::{
    CANDIDATE_SCHEMA, Candidate as KerfCandidate, CandidateInput, Constraints, Direction,
    MetricDefinition as KerfMetricDefinition, pareto,
};
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const CAMPAIGN_STATE_SCHEMA: &str = "kerf.optimization-campaign.v1";
pub const CAMPAIGN_EVALUATION_SCHEMA: &str = "kerf.optimization-evaluation.v1";

pub type DominanceMap = BTreeMap<String, Vec<String>>;

/// A new system and an existing system differ only in their initial subject
/// binding. Both enter the same search, evidence, frontier, and stopping loop.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum CampaignSubject {
    Greenfield {
        subject_id: String,
        intent_model_sha256: String,
    },
    Brownfield {
        subject_id: String,
        repository: String,
        revision: String,
        source_tree_sha256: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PurposeEnvelope {
    pub envelope_id: String,
    pub purpose_sha256: String,
    pub environment_sha256: String,
    pub hard_constraints_sha256: String,
    pub metric_policy_sha256: String,
    pub transformation_policy_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorObligation {
    pub generator_id: String,
    pub generator_version_sha256: String,
    pub scope_id: String,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignMetricDefinition {
    pub id: String,
    pub unit: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateVerification {
    Pending,
    Rejected { evidence_sha256: String },
    Verified { evidence_sha256: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignCandidate {
    pub id: String,
    pub search_basis_sha256: String,
    #[serde(default)]
    pub origin_sweep_id: Option<String>,
    pub verification: CandidateVerification,
    #[serde(default)]
    pub metrics: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratorOutcome {
    Complete {
        #[serde(default)]
        novel_candidate_ids: Vec<String>,
        #[serde(default)]
        examined_finite_members: Vec<String>,
    },
    Blocked {
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorReceipt {
    pub generator_id: String,
    pub generator_version_sha256: String,
    pub scope_id: String,
    pub scope_sha256: String,
    pub evidence_sha256: String,
    pub outcome: GeneratorOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignSweep {
    pub id: String,
    pub ordinal: u64,
    pub search_basis_sha256: String,
    pub decision_state_sha256: String,
    pub receipts: Vec<GeneratorReceipt>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteSearchUniverse {
    pub id: String,
    pub declared_members: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBudget {
    pub unit: String,
    pub limit: u64,
    pub consumed: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignState {
    pub schema_version: String,
    pub campaign_id: String,
    pub evaluator: EvaluatorIdentity,
    pub subject: CampaignSubject,
    pub envelope: PurposeEnvelope,
    pub semantic_model_sha256: String,
    pub generators: Vec<GeneratorObligation>,
    pub metric_definitions: Vec<CampaignMetricDefinition>,
    #[serde(default)]
    pub candidates: Vec<CampaignCandidate>,
    #[serde(default)]
    pub current_candidate_id: Option<String>,
    #[serde(default)]
    pub sweeps: Vec<CampaignSweep>,
    #[serde(default)]
    pub finite_universe: Option<FiniteSearchUniverse>,
    pub budget: ResourceBudget,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    #[serde(default)]
    pub parent_state_sha256: Option<String>,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Open,
    FrontierSet,
    EnvelopeStationary,
    BoundedExhaustiveFixedPoint,
    BudgetExhaustedWithUnknowns,
}

#[derive(Debug, Serialize)]
pub struct GeneratorCoverage {
    pub required: usize,
    pub latest_complete: usize,
    pub complete_current_sweeps: usize,
    pub unchanged_quiescent_sweeps: usize,
}

#[derive(Debug, Serialize)]
pub struct FiniteCoverage {
    pub universe_id: String,
    pub declared: usize,
    pub observed: usize,
    pub missing: Vec<String>,
    pub undeclared: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CampaignEvaluation {
    pub schema_version: String,
    pub campaign_id: String,
    pub mode: &'static str,
    pub subject_state_sha256: String,
    pub envelope_sha256: String,
    pub search_basis_sha256: String,
    pub decision_state_sha256: String,
    pub canonical_state_sha256: String,
    pub status: CampaignStatus,
    pub budget_exhausted: bool,
    pub current_candidate_id: Option<String>,
    pub frontier: Vec<String>,
    pub dominated: DominanceMap,
    pub generator_coverage: GeneratorCoverage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finite_coverage: Option<FiniteCoverage>,
    pub reasons: Vec<String>,
    pub nonclaims: Vec<&'static str>,
}

/// Compute current identities without accepting a caller-declared exhaustion
/// or fixed-point boolean.
///
/// # Errors
///
/// Returns [`ToolError`] when the state cannot be serialized.
pub fn campaign_identities(state: &CampaignState) -> Result<(String, String), ToolError> {
    Ok((search_basis_sha256(state)?, decision_state_sha256(state)?))
}

/// Evaluate a living optimization state. Generator coverage, budget
/// exhaustion, the represented Pareto frontier, and stopping are all derived.
///
/// # Errors
///
/// Returns [`ToolError`] for malformed identities, duplicate authority, stale
/// evaluator binding, incomplete metric vectors, or inconsistent receipts.
#[allow(clippy::too_many_lines)]
pub fn evaluate_campaign(state: &CampaignState) -> Result<CampaignEvaluation, ToolError> {
    validate_state(state)?;
    let evaluator = current_evaluator_identity()?;
    if state.evaluator != evaluator {
        return Err(ToolError::invalid(
            "campaign evaluator identity does not match the executing evaluator",
        ));
    }

    let subject_state_sha256 = canonical_sha256(&state.subject, "campaign subject")?;
    let envelope_sha256 = canonical_sha256(&state.envelope, "purpose envelope")?;
    let search_basis_sha256 = search_basis_sha256(state)?;
    let decision_state_sha256 = decision_state_sha256(state)?;
    let canonical_state_sha256 = canonical_state_sha256(state)?;
    let obligation_keys = state
        .generators
        .iter()
        .map(obligation_key)
        .collect::<BTreeSet<_>>();
    let candidates_by_id = state
        .candidates
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate))
        .collect::<BTreeMap<_, _>>();

    let mut current_sweeps = state
        .sweeps
        .iter()
        .filter(|sweep| sweep.search_basis_sha256 == search_basis_sha256)
        .collect::<Vec<_>>();
    current_sweeps.sort_by_key(|sweep| (sweep.ordinal, sweep.id.as_str()));
    let mut audited_sweeps = Vec::with_capacity(current_sweeps.len());
    for sweep in &current_sweeps {
        audited_sweeps.push(audit_sweep(sweep, &obligation_keys, &candidates_by_id)?);
    }

    let latest_complete = audited_sweeps
        .last()
        .filter(|audit| audit.complete)
        .map_or(0, |audit| audit.covered);
    let complete_current_sweeps = audited_sweeps.iter().filter(|audit| audit.complete).count();
    let unchanged_quiescent_sweeps = audited_sweeps
        .iter()
        .rev()
        .take_while(|audit| {
            audit.complete
                && audit.novel_candidate_ids.is_empty()
                && audit.decision_state_sha256 == decision_state_sha256
        })
        .count();

    let represented = state
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.search_basis_sha256 == search_basis_sha256
                && matches!(
                    candidate.verification,
                    CandidateVerification::Verified { .. }
                )
        })
        .collect::<Vec<_>>();
    let (frontier, dominated) = compute_frontier(state, &represented)?;
    let pending = state.candidates.iter().any(|candidate| {
        candidate.search_basis_sha256 == search_basis_sha256
            && matches!(candidate.verification, CandidateVerification::Pending)
    });
    let current_on_frontier = state
        .current_candidate_id
        .as_ref()
        .is_some_and(|current| frontier.contains(current));

    let finite_coverage = state.finite_universe.as_ref().map(|universe| {
        let declared = universe
            .declared_members
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let observed = audited_sweeps
            .last()
            .filter(|audit| audit.complete)
            .map_or_else(BTreeSet::new, |audit| audit.examined_finite_members.clone());
        FiniteCoverage {
            universe_id: universe.id.clone(),
            declared: declared.len(),
            observed: observed.len(),
            missing: declared.difference(&observed).cloned().collect(),
            undeclared: observed.difference(&declared).cloned().collect(),
        }
    });
    let finite_complete = finite_coverage
        .as_ref()
        .is_some_and(|coverage| coverage.missing.is_empty() && coverage.undeclared.is_empty());
    let budget_exhausted = state.budget.consumed >= state.budget.limit;

    let mut reasons = Vec::new();
    reasons.extend(state.blockers.iter().map(|item| format!("blocker: {item}")));
    reasons.extend(state.unknowns.iter().map(|item| format!("unknown: {item}")));
    if latest_complete != obligation_keys.len() {
        reasons.push(format!(
            "latest sweep covers {latest_complete}/{} current generator obligations",
            obligation_keys.len()
        ));
    }
    if pending {
        reasons.push("at least one current-basis candidate still awaits verification".to_owned());
    }
    if state.current_candidate_id.is_none() {
        reasons.push("no implementation candidate is current yet".to_owned());
    } else if !current_on_frontier {
        reasons.push(
            "the current implementation is not on the represented Pareto frontier".to_owned(),
        );
    }
    if !finite_complete && state.finite_universe.is_some() {
        reasons.push("the declared finite search universe is not exactly covered".to_owned());
    }

    let base_ready = state.blockers.is_empty()
        && state.unknowns.is_empty()
        && !pending
        && current_on_frontier
        && latest_complete == obligation_keys.len();
    let stable = base_ready && unchanged_quiescent_sweeps >= 2;
    let bounded_exhaustive = base_ready && finite_complete;
    let status = if bounded_exhaustive {
        CampaignStatus::BoundedExhaustiveFixedPoint
    } else if stable && frontier.len() > 1 {
        CampaignStatus::FrontierSet
    } else if stable {
        CampaignStatus::EnvelopeStationary
    } else if budget_exhausted {
        CampaignStatus::BudgetExhaustedWithUnknowns
    } else {
        if base_ready && unchanged_quiescent_sweeps < 2 {
            reasons.push(format!(
                "{unchanged_quiescent_sweeps}/2 unchanged complete quiescent sweeps recorded"
            ));
        }
        CampaignStatus::Open
    };

    Ok(CampaignEvaluation {
        schema_version: CAMPAIGN_EVALUATION_SCHEMA.to_owned(),
        campaign_id: state.campaign_id.clone(),
        mode: match state.subject {
            CampaignSubject::Greenfield { .. } => "greenfield",
            CampaignSubject::Brownfield { .. } => "brownfield",
        },
        subject_state_sha256,
        envelope_sha256,
        search_basis_sha256,
        decision_state_sha256,
        canonical_state_sha256,
        status,
        budget_exhausted,
        current_candidate_id: state.current_candidate_id.clone(),
        frontier,
        dominated,
        generator_coverage: GeneratorCoverage {
            required: obligation_keys.len(),
            latest_complete,
            complete_current_sweeps,
            unchanged_quiescent_sweeps,
        },
        finite_coverage,
        reasons,
        nonclaims: vec![
            "Stationarity is relative to the bound purpose envelope, semantic model, generator versions, scopes, metrics, evaluator, and represented evidence.",
            "Two unchanged complete sweeps establish only exhaustion of the declared generator obligations; they do not prove a universal optimum over unrepresented transformations.",
            "A bounded exhaustive fixed point is exhaustive only for the explicitly declared finite universe.",
            "Metric values and evidence digests remain assertions until the execution and independent verification stages validate their provenance.",
        ],
    })
}

struct SweepAudit {
    complete: bool,
    covered: usize,
    decision_state_sha256: String,
    novel_candidate_ids: BTreeSet<String>,
    examined_finite_members: BTreeSet<String>,
}

fn audit_sweep(
    sweep: &CampaignSweep,
    obligations: &BTreeSet<String>,
    candidates: &BTreeMap<&str, &CampaignCandidate>,
) -> Result<SweepAudit, ToolError> {
    let mut receipts = BTreeMap::new();
    let mut novel_candidate_ids = BTreeSet::new();
    let mut examined_finite_members = BTreeSet::new();
    let mut all_complete = true;
    for receipt in &sweep.receipts {
        let key = receipt_key(receipt);
        if receipts.insert(key.clone(), receipt).is_some() {
            return Err(ToolError::invalid(format!(
                "sweep {:?} has duplicate receipt for {key:?}",
                sweep.id
            )));
        }
        match &receipt.outcome {
            GeneratorOutcome::Complete {
                novel_candidate_ids: ids,
                examined_finite_members: members,
            } => {
                for id in ids {
                    if !novel_candidate_ids.insert(id.clone()) {
                        return Err(ToolError::invalid(format!(
                            "sweep {:?} reports candidate {id:?} more than once",
                            sweep.id
                        )));
                    }
                    let candidate = candidates.get(id.as_str()).ok_or_else(|| {
                        ToolError::invalid(format!(
                            "sweep {:?} reports unknown candidate {id:?}",
                            sweep.id
                        ))
                    })?;
                    if candidate.origin_sweep_id.as_deref() != Some(sweep.id.as_str())
                        || candidate.search_basis_sha256 != sweep.search_basis_sha256
                    {
                        return Err(ToolError::invalid(format!(
                            "candidate {id:?} is not bound to sweep {:?} and its search basis",
                            sweep.id
                        )));
                    }
                }
                examined_finite_members.extend(members.iter().cloned());
            }
            GeneratorOutcome::Blocked { .. } => all_complete = false,
        }
    }
    let receipt_keys = receipts.keys().cloned().collect::<BTreeSet<_>>();
    let complete = all_complete && receipt_keys == *obligations;
    Ok(SweepAudit {
        complete,
        covered: receipt_keys.intersection(obligations).count(),
        decision_state_sha256: sweep.decision_state_sha256.clone(),
        novel_candidate_ids,
        examined_finite_members,
    })
}

fn compute_frontier(
    state: &CampaignState,
    candidates: &[&CampaignCandidate],
) -> Result<(Vec<String>, DominanceMap), ToolError> {
    if candidates.is_empty() {
        return Ok((Vec::new(), BTreeMap::new()));
    }
    let input = CandidateInput {
        schema_version: CANDIDATE_SCHEMA.to_owned(),
        elements: vec!["candidate".to_owned()],
        modules: vec!["represented".to_owned()],
        constraints: Constraints::default(),
        metric_definitions: state
            .metric_definitions
            .iter()
            .map(|definition| KerfMetricDefinition {
                id: definition.id.clone(),
                unit: definition.unit.clone(),
                direction: definition.direction,
            })
            .collect(),
        candidates: candidates
            .iter()
            .map(|candidate| KerfCandidate {
                id: candidate.id.clone(),
                assignment: BTreeMap::from([("candidate".to_owned(), "represented".to_owned())]),
                authority: BTreeMap::new(),
                metrics: candidate.metrics.clone(),
            })
            .collect(),
    };
    let result = pareto(&input)?;
    Ok((
        result.frontier,
        result
            .dominated
            .into_iter()
            .map(|candidate| (candidate.id, candidate.dominated_by))
            .collect(),
    ))
}

#[allow(clippy::too_many_lines)]
fn validate_state(state: &CampaignState) -> Result<(), ToolError> {
    if state.schema_version != CAMPAIGN_STATE_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {CAMPAIGN_STATE_SCHEMA:?}",
            state.schema_version
        )));
    }
    require_id("campaign", &state.campaign_id)?;
    match &state.subject {
        CampaignSubject::Greenfield {
            subject_id,
            intent_model_sha256,
        } => {
            require_id("subject", subject_id)?;
            require_sha256("subject.intent_model_sha256", intent_model_sha256)?;
        }
        CampaignSubject::Brownfield {
            subject_id,
            repository,
            revision,
            source_tree_sha256,
        } => {
            require_id("subject", subject_id)?;
            if repository.is_empty() || revision.is_empty() {
                return Err(ToolError::invalid(
                    "brownfield repository and revision must not be empty",
                ));
            }
            require_sha256("subject.source_tree_sha256", source_tree_sha256)?;
        }
    }
    require_id("envelope", &state.envelope.envelope_id)?;
    for (field, value) in [
        ("envelope.purpose_sha256", &state.envelope.purpose_sha256),
        (
            "envelope.environment_sha256",
            &state.envelope.environment_sha256,
        ),
        (
            "envelope.hard_constraints_sha256",
            &state.envelope.hard_constraints_sha256,
        ),
        (
            "envelope.metric_policy_sha256",
            &state.envelope.metric_policy_sha256,
        ),
        (
            "envelope.transformation_policy_sha256",
            &state.envelope.transformation_policy_sha256,
        ),
        ("semantic_model_sha256", &state.semantic_model_sha256),
    ] {
        require_sha256(field, value)?;
    }
    if state.generators.is_empty() {
        return Err(ToolError::invalid(
            "generators must contain at least one versioned scope obligation",
        ));
    }
    let mut generators = BTreeSet::new();
    for generator in &state.generators {
        require_id("generator", &generator.generator_id)?;
        require_id("generator scope", &generator.scope_id)?;
        require_sha256(
            "generator.generator_version_sha256",
            &generator.generator_version_sha256,
        )?;
        require_sha256("generator.scope_sha256", &generator.scope_sha256)?;
        if !generators.insert(obligation_key(generator)) {
            return Err(ToolError::invalid(
                "generator obligations must have unique versioned scope identities",
            ));
        }
    }
    if state.metric_definitions.is_empty() {
        return Err(ToolError::invalid("metric_definitions must not be empty"));
    }
    let mut metrics = BTreeSet::new();
    for metric in &state.metric_definitions {
        require_id("metric", &metric.id)?;
        if metric.unit.is_empty() || !metrics.insert(metric.id.as_str()) {
            return Err(ToolError::invalid(
                "metric definitions need unique IDs and non-empty units",
            ));
        }
    }
    let mut candidates = BTreeSet::new();
    for candidate in &state.candidates {
        require_id("candidate", &candidate.id)?;
        require_sha256(
            "candidate.search_basis_sha256",
            &candidate.search_basis_sha256,
        )?;
        if !candidates.insert(candidate.id.as_str()) {
            return Err(ToolError::invalid("candidate IDs must be unique"));
        }
        match &candidate.verification {
            CandidateVerification::Pending => {}
            CandidateVerification::Rejected { evidence_sha256 }
            | CandidateVerification::Verified { evidence_sha256 } => {
                require_sha256("candidate evidence_sha256", evidence_sha256)?;
            }
        }
        if matches!(
            candidate.verification,
            CandidateVerification::Verified { .. }
        ) && candidate
            .metrics
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != metrics
        {
            return Err(ToolError::invalid(format!(
                "verified candidate {:?} must supply exactly every declared metric",
                candidate.id
            )));
        }
    }
    if let Some(current) = &state.current_candidate_id {
        let Some(candidate) = state
            .candidates
            .iter()
            .find(|candidate| &candidate.id == current)
        else {
            return Err(ToolError::invalid(
                "current_candidate_id must reference a candidate",
            ));
        };
        if !matches!(
            candidate.verification,
            CandidateVerification::Verified { .. }
        ) {
            return Err(ToolError::invalid(
                "current candidate must have a verification receipt",
            ));
        }
    }
    let mut sweep_ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for sweep in &state.sweeps {
        require_id("sweep", &sweep.id)?;
        require_sha256("sweep.search_basis_sha256", &sweep.search_basis_sha256)?;
        require_sha256("sweep.decision_state_sha256", &sweep.decision_state_sha256)?;
        if !sweep_ids.insert(sweep.id.as_str()) || !ordinals.insert(sweep.ordinal) {
            return Err(ToolError::invalid("sweep IDs and ordinals must be unique"));
        }
        for receipt in &sweep.receipts {
            require_id("receipt generator", &receipt.generator_id)?;
            require_id("receipt scope", &receipt.scope_id)?;
            require_sha256(
                "receipt.generator_version_sha256",
                &receipt.generator_version_sha256,
            )?;
            require_sha256("receipt.scope_sha256", &receipt.scope_sha256)?;
            require_sha256("receipt.evidence_sha256", &receipt.evidence_sha256)?;
            if let GeneratorOutcome::Blocked { reason } = &receipt.outcome
                && reason.is_empty()
            {
                return Err(ToolError::invalid(
                    "blocked generator receipts require a reason",
                ));
            }
        }
    }
    if let Some(universe) = &state.finite_universe {
        require_id("finite universe", &universe.id)?;
        let members = universe
            .declared_members
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if members.is_empty()
            || members.len() != universe.declared_members.len()
            || members.contains("")
        {
            return Err(ToolError::invalid(
                "finite universe members must be non-empty and unique",
            ));
        }
    }
    if state.budget.unit.is_empty() || state.budget.limit == 0 {
        return Err(ToolError::invalid(
            "resource budget requires a non-empty unit and a positive limit",
        ));
    }
    if state.blockers.iter().any(String::is_empty)
        || state.unknowns.iter().any(String::is_empty)
        || state.nonclaims.is_empty()
        || state.nonclaims.iter().any(String::is_empty)
    {
        return Err(ToolError::invalid(
            "blockers, unknowns, and nonclaims must not contain empty entries; at least one nonclaim is required",
        ));
    }
    if let Some(parent) = &state.parent_state_sha256 {
        require_sha256("parent_state_sha256", parent)?;
    }
    Ok(())
}

fn search_basis_sha256(state: &CampaignState) -> Result<String, ToolError> {
    let generators = state
        .generators
        .iter()
        .map(|generator| (obligation_key(generator), generator))
        .collect::<BTreeMap<_, _>>();
    let metrics = state
        .metric_definitions
        .iter()
        .map(|metric| (metric.id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    canonical_sha256(
        &json!({
            "algorithm": "kerf.search-basis.v1",
            "evaluator": state.evaluator,
            "subject": state.subject,
            "envelope": state.envelope,
            "semantic_model_sha256": state.semantic_model_sha256,
            "generators": generators,
            "metric_definitions": metrics,
            "current_candidate_id": state.current_candidate_id,
            "finite_universe": normalized_finite_universe(state.finite_universe.as_ref()),
        }),
        "search basis",
    )
}

fn decision_state_sha256(state: &CampaignState) -> Result<String, ToolError> {
    let candidates = state
        .candidates
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate))
        .collect::<BTreeMap<_, _>>();
    let mut blockers = state.blockers.clone();
    blockers.sort();
    let mut unknowns = state.unknowns.clone();
    unknowns.sort();
    canonical_sha256(
        &json!({
            "algorithm": "kerf.decision-state.v1",
            "search_basis_sha256": search_basis_sha256(state)?,
            "candidates": candidates,
            "blockers": blockers,
            "unknowns": unknowns,
        }),
        "decision state",
    )
}

fn canonical_state_sha256(state: &CampaignState) -> Result<String, ToolError> {
    let sweeps = state
        .sweeps
        .iter()
        .map(|sweep| {
            let receipts = sweep
                .receipts
                .iter()
                .map(|receipt| (receipt_key(receipt), normalized_receipt(receipt)))
                .collect::<BTreeMap<_, _>>();
            (
                format!("{:020}\0{}", sweep.ordinal, sweep.id),
                json!({
                    "id": sweep.id,
                    "ordinal": sweep.ordinal,
                    "search_basis_sha256": sweep.search_basis_sha256,
                    "decision_state_sha256": sweep.decision_state_sha256,
                    "receipts": receipts,
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut nonclaims = state.nonclaims.clone();
    nonclaims.sort();
    canonical_sha256(
        &json!({
            "algorithm": "kerf.campaign-state.v1",
            "decision_state_sha256": decision_state_sha256(state)?,
            "sweeps": sweeps,
            "budget": state.budget,
            "parent_state_sha256": state.parent_state_sha256,
            "nonclaims": nonclaims,
        }),
        "canonical campaign state",
    )
}

fn normalized_finite_universe(universe: Option<&FiniteSearchUniverse>) -> Value {
    universe.map_or(Value::Null, |universe| {
        let mut members = universe.declared_members.clone();
        members.sort();
        json!({"id": universe.id, "declared_members": members})
    })
}

fn normalized_receipt(receipt: &GeneratorReceipt) -> Value {
    let outcome = match &receipt.outcome {
        GeneratorOutcome::Complete {
            novel_candidate_ids,
            examined_finite_members,
        } => {
            let mut candidate_ids = novel_candidate_ids.clone();
            candidate_ids.sort();
            let mut members = examined_finite_members.clone();
            members.sort();
            json!({
                "status": "complete",
                "novel_candidate_ids": candidate_ids,
                "examined_finite_members": members,
            })
        }
        GeneratorOutcome::Blocked { reason } => {
            json!({"status": "blocked", "reason": reason})
        }
    };
    json!({
        "generator_id": receipt.generator_id,
        "generator_version_sha256": receipt.generator_version_sha256,
        "scope_id": receipt.scope_id,
        "scope_sha256": receipt.scope_sha256,
        "evidence_sha256": receipt.evidence_sha256,
        "outcome": outcome,
    })
}

fn obligation_key(generator: &GeneratorObligation) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        generator.generator_id,
        generator.generator_version_sha256,
        generator.scope_id,
        generator.scope_sha256
    )
}

fn receipt_key(receipt: &GeneratorReceipt) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        receipt.generator_id,
        receipt.generator_version_sha256,
        receipt.scope_id,
        receipt.scope_sha256
    )
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
