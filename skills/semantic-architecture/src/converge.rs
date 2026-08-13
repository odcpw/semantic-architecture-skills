//! Pure, plan-bound, information-bearing stopping predicates.

use crate::TOOL_VERSION;
use crate::manifest::{
    ExecutedEvidenceManifest, ExecutionPlan, ExpectedObservation, ManifestStatus, audit_manifest,
};
use crate::semantic::{ClaimVerdict, ClosureKind};
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub const DECISION_CONTRACT_SCHEMA: &str = "kerf.decision-contract.v1";
pub const CONVERGENCE_EVALUATION_SCHEMA: &str = "kerf.convergence-evaluation.v1";
pub const EVALUATOR_IDENTITY_SCHEMA: &str = "kerf.evaluator-identity.v1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorIdentity {
    pub schema_version: String,
    pub tool_version: String,
    pub executable_sha256: String,
    pub skill_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePackage {
    pub plan: ExecutionPlan,
    pub manifest: ExecutedEvidenceManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContract {
    pub schema_version: String,
    pub decision_id: String,
    pub retirement_condition: String,
    pub retirement_condition_met: bool,
    pub evaluator: EvaluatorIdentity,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub mandatory_claims: Vec<MandatoryClaimRef>,
    #[serde(default)]
    pub eligible_tests: Vec<EligibleTest>,
    pub generators_exhausted: bool,
    #[serde(default)]
    pub finite_universe: Option<FiniteUniverse>,
    pub budget_exhausted: bool,
    #[serde(default)]
    pub prior_state_sha256: Option<String>,
    #[serde(default)]
    pub provider_versions: BTreeMap<String, String>,
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MandatoryClaimRef {
    pub plan_sha256: String,
    pub claim_id: String,
    pub required_closure: ClosureKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EligibleTest {
    pub id: String,
    pub affected_claims: Vec<MandatoryClaimRef>,
    pub legal_outcomes: Vec<ClaimVerdict>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteUniverse {
    pub id: String,
    pub closure: ClosureKind,
    pub declared_members: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceStatus {
    Open,
    DecisionFixedPoint,
    BoundedExhaustiveFixedPoint,
    BudgetExhaustedWithUnknowns,
    Refuted,
    BindingRejected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConvergenceEvaluation {
    pub schema_version: String,
    pub decision_id: String,
    pub contract_sha256: String,
    pub evaluator: EvaluatorIdentity,
    pub package_sha256s: Vec<String>,
    pub decision_projection_sha256s: Vec<String>,
    pub canonical_state_sha256: String,
    pub status: ConvergenceStatus,
    pub eligible_remaining: usize,
    pub mandatory_claims_supported: usize,
    pub mandatory_claims_required: usize,
    pub observed_finite_members: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    pub nonclaims: Vec<String>,
}

/// Evaluate whether a decision reached a stable, plan-bound fixed point.
///
/// Every package is re-audited against its exact execution plan. Only an
/// `Accepted` manifest contributes positive evidence. A second evaluation with
/// an unchanged canonical state is required because the pure evaluator cannot
/// itself observe that a complete discovery scheduler sweep occurred.
///
/// # Errors
///
/// Returns [`ToolError`] for malformed contracts, zero-information tests,
/// inconsistent claim identities, or malformed packages.
#[allow(clippy::too_many_lines)]
pub fn evaluate(
    contract: &DecisionContract,
    packages: &[EvidencePackage],
) -> Result<ConvergenceEvaluation, ToolError> {
    validate_contract(contract)?;
    let evaluator = current_evaluator_identity()?;
    if contract.evaluator != evaluator {
        return Err(ToolError::invalid(format!(
            "decision contract evaluator identity does not match the executing evaluator; expected {}, observed {}",
            contract.evaluator.executable_sha256, evaluator.executable_sha256
        )));
    }
    if packages.is_empty() {
        return Err(ToolError::invalid(
            "convergence evaluation requires at least one evidence package",
        ));
    }
    let contract_sha256 = canonical_sha256(contract, "decision contract")?;
    let mut package_sha256s = packages
        .iter()
        .map(|package| canonical_sha256(package, "evidence package"))
        .collect::<Result<Vec<_>, _>>()?;
    package_sha256s.sort();

    let mut audited_statuses = Vec::new();
    let mut decision_projection_sha256s = Vec::new();
    let mut claims = BTreeMap::new();
    let mut observed_members = BTreeSet::new();
    for package in packages {
        if package.plan.decision_id != contract.decision_id
            || package.manifest.decision_id != contract.decision_id
        {
            return Err(ToolError::invalid(format!(
                "package decision does not match contract {:?}",
                contract.decision_id
            )));
        }
        let plan_sha256 = canonical_sha256(&package.plan, "execution plan")?;
        let status = audit_manifest(&package.plan, &package.manifest)?;
        audited_statuses.push(status);
        decision_projection_sha256s.push(canonical_sha256(
            &decision_projection(&plan_sha256, status, &package.manifest),
            "evidence decision projection",
        )?);
        if status == ManifestStatus::Accepted {
            observed_members.extend(decision_reachable_case_ids(
                contract,
                &plan_sha256,
                &package.plan,
            )?);
        }
        for claim in &package.manifest.claim_receipts {
            let key = (plan_sha256.clone(), claim.id.clone());
            if let Some(previous) = claims.insert(key, claim)
                && (previous.subject != claim.subject
                    || previous.kind != claim.kind
                    || previous.epistemic_class != claim.epistemic_class
                    || previous.closure != claim.closure
                    || previous.validator_ids != claim.validator_ids
                    || previous.negative_control_id != claim.negative_control_id
                    || previous.verdict != claim.verdict
                    || previous.negative_control_passed != claim.negative_control_passed)
            {
                return Err(ToolError::invalid(format!(
                    "claim {:?} has inconsistent identities or receipts",
                    claim.id
                )));
            }
        }
    }
    decision_projection_sha256s.sort();

    let mut supported = 0;
    let mut missing_or_open = Vec::new();
    for required in &contract.mandatory_claims {
        let key = (required.plan_sha256.clone(), required.claim_id.clone());
        match claims.get(&key) {
            Some(claim)
                if claim.verdict == ClaimVerdict::Supported
                    && claim.negative_control_passed
                    && claim.closure == required.required_closure =>
            {
                supported += 1;
            }
            Some(claim) => missing_or_open.push(format!(
                "claim {:?} is {:?}, control_passed={}, closure={:?}",
                required.claim_id, claim.verdict, claim.negative_control_passed, claim.closure
            )),
            None => missing_or_open.push(format!(
                "claim {:?} from plan {} has no receipt",
                required.claim_id, required.plan_sha256
            )),
        }
    }

    let expected_members = contract.finite_universe.as_ref().map(|universe| {
        universe
            .declared_members
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    });
    let finite_complete = expected_members
        .as_ref()
        .is_none_or(|expected| expected == &observed_members);
    if let Some(expected) = &expected_members
        && expected != &observed_members
    {
        let missing = expected
            .difference(&observed_members)
            .cloned()
            .collect::<Vec<_>>();
        let undeclared = observed_members
            .difference(expected)
            .cloned()
            .collect::<Vec<_>>();
        missing_or_open.push(format!(
            "finite universe mismatch; missing={missing:?}; undeclared={undeclared:?}"
        ));
    }

    let mut normalized_contract = contract.clone();
    normalized_contract.prior_state_sha256 = None;
    let canonical_state = json!({
        "algorithm": "kerf.convergence-state.v3",
        "evaluator": evaluator,
        "contract": normalized_contract,
        "decision_projection_sha256s": decision_projection_sha256s,
        "claims": claims.iter().map(|((plan, id), claim)| json!({
            "plan_sha256": plan,
            "id": id,
            "subject": claim.subject,
            "kind": claim.kind,
            "epistemic_class": claim.epistemic_class,
            "closure": claim.closure,
            "validator_ids": claim.validator_ids,
            "negative_control_id": claim.negative_control_id,
            "verdict": claim.verdict,
            "negative_control_passed": claim.negative_control_passed,
        })).collect::<Vec<_>>(),
        "observed_finite_members": observed_members,
    });
    let canonical_state_sha256 = canonical_sha256(&canonical_state, "convergence state")?;

    let binding_rejected = audited_statuses.iter().any(|status| {
        matches!(
            status,
            ManifestStatus::BindingRejected | ManifestStatus::NegativeControlFailed
        )
    });
    let refuted = audited_statuses.contains(&ManifestStatus::Refuted);
    let unknown = audited_statuses.contains(&ManifestStatus::Unknown);
    let eligible_remaining = contract.eligible_tests.len();
    let stable = contract.prior_state_sha256.as_deref() == Some(&canonical_state_sha256);
    let ready = contract.retirement_condition_met
        && contract.blockers.is_empty()
        && missing_or_open.is_empty()
        && contract.generators_exhausted
        && finite_complete
        && !unknown
        && eligible_remaining == 0;

    let (status, witness) = if binding_rejected {
        (
            ConvergenceStatus::BindingRejected,
            Some("an evidence package failed binding or its negative control".to_owned()),
        )
    } else if refuted {
        (
            ConvergenceStatus::Refuted,
            Some("a decision claim was refuted".to_owned()),
        )
    } else if contract.budget_exhausted {
        (
            ConvergenceStatus::BudgetExhaustedWithUnknowns,
            Some("the declared budget expired before the fixed-point predicate held".to_owned()),
        )
    } else if !ready {
        let mut reasons = Vec::new();
        if !contract.retirement_condition_met {
            reasons.push("retirement condition is not met".to_owned());
        }
        reasons.extend(
            contract
                .blockers
                .iter()
                .map(|item| format!("blocker: {item}")),
        );
        reasons.extend(missing_or_open);
        if !contract.generators_exhausted {
            reasons.push("declared generators are not exhausted".to_owned());
        }
        if unknown {
            reasons.push("at least one required package is unknown".to_owned());
        }
        if eligible_remaining > 0 {
            reasons.push(format!(
                "{eligible_remaining} information-bearing tests remain"
            ));
        }
        (ConvergenceStatus::Open, Some(reasons.join("; ")))
    } else if !stable {
        (
            ConvergenceStatus::Open,
            Some("one unchanged canonical-state sweep is required".to_owned()),
        )
    } else if contract.finite_universe.as_ref().is_some_and(|universe| {
        universe.closure == ClosureKind::ExhaustiveFinite && finite_complete
    }) {
        (ConvergenceStatus::BoundedExhaustiveFixedPoint, None)
    } else {
        (ConvergenceStatus::DecisionFixedPoint, None)
    };

    Ok(ConvergenceEvaluation {
        schema_version: CONVERGENCE_EVALUATION_SCHEMA.to_owned(),
        decision_id: contract.decision_id.clone(),
        contract_sha256,
        evaluator,
        package_sha256s,
        decision_projection_sha256s,
        canonical_state_sha256,
        status,
        eligible_remaining,
        mandatory_claims_supported: supported,
        mandatory_claims_required: contract.mandatory_claims.len(),
        observed_finite_members: observed_members.into_iter().collect(),
        witness,
        nonclaims: vec![
            "A decision fixed point is a pure predicate over trusted plan-bound receipts and declared scheduler state; it does not independently observe that discovery was complete.".to_owned(),
            "A bounded exhaustive fixed point is exhaustive only for the explicitly enumerated finite member set in the contract.".to_owned(),
            "Persisted unsigned JSON is internally auditable but not cryptographic proof that execution physically occurred; hostile-fabrication resistance requires in-process execution or trusted attestation.".to_owned(),
            "Provider agreement, elapsed time, pass count, and budget consumption are not success criteria.".to_owned(),
        ],
    })
}

/// Return the content identity of the executable containing the evaluator.
///
/// # Errors
///
/// Returns [`ToolError`] when the current executable cannot be resolved or
/// read. The digest binds a convergence state to the actual evaluator bytes,
/// not merely to a caller-supplied semantic version.
pub fn current_evaluator_identity() -> Result<EvaluatorIdentity, ToolError> {
    let path = std::env::current_exe()
        .map_err(|error| ToolError::io(format!("resolve current evaluator executable: {error}")))?;
    let bytes = std::fs::read(&path).map_err(|error| {
        ToolError::io(format!(
            "read current evaluator executable {}: {error}",
            path.display()
        ))
    })?;
    Ok(EvaluatorIdentity {
        schema_version: EVALUATOR_IDENTITY_SCHEMA.to_owned(),
        tool_version: TOOL_VERSION.to_owned(),
        executable_sha256: sha256_hex(&bytes),
        skill_sha256: sha256_hex(include_bytes!("../SKILL.md")),
    })
}

fn decision_reachable_case_ids(
    contract: &DecisionContract,
    plan_sha256: &str,
    plan: &ExecutionPlan,
) -> Result<BTreeSet<String>, ToolError> {
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
    let mut case_ids = BTreeSet::new();
    for required in contract
        .mandatory_claims
        .iter()
        .filter(|required| required.plan_sha256 == plan_sha256)
    {
        let Some(claim) = plan
            .claims
            .iter()
            .find(|claim| claim.id == required.claim_id)
        else {
            continue;
        };
        let control = controls
            .get(claim.negative_control_id.as_str())
            .ok_or_else(|| {
                ToolError::internal(format!(
                    "validated claim {:?} has no negative control",
                    claim.id
                ))
            })?;
        for validator_id in claim
            .validator_ids
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(control.validator_id.as_str()))
        {
            let validator = validators.get(validator_id).ok_or_else(|| {
                ToolError::internal(format!(
                    "validated claim {:?} has no validator {validator_id:?}",
                    claim.id
                ))
            })?;
            let observed = invocations
                .get(validator.observed_invocation_id.as_str())
                .ok_or_else(|| ToolError::internal("validated observed invocation is absent"))?;
            case_ids.insert(observed.case_id.clone());
            if let ExpectedObservation::Invocation { invocation_id } = &validator.expected {
                let expected = invocations.get(invocation_id.as_str()).ok_or_else(|| {
                    ToolError::internal("validated expected invocation is absent")
                })?;
                case_ids.insert(expected.case_id.clone());
            }
        }
    }
    Ok(case_ids)
}

fn decision_projection(
    plan_sha256: &str,
    status: ManifestStatus,
    manifest: &ExecutedEvidenceManifest,
) -> serde_json::Value {
    json!({
        "algorithm": "kerf.evidence-decision-projection.v1",
        "plan_sha256": plan_sha256,
        "status": status,
        "validators": manifest.validator_receipts,
        "claims": manifest.claim_receipts,
        "negative_controls": manifest.negative_control_receipts,
        "counterexamples": manifest.counterexamples,
        "witness": manifest.witness,
        "nonclaims": manifest.nonclaims,
    })
}

fn validate_contract(contract: &DecisionContract) -> Result<(), ToolError> {
    if contract.schema_version != DECISION_CONTRACT_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {DECISION_CONTRACT_SCHEMA:?}",
            contract.schema_version
        )));
    }
    if contract.decision_id.is_empty() || contract.retirement_condition.is_empty() {
        return Err(ToolError::invalid(
            "decision_id and retirement_condition must not be empty",
        ));
    }
    if contract.evaluator.schema_version != EVALUATOR_IDENTITY_SCHEMA
        || contract.evaluator.tool_version.is_empty()
    {
        return Err(ToolError::invalid(format!(
            "contract evaluator must use schema {EVALUATOR_IDENTITY_SCHEMA:?} and a non-empty tool version"
        )));
    }
    require_sha256(
        "contract.evaluator.executable_sha256",
        &contract.evaluator.executable_sha256,
    )?;
    require_sha256(
        "contract.evaluator.skill_sha256",
        &contract.evaluator.skill_sha256,
    )?;
    if contract.nonclaims.is_empty() || contract.nonclaims.iter().any(String::is_empty) {
        return Err(ToolError::invalid(
            "decision contract must contain at least one non-empty nonclaim",
        ));
    }
    if contract.mandatory_claims.is_empty() {
        return Err(ToolError::invalid(
            "mandatory_claims must contain at least one plan-bound claim",
        ));
    }
    let mut claims = BTreeSet::new();
    for claim in &contract.mandatory_claims {
        require_sha256("mandatory_claim.plan_sha256", &claim.plan_sha256)?;
        if claim.claim_id.is_empty() || !claims.insert(claim_key(claim)) {
            return Err(ToolError::invalid(
                "mandatory claims must be non-empty and unique",
            ));
        }
    }
    let mut tests = BTreeSet::new();
    for test in &contract.eligible_tests {
        if test.id.is_empty() || !tests.insert(test.id.as_str()) {
            return Err(ToolError::invalid(
                "eligible test IDs must be non-empty and unique",
            ));
        }
        if test.affected_claims.is_empty()
            || test
                .affected_claims
                .iter()
                .any(|claim| !claims.contains(&claim_key(claim)))
        {
            return Err(ToolError::invalid(format!(
                "eligible test {:?} must affect a mandatory plan-bound claim",
                test.id
            )));
        }
        let outcomes: BTreeSet<_> = test
            .legal_outcomes
            .iter()
            .map(|outcome| format!("{outcome:?}"))
            .collect();
        if outcomes.len() < 2 {
            return Err(ToolError::invalid(format!(
                "eligible test {:?} has no information gain: fewer than two legal verdicts",
                test.id
            )));
        }
    }
    if let Some(universe) = &contract.finite_universe {
        let members: BTreeSet<_> = universe.declared_members.iter().collect();
        if universe.id.is_empty()
            || universe.declared_members.is_empty()
            || members.len() != universe.declared_members.len()
        {
            return Err(ToolError::invalid(
                "finite_universe needs a non-empty ID and unique non-empty declared members",
            ));
        }
        if universe.declared_members.iter().any(String::is_empty) {
            return Err(ToolError::invalid(
                "finite_universe members must not be empty",
            ));
        }
    }
    if let Some(prior) = &contract.prior_state_sha256 {
        require_sha256("prior_state_sha256", prior)?;
    }
    Ok(())
}

fn claim_key(claim: &MandatoryClaimRef) -> String {
    format!(
        "{}\0{}\0{:?}",
        claim.plan_sha256, claim.claim_id, claim.required_closure
    )
}

fn canonical_sha256<T: Serialize>(value: &T, label: &str) -> Result<String, ToolError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| ToolError::internal(format!("serialize {label}: {error}")))
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
