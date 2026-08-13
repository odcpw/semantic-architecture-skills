use crate::cert::{self, CertificateClaim, CertificateInput};
use crate::lts::{self, ContextMonitor, Machine};
use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const CANDIDATE_SCHEMA: &str = "kerf.candidates.v1";
pub const MIGRATION_GRAPH_SCHEMA: &str = "kerf.migration-graph.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateInput {
    pub schema_version: String,
    pub elements: Vec<String>,
    pub modules: Vec<String>,
    #[serde(default)]
    pub constraints: Constraints,
    pub metric_definitions: Vec<MetricDefinition>,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Constraints {
    pub co_locate: Vec<[String; 2]>,
    pub separate: Vec<[String; 2]>,
    pub pinned: BTreeMap<String, String>,
    pub authority_groups: Vec<AuthorityGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityGroup {
    pub id: String,
    pub members: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricDefinition {
    pub id: String,
    pub unit: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub assignment: BTreeMap<String, String>,
    #[serde(default)]
    pub authority: BTreeMap<String, String>,
    pub metrics: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationGraphInput {
    pub schema_version: String,
    pub carrier_contracts: Vec<MigrationCarrierContract>,
    pub nodes: Vec<MigrationNode>,
    pub edges: Vec<MigrationEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationCarrierContract {
    pub id: String,
    #[serde(default)]
    pub required_context: Option<ContextMonitor>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationNode {
    pub id: String,
    pub carriers: Vec<MigrationNodeCarrier>,
    #[serde(default)]
    pub cleanup: Option<CleanupState>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationNodeCarrier {
    pub id: String,
    pub machine: Machine,
    pub writer: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CleanupState {
    pub obligation: String,
    pub rank: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub operation: String,
    pub affected_carrier: String,
    pub evidence: CertificateInput,
    #[serde(default)]
    pub authority_transfer: Option<AuthorityTransfer>,
    pub cleanup_transition: CleanupTransition,
    #[serde(default)]
    pub reverse_evidence: Option<ReverseEvidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityTransfer {
    pub from_writer: String,
    pub to_writer: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CleanupTransition {
    Neutral,
    Introduce {
        obligation: String,
        to_rank: u64,
    },
    Progress {
        obligation: String,
        from_rank: u64,
        to_rank: u64,
    },
    Discharge {
        obligation: String,
        from_rank: u64,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReverseEvidence {
    pub operation: String,
    pub certificate: CertificateInput,
    #[serde(default)]
    pub authority_transfer: Option<AuthorityTransfer>,
}

#[derive(Debug, Serialize)]
pub struct CandidateValidationResult {
    pub algorithm: &'static str,
    pub candidates: Vec<CandidateStatus>,
    pub feasible_count: usize,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ParetoResult {
    pub algorithm: &'static str,
    pub supplied_set_enumeration: SuppliedSetCertificate,
    pub frontier: Vec<String>,
    pub dominated: Vec<DominatedCandidate>,
    pub infeasible: Vec<String>,
    pub metric_definitions: Vec<MetricDefinitionOutput>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct SuppliedSetCertificate {
    pub supplied_candidate_count: usize,
    pub feasible_candidate_count: usize,
    pub statement: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MetricDefinitionOutput {
    pub id: String,
    pub unit: String,
    pub direction: Direction,
}

#[derive(Debug, Serialize)]
pub struct CandidateStatus {
    pub id: String,
    pub feasible: bool,
    pub violations: Vec<ConstraintViolation>,
}

#[derive(Debug, Serialize)]
pub struct ConstraintViolation {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DominatedCandidate {
    pub id: String,
    pub dominated_by: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MigrationValidationResult {
    pub algorithm: &'static str,
    pub valid: bool,
    pub nodes_checked: usize,
    pub edges_checked: usize,
    pub first_violation: Option<MigrationViolation>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct MigrationViolation {
    pub kind: &'static str,
    pub id: String,
    pub carrier: Option<String>,
    pub code: &'static str,
    pub details: BTreeMap<String, Value>,
}

struct Validated<'a> {
    metric_definitions: Vec<(&'a MetricDefinition, Direction)>,
    candidates: Vec<ValidatedCandidate<'a>>,
}

struct ValidatedCandidate<'a> {
    candidate: &'a Candidate,
    metrics: Vec<u128>,
    violations: Vec<ConstraintViolation>,
}

struct PreparedMigration<'a> {
    contracts: BTreeMap<&'a str, &'a MigrationCarrierContract>,
    nodes: BTreeMap<&'a str, PreparedMigrationNode<'a>>,
    edges: Vec<PreparedMigrationEdge<'a>>,
}

struct PreparedMigrationEdge<'a> {
    edge: &'a MigrationEdge,
    forward_verification: cert::CertificateResult,
    reverse_verification: Option<cert::CertificateResult>,
}

struct PreparedMigrationNode<'a> {
    node: &'a MigrationNode,
    carriers: BTreeMap<&'a str, PreparedMigrationCarrier<'a>>,
}

struct PreparedMigrationCarrier<'a> {
    carrier: &'a MigrationNodeCarrier,
    machine_encoding: Vec<u8>,
    actions: Vec<String>,
}

/// Validate the candidate schema and its executable hard constraints.
///
/// # Errors
///
/// Returns [`ToolError`] when IDs, references, assignments, authority selections,
/// metrics, or schema versions are invalid.
pub fn validate(input: &CandidateInput) -> Result<CandidateValidationResult, ToolError> {
    let validated = validate_all(input)?;
    let feasible_count = validated
        .candidates
        .iter()
        .filter(|candidate| candidate.violations.is_empty())
        .count();
    Ok(CandidateValidationResult {
        algorithm: "explicit_constraint_validation_v1",
        candidates: validated
            .candidates
            .into_iter()
            .map(|candidate| CandidateStatus {
                id: candidate.candidate.id.clone(),
                feasible: candidate.violations.is_empty(),
                violations: candidate.violations,
            })
            .collect(),
        feasible_count,
        nonclaims: candidate_nonclaims(),
    })
}

/// Compute the exact Pareto frontier of the feasible supplied candidate set.
///
/// # Errors
///
/// Returns [`ToolError`] when the candidate model is invalid.
pub fn pareto(input: &CandidateInput) -> Result<ParetoResult, ToolError> {
    let validated = validate_all(input)?;
    let feasible: Vec<&ValidatedCandidate<'_>> = validated
        .candidates
        .iter()
        .filter(|candidate| candidate.violations.is_empty())
        .collect();
    let mut frontier = Vec::new();
    let mut dominated = Vec::new();
    for candidate in &feasible {
        let mut dominators: Vec<String> = feasible
            .iter()
            .filter(|other| {
                other.candidate.id != candidate.candidate.id
                    && dominates(other, candidate, &validated.metric_definitions)
            })
            .map(|other| other.candidate.id.clone())
            .collect();
        dominators.sort();
        if dominators.is_empty() {
            frontier.push(candidate.candidate.id.clone());
        } else {
            dominated.push(DominatedCandidate {
                id: candidate.candidate.id.clone(),
                dominated_by: dominators,
            });
        }
    }
    frontier.sort();
    dominated.sort_by(|left, right| left.id.cmp(&right.id));
    let mut infeasible: Vec<String> = validated
        .candidates
        .iter()
        .filter(|candidate| !candidate.violations.is_empty())
        .map(|candidate| candidate.candidate.id.clone())
        .collect();
    infeasible.sort();
    Ok(ParetoResult {
        algorithm: "exact_product_order_v1",
        supplied_set_enumeration: SuppliedSetCertificate {
            supplied_candidate_count: validated.candidates.len(),
            feasible_candidate_count: feasible.len(),
            statement: "All supplied candidates were validated and all represented-feasible candidates were pairwise compared; no candidate-space exhaustiveness is claimed.",
        },
        frontier,
        dominated,
        infeasible,
        metric_definitions: validated
            .metric_definitions
            .iter()
            .map(|(definition, direction)| MetricDefinitionOutput {
                id: definition.id.clone(),
                unit: definition.unit.clone(),
                direction: *direction,
            })
            .collect(),
        nonclaims: candidate_nonclaims(),
    })
}

/// Validate a finite graph of proof-carrying modeled migrations.
///
/// # Errors
///
/// Returns [`ToolError`] when the graph shape, IDs, machines, contexts,
/// endpoint references, or nested certificate inputs are invalid. A
/// well-formed graph with a false edge claim or open cleanup sink is a
/// completed result with `valid: false`.
pub fn validate_migration(
    input: &MigrationGraphInput,
) -> Result<MigrationValidationResult, ToolError> {
    let prepared = prepare_migration(input)?;
    for (edge_index, edge) in prepared.edges.iter().enumerate() {
        if let Some(violation) = validate_migration_edge(&prepared, edge)? {
            return Ok(migration_result(
                input.nodes.len(),
                edge_index + 1,
                Some(violation),
            ));
        }
    }

    let outgoing: BTreeSet<&str> = prepared
        .edges
        .iter()
        .map(|edge| edge.edge.source.as_str())
        .collect();
    if let Some(node) = prepared
        .nodes
        .values()
        .find(|node| node.node.cleanup.is_some() && !outgoing.contains(node.node.id.as_str()))
    {
        return Ok(migration_result(
            input.nodes.len(),
            input.edges.len(),
            Some(node_violation(
                node.node,
                "open_cleanup_sink",
                [("cleanup", json!(node.node.cleanup))],
            )),
        ));
    }

    Ok(migration_result(input.nodes.len(), input.edges.len(), None))
}

fn migration_result(
    nodes_checked: usize,
    edges_checked: usize,
    first_violation: Option<MigrationViolation>,
) -> MigrationValidationResult {
    MigrationValidationResult {
        algorithm: "finite_proof_carrying_migration_graph_v1",
        valid: first_violation.is_none(),
        nodes_checked,
        edges_checked,
        first_violation,
        nonclaims: migration_nonclaims(),
    }
}

fn prepare_migration(input: &MigrationGraphInput) -> Result<PreparedMigration<'_>, ToolError> {
    if input.schema_version != MIGRATION_GRAPH_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {:?}",
            input.schema_version, MIGRATION_GRAPH_SCHEMA
        )));
    }
    require_unique_nonempty_ids(
        "carrier contract",
        input
            .carrier_contracts
            .iter()
            .map(|contract| contract.id.as_str()),
        true,
    )?;
    require_unique_nonempty_ids(
        "node",
        input.nodes.iter().map(|node| node.id.as_str()),
        true,
    )?;
    require_unique_nonempty_ids(
        "edge",
        input.edges.iter().map(|edge| edge.id.as_str()),
        false,
    )?;

    let contracts: BTreeMap<&str, &MigrationCarrierContract> = input
        .carrier_contracts
        .iter()
        .map(|contract| (contract.id.as_str(), contract))
        .collect();
    let expected_carriers: BTreeSet<&str> = contracts.keys().copied().collect();
    let mut nodes = BTreeMap::new();
    for node in &input.nodes {
        validate_cleanup_state(node)?;
        require_unique_nonempty_ids(
            "node carrier",
            node.carriers.iter().map(|carrier| carrier.id.as_str()),
            true,
        )?;
        let actual_carriers: BTreeSet<&str> = node
            .carriers
            .iter()
            .map(|carrier| carrier.id.as_str())
            .collect();
        if actual_carriers != expected_carriers {
            return Err(ToolError::invalid(format!(
                "node {:?} carrier IDs must equal the declared carrier-contract set",
                node.id
            )));
        }
        let mut carriers = BTreeMap::new();
        for carrier in &node.carriers {
            if carrier.writer.is_empty() {
                return Err(ToolError::invalid(format!(
                    "node {:?}, carrier {:?} writer must not be empty",
                    node.id, carrier.id
                )));
            }
            let indexed = lts::index_machine(&carrier.machine)?;
            let machine_encoding = cert::canonical_machine_encoding(&carrier.machine)?;
            carriers.insert(
                carrier.id.as_str(),
                PreparedMigrationCarrier {
                    carrier,
                    machine_encoding,
                    actions: indexed.actions,
                },
            );
        }
        nodes.insert(node.id.as_str(), PreparedMigrationNode { node, carriers });
    }

    validate_carrier_contracts(&contracts, &nodes)?;
    let node_ids: BTreeSet<&str> = nodes.keys().copied().collect();
    let mut edge_inputs: Vec<&MigrationEdge> = input.edges.iter().collect();
    edge_inputs.sort_by(|left, right| left.id.cmp(&right.id));
    let mut edges = Vec::with_capacity(edge_inputs.len());
    for edge in edge_inputs {
        validate_migration_edge_shape(edge, &node_ids, &expected_carriers)?;
        let forward_verification = cert::verify(&edge.evidence)?;
        let reverse_verification = edge
            .reverse_evidence
            .as_ref()
            .map(|reverse| cert::verify(&reverse.certificate))
            .transpose()?;
        edges.push(PreparedMigrationEdge {
            edge,
            forward_verification,
            reverse_verification,
        });
    }

    Ok(PreparedMigration {
        contracts,
        nodes,
        edges,
    })
}

fn validate_cleanup_state(node: &MigrationNode) -> Result<(), ToolError> {
    if let Some(cleanup) = &node.cleanup
        && (cleanup.obligation.is_empty() || cleanup.rank == 0)
    {
        return Err(ToolError::invalid(format!(
            "node {:?} cleanup requires a non-empty obligation and positive rank",
            node.id
        )));
    }
    Ok(())
}

fn validate_carrier_contracts(
    contracts: &BTreeMap<&str, &MigrationCarrierContract>,
    nodes: &BTreeMap<&str, PreparedMigrationNode<'_>>,
) -> Result<(), ToolError> {
    for (carrier_id, contract) in contracts {
        let mut node_carriers = nodes.values().map(|node| &node.carriers[carrier_id]);
        let first = node_carriers
            .next()
            .ok_or_else(|| ToolError::internal("validated migration graph has no nodes"))?;
        for carrier in node_carriers {
            if carrier.actions != first.actions {
                return Err(ToolError::invalid(format!(
                    "carrier {carrier_id:?} must use one canonical action alphabet at every node"
                )));
            }
        }
        cert::canonical_context_encoding(contract.required_context.as_ref(), &first.actions)?;
    }
    Ok(())
}

fn validate_migration_edge_shape(
    edge: &MigrationEdge,
    node_ids: &BTreeSet<&str>,
    carrier_ids: &BTreeSet<&str>,
) -> Result<(), ToolError> {
    if !node_ids.contains(edge.source.as_str()) || !node_ids.contains(edge.target.as_str()) {
        return Err(ToolError::invalid(format!(
            "edge {:?} references an unknown source or target node",
            edge.id
        )));
    }
    if edge.operation.is_empty() {
        return Err(ToolError::invalid(format!(
            "edge {:?} operation must not be empty",
            edge.id
        )));
    }
    if !carrier_ids.contains(edge.affected_carrier.as_str()) {
        return Err(ToolError::invalid(format!(
            "edge {:?} names unknown affected carrier {:?}",
            edge.id, edge.affected_carrier
        )));
    }
    validate_transfer_shape(edge.authority_transfer.as_ref(), "authority_transfer")?;
    validate_cleanup_transition_shape(&edge.cleanup_transition, &edge.id)?;
    if let Some(reverse) = &edge.reverse_evidence {
        if reverse.operation.is_empty() {
            return Err(ToolError::invalid(format!(
                "edge {:?} reverse_evidence.operation must not be empty",
                edge.id
            )));
        }
        validate_transfer_shape(
            reverse.authority_transfer.as_ref(),
            "reverse_evidence.authority_transfer",
        )?;
    }
    Ok(())
}

fn validate_transfer_shape(
    transfer: Option<&AuthorityTransfer>,
    field: &str,
) -> Result<(), ToolError> {
    if let Some(transfer) = transfer
        && (transfer.from_writer.is_empty() || transfer.to_writer.is_empty())
    {
        return Err(ToolError::invalid(format!(
            "{field} writer IDs must not be empty"
        )));
    }
    Ok(())
}

fn validate_cleanup_transition_shape(
    transition: &CleanupTransition,
    edge_id: &str,
) -> Result<(), ToolError> {
    let valid = match transition {
        CleanupTransition::Neutral => true,
        CleanupTransition::Introduce {
            obligation,
            to_rank,
        } => !obligation.is_empty() && *to_rank > 0,
        CleanupTransition::Progress {
            obligation,
            from_rank,
            to_rank,
        } => !obligation.is_empty() && *from_rank > 0 && *to_rank > 0,
        CleanupTransition::Discharge {
            obligation,
            from_rank,
        } => !obligation.is_empty() && *from_rank > 0,
    };
    if !valid {
        return Err(ToolError::invalid(format!(
            "edge {edge_id:?} cleanup_transition requires non-empty obligations and positive ranks"
        )));
    }
    Ok(())
}

fn validate_migration_edge(
    prepared: &PreparedMigration<'_>,
    prepared_edge: &PreparedMigrationEdge<'_>,
) -> Result<Option<MigrationViolation>, ToolError> {
    let edge = prepared_edge.edge;
    let source = &prepared.nodes[edge.source.as_str()];
    let target = &prepared.nodes[edge.target.as_str()];
    let changed: Vec<&str> = prepared
        .contracts
        .keys()
        .copied()
        .filter(|carrier_id| {
            source.carriers[carrier_id].machine_encoding
                != target.carriers[carrier_id].machine_encoding
        })
        .collect();
    if changed != [edge.affected_carrier.as_str()] {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "changed_carrier_set_mismatch",
            [
                ("declared", json!([edge.affected_carrier])),
                ("actual", json!(changed)),
            ],
        )));
    }

    let authority_changes: Vec<&str> = prepared
        .contracts
        .keys()
        .copied()
        .filter(|carrier_id| {
            source.carriers[carrier_id].carrier.writer != target.carriers[carrier_id].carrier.writer
        })
        .collect();
    if authority_changes
        .iter()
        .any(|carrier_id| *carrier_id != edge.affected_carrier)
    {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "authority_change_outside_affected_carrier",
            [("actual", json!(authority_changes))],
        )));
    }

    let source_carrier = &source.carriers[edge.affected_carrier.as_str()];
    let target_carrier = &target.carriers[edge.affected_carrier.as_str()];
    let contract = prepared.contracts[edge.affected_carrier.as_str()];
    if let Some(violation) = validate_bound_evidence(
        edge,
        &edge.evidence,
        source_carrier,
        target_carrier,
        contract.required_context.as_ref(),
        "forward",
        &prepared_edge.forward_verification,
    )? {
        return Ok(Some(violation));
    }
    if let Some(violation) = validate_authority_claim(
        edge,
        edge.authority_transfer.as_ref(),
        &source_carrier.carrier.writer,
        &target_carrier.carrier.writer,
        "authority_transfer_mismatch",
    ) {
        return Ok(Some(violation));
    }

    if let Some(reverse) = &edge.reverse_evidence {
        if let Some(violation) = validate_bound_evidence(
            edge,
            &reverse.certificate,
            target_carrier,
            source_carrier,
            contract.required_context.as_ref(),
            "reverse",
            prepared_edge
                .reverse_verification
                .as_ref()
                .ok_or_else(|| ToolError::internal("prepared reverse evidence is missing"))?,
        )? {
            return Ok(Some(violation));
        }
        if let Some(violation) = validate_authority_claim(
            edge,
            reverse.authority_transfer.as_ref(),
            &target_carrier.carrier.writer,
            &source_carrier.carrier.writer,
            "reverse_authority_transfer_mismatch",
        ) {
            return Ok(Some(violation));
        }
    }

    Ok(validate_cleanup_claim(
        edge,
        source.node.cleanup.as_ref(),
        target.node.cleanup.as_ref(),
    ))
}

fn validate_bound_evidence(
    edge: &MigrationEdge,
    certificate: &CertificateInput,
    expected_source: &PreparedMigrationCarrier<'_>,
    expected_target: &PreparedMigrationCarrier<'_>,
    required_context: Option<&ContextMonitor>,
    orientation: &str,
    verification: &cert::CertificateResult,
) -> Result<Option<MigrationViolation>, ToolError> {
    let CertificateClaim::CompareEquivalenceRelation { subject, .. } = &certificate.claim else {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "unsupported_evidence_kind",
            [("orientation", json!(orientation))],
        )));
    };

    let left = cert::canonical_machine_encoding(&subject.left)?;
    if left != expected_source.machine_encoding {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "evidence_source_binding_mismatch",
            [("orientation", json!(orientation))],
        )));
    }
    let right = cert::canonical_machine_encoding(&subject.right)?;
    if right != expected_target.machine_encoding {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "evidence_target_binding_mismatch",
            [("orientation", json!(orientation))],
        )));
    }
    let actual_context =
        cert::canonical_context_encoding(subject.context.as_ref(), &expected_source.actions)?;
    let expected_context =
        cert::canonical_context_encoding(required_context, &expected_source.actions)?;
    if actual_context != expected_context {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "evidence_context_binding_mismatch",
            [("orientation", json!(orientation))],
        )));
    }

    if !verification.accepted {
        return Ok(Some(edge_violation(
            edge,
            Some(&edge.affected_carrier),
            "evidence_rejected",
            [
                ("orientation", json!(orientation)),
                ("rejection", json!(verification.rejection)),
            ],
        )));
    }

    Ok(None)
}

fn validate_authority_claim(
    edge: &MigrationEdge,
    actual: Option<&AuthorityTransfer>,
    from_writer: &str,
    to_writer: &str,
    code: &'static str,
) -> Option<MigrationViolation> {
    let expected = (from_writer != to_writer).then(|| AuthorityTransfer {
        from_writer: from_writer.to_owned(),
        to_writer: to_writer.to_owned(),
    });
    (actual != expected.as_ref()).then(|| {
        edge_violation(
            edge,
            Some(&edge.affected_carrier),
            code,
            [("expected", json!(expected)), ("actual", json!(actual))],
        )
    })
}

fn validate_cleanup_claim(
    edge: &MigrationEdge,
    source: Option<&CleanupState>,
    target: Option<&CleanupState>,
) -> Option<MigrationViolation> {
    let expected = match (source, target) {
        (None, None) => CleanupTransition::Neutral,
        (None, Some(target)) => CleanupTransition::Introduce {
            obligation: target.obligation.clone(),
            to_rank: target.rank,
        },
        (Some(source), None) => CleanupTransition::Discharge {
            obligation: source.obligation.clone(),
            from_rank: source.rank,
        },
        (Some(source), Some(target)) if source.obligation != target.obligation => {
            return Some(edge_violation(
                edge,
                None,
                "cleanup_obligation_swap",
                [("source", json!(source)), ("target", json!(target))],
            ));
        }
        (Some(source), Some(target)) if target.rank >= source.rank => {
            return Some(edge_violation(
                edge,
                None,
                "cleanup_rank_not_decreasing",
                [
                    ("source_rank", json!(source.rank)),
                    ("target_rank", json!(target.rank)),
                ],
            ));
        }
        (Some(source), Some(target)) => CleanupTransition::Progress {
            obligation: source.obligation.clone(),
            from_rank: source.rank,
            to_rank: target.rank,
        },
    };
    (edge.cleanup_transition != expected).then(|| {
        edge_violation(
            edge,
            None,
            "cleanup_transition_claim_mismatch",
            [
                ("expected", json!(expected)),
                ("actual", json!(edge.cleanup_transition)),
            ],
        )
    })
}

fn require_unique_nonempty_ids<'a>(
    kind: &str,
    ids: impl Iterator<Item = &'a str>,
    require_nonempty_collection: bool,
) -> Result<(), ToolError> {
    let mut ordered: Vec<&str> = ids.collect();
    if require_nonempty_collection && ordered.is_empty() {
        return Err(ToolError::invalid(format!("{kind}s must not be empty")));
    }
    if ordered.iter().any(|id| id.is_empty()) {
        return Err(ToolError::invalid(format!("{kind} IDs must not be empty")));
    }
    ordered.sort_unstable();
    if let Some(duplicate) = ordered
        .windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
    {
        return Err(ToolError::invalid(format!(
            "duplicate {kind} ID {duplicate:?}"
        )));
    }
    Ok(())
}

fn edge_violation<const N: usize>(
    edge: &MigrationEdge,
    carrier: Option<&str>,
    code: &'static str,
    details: [(&str, Value); N],
) -> MigrationViolation {
    MigrationViolation {
        kind: "edge",
        id: edge.id.clone(),
        carrier: carrier.map(str::to_owned),
        code,
        details: details
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

fn node_violation<const N: usize>(
    node: &MigrationNode,
    code: &'static str,
    details: [(&str, Value); N],
) -> MigrationViolation {
    MigrationViolation {
        kind: "node",
        id: node.id.clone(),
        carrier: None,
        code,
        details: details
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

fn migration_nonclaims() -> Vec<&'static str> {
    vec![
        "The result is exact only for the supplied finite graph, carrier machines, fixed contexts, certificates, writer labels, and cleanup ranks.",
        "Per-carrier equivalence evidence does not prove interacting-carrier composition or whole-system equivalence.",
        "Revision and operation labels are not Git bindings or evidence that an operation executes.",
        "Writer labels and authority transfers are supplied model facts, not runtime enforcement.",
        "Reverse evidence validates only the supplied reverse semantic and authority claims; it does not prove rollback availability and is not a graph edge.",
        "Certificate producer provenance remains orthogonal to semantic acceptance; an expected-provenance mismatch does not invalidate an otherwise accepted edge proof.",
        "The validator does not add or remove carriers, infer evidence, search paths, optimize migration order, aggregate scalar debt energy, or prove source-code equivalence.",
    ]
}

#[allow(clippy::too_many_lines)]
fn validate_all(input: &CandidateInput) -> Result<Validated<'_>, ToolError> {
    if input.schema_version != CANDIDATE_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {:?}",
            input.schema_version, CANDIDATE_SCHEMA
        )));
    }
    let elements = unique_ids("elements", &input.elements)?;
    let modules = unique_ids("modules", &input.modules)?;
    if input.candidates.is_empty() {
        return Err(ToolError::invalid("candidates must not be empty"));
    }
    let mut candidate_ids = BTreeSet::new();
    for candidate in &input.candidates {
        require_id("candidate", &candidate.id)?;
        if !candidate_ids.insert(candidate.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate candidate ID {:?}",
                candidate.id
            )));
        }
    }

    validate_constraints(&input.constraints, &elements, &modules)?;
    let metric_definitions = validate_metric_definitions(&input.metric_definitions)?;
    let expected_metrics: BTreeSet<&str> = input
        .metric_definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect();
    let authority_groups: BTreeMap<&str, &AuthorityGroup> = input
        .constraints
        .authority_groups
        .iter()
        .map(|group| (group.id.as_str(), group))
        .collect();
    let expected_authority: BTreeSet<&str> = authority_groups.keys().copied().collect();

    let mut candidates = Vec::with_capacity(input.candidates.len());
    for candidate in &input.candidates {
        let assignment_keys: BTreeSet<&str> =
            candidate.assignment.keys().map(String::as_str).collect();
        if assignment_keys != elements {
            return Err(ToolError::invalid(format!(
                "candidate {:?} assignment keys must equal the declared element set",
                candidate.id
            )));
        }
        for (element, module) in &candidate.assignment {
            if !modules.contains(module.as_str()) {
                return Err(ToolError::invalid(format!(
                    "candidate {:?} assigns element {:?} to unknown module {:?}",
                    candidate.id, element, module
                )));
            }
        }
        let authority_keys: BTreeSet<&str> =
            candidate.authority.keys().map(String::as_str).collect();
        if authority_keys != expected_authority {
            return Err(ToolError::invalid(format!(
                "candidate {:?} authority keys must equal the declared authority-group set",
                candidate.id
            )));
        }
        let metric_keys: BTreeSet<&str> = candidate.metrics.keys().map(String::as_str).collect();
        if metric_keys != expected_metrics {
            return Err(ToolError::invalid(format!(
                "candidate {:?} metric keys must equal the declared metric-definition set",
                candidate.id
            )));
        }
        let metrics = input
            .metric_definitions
            .iter()
            .map(|definition| parse_u128(&candidate.metrics[&definition.id], &definition.id))
            .collect::<Result<Vec<_>, _>>()?;

        let mut violations = Vec::new();
        for [left, right] in &input.constraints.co_locate {
            if candidate.assignment[left] != candidate.assignment[right] {
                violations.push(ConstraintViolation {
                    code: "co_location",
                    message: format!("elements {left:?} and {right:?} must be co-located"),
                });
            }
        }
        for [left, right] in &input.constraints.separate {
            if candidate.assignment[left] == candidate.assignment[right] {
                violations.push(ConstraintViolation {
                    code: "separation",
                    message: format!("elements {left:?} and {right:?} must be separated"),
                });
            }
        }
        for (element, module) in &input.constraints.pinned {
            if candidate.assignment[element] != *module {
                violations.push(ConstraintViolation {
                    code: "pin",
                    message: format!("element {element:?} must be assigned to module {module:?}"),
                });
            }
        }
        for (group_id, representative) in &candidate.authority {
            let group = authority_groups[group_id.as_str()];
            if !group.members.contains(representative) {
                violations.push(ConstraintViolation {
                    code: "authority",
                    message: format!(
                        "authority representative {representative:?} is not a member of group {group_id:?}"
                    ),
                });
            }
        }
        candidates.push(ValidatedCandidate {
            candidate,
            metrics,
            violations,
        });
    }

    Ok(Validated {
        metric_definitions,
        candidates,
    })
}

fn validate_constraints(
    constraints: &Constraints,
    elements: &BTreeSet<&str>,
    modules: &BTreeSet<&str>,
) -> Result<(), ToolError> {
    for (kind, pairs) in [
        ("co_locate", &constraints.co_locate),
        ("separate", &constraints.separate),
    ] {
        for pair in pairs {
            for element in pair {
                if !elements.contains(element.as_str()) {
                    return Err(ToolError::invalid(format!(
                        "{kind} references unknown element {element:?}"
                    )));
                }
            }
            if pair[0] == pair[1] {
                return Err(ToolError::invalid(format!(
                    "{kind} pair must contain two distinct elements"
                )));
            }
        }
    }
    for (element, module) in &constraints.pinned {
        if !elements.contains(element.as_str()) || !modules.contains(module.as_str()) {
            return Err(ToolError::invalid(format!(
                "pin {element:?} -> {module:?} references an unknown element or module"
            )));
        }
    }
    let mut group_ids = BTreeSet::new();
    for group in &constraints.authority_groups {
        require_id("authority group", &group.id)?;
        if !group_ids.insert(group.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate authority-group ID {:?}",
                group.id
            )));
        }
        if group.members.is_empty() {
            return Err(ToolError::invalid(format!(
                "authority group {:?} must have at least one member",
                group.id
            )));
        }
        let mut members = BTreeSet::new();
        for member in &group.members {
            if !elements.contains(member.as_str()) {
                return Err(ToolError::invalid(format!(
                    "authority group {:?} references unknown element {:?}",
                    group.id, member
                )));
            }
            if !members.insert(member.as_str()) {
                return Err(ToolError::invalid(format!(
                    "authority group {:?} contains duplicate member {:?}",
                    group.id, member
                )));
            }
        }
    }
    Ok(())
}

fn validate_metric_definitions(
    definitions: &[MetricDefinition],
) -> Result<Vec<(&MetricDefinition, Direction)>, ToolError> {
    if definitions.is_empty() {
        return Err(ToolError::invalid("metric_definitions must not be empty"));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::with_capacity(definitions.len());
    for definition in definitions {
        require_id("metric", &definition.id)?;
        if definition.unit.is_empty() {
            return Err(ToolError::invalid(format!(
                "metric {:?} must declare a non-empty unit",
                definition.id
            )));
        }
        if !ids.insert(definition.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate metric ID {:?}",
                definition.id
            )));
        }
        result.push((definition, definition.direction));
    }
    Ok(result)
}

fn unique_ids<'a>(kind: &str, values: &'a [String]) -> Result<BTreeSet<&'a str>, ToolError> {
    if values.is_empty() {
        return Err(ToolError::invalid(format!("{kind} must not be empty")));
    }
    let mut result = BTreeSet::new();
    for value in values {
        require_id(kind, value)?;
        if !result.insert(value.as_str()) {
            return Err(ToolError::invalid(format!("duplicate {kind} ID {value:?}")));
        }
    }
    Ok(result)
}

fn require_id(kind: &str, value: &str) -> Result<(), ToolError> {
    if value.is_empty() {
        Err(ToolError::invalid(format!("{kind} IDs must not be empty")))
    } else {
        Ok(())
    }
}

fn parse_u128(value: &str, metric: &str) -> Result<u128, ToolError> {
    value.parse::<u128>().map_err(|_| {
        ToolError::invalid(format!(
            "metric {metric:?} value {value:?} must be an unsigned base-10 integer string"
        ))
    })
}

fn dominates(
    left: &ValidatedCandidate<'_>,
    right: &ValidatedCandidate<'_>,
    definitions: &[(&MetricDefinition, Direction)],
) -> bool {
    let mut strictly_better = false;
    for (index, (_, direction)) in definitions.iter().enumerate() {
        let ordering = left.metrics[index].cmp(&right.metrics[index]);
        let no_worse = match direction {
            Direction::Minimize => !ordering.is_gt(),
            Direction::Maximize => !ordering.is_lt(),
        };
        if !no_worse {
            return false;
        }
        if !ordering.is_eq() {
            strictly_better = true;
        }
    }
    strictly_better
}

fn candidate_nonclaims() -> Vec<&'static str> {
    vec![
        "Feasibility covers only the executable constraints represented in this input.",
        "Authority selections and metric values are supplied assertions; the tool validates their shape and references, not their truth in an implementation.",
        "The tool does not accept a user-declared semantic-equivalence boolean as proof.",
        "The frontier is exact only within the supplied candidate set and metric definitions.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert::{
        CERTIFICATE_CHECK_SCHEMA, COMPARE_SUBJECT_SCHEMA, CompareSubject, RelationTuple,
    };
    use crate::lts::{ContextState, State, Transition};

    fn input() -> CandidateInput {
        CandidateInput {
            schema_version: CANDIDATE_SCHEMA.to_owned(),
            elements: vec!["world".to_owned(), "view".to_owned()],
            modules: vec!["core".to_owned(), "ui".to_owned()],
            constraints: Constraints {
                pinned: BTreeMap::from([("world".to_owned(), "core".to_owned())]),
                authority_groups: vec![AuthorityGroup {
                    id: "world_state".to_owned(),
                    members: vec!["world".to_owned()],
                }],
                ..Constraints::default()
            },
            metric_definitions: vec![
                MetricDefinition {
                    id: "boundary_bits".to_owned(),
                    unit: "bits_per_step".to_owned(),
                    direction: Direction::Minimize,
                },
                MetricDefinition {
                    id: "headroom".to_owned(),
                    unit: "bytes_per_second".to_owned(),
                    direction: Direction::Maximize,
                },
            ],
            candidates: vec![
                Candidate {
                    id: "tight".to_owned(),
                    assignment: BTreeMap::from([
                        ("world".to_owned(), "core".to_owned()),
                        ("view".to_owned(), "ui".to_owned()),
                    ]),
                    authority: BTreeMap::from([("world_state".to_owned(), "world".to_owned())]),
                    metrics: BTreeMap::from([
                        ("boundary_bits".to_owned(), "10".to_owned()),
                        ("headroom".to_owned(), "100".to_owned()),
                    ]),
                },
                Candidate {
                    id: "loose".to_owned(),
                    assignment: BTreeMap::from([
                        ("world".to_owned(), "core".to_owned()),
                        ("view".to_owned(), "ui".to_owned()),
                    ]),
                    authority: BTreeMap::from([("world_state".to_owned(), "world".to_owned())]),
                    metrics: BTreeMap::from([
                        ("boundary_bits".to_owned(), "20".to_owned()),
                        ("headroom".to_owned(), "50".to_owned()),
                    ]),
                },
            ],
        }
    }

    fn migration_machine(state_id: &str) -> Machine {
        let transitions = BTreeMap::from([
            (
                "idle".to_owned(),
                Transition {
                    to: state_id.to_owned(),
                    emission: "waited".to_owned(),
                },
            ),
            (
                "step".to_owned(),
                Transition {
                    to: state_id.to_owned(),
                    emission: "done".to_owned(),
                },
            ),
        ]);
        let unreachable = format!("{state_id}-unreachable");
        Machine {
            initial: state_id.to_owned(),
            actions: vec!["step".to_owned(), "idle".to_owned()],
            states: vec![
                State {
                    id: state_id.to_owned(),
                    observation: "ready".to_owned(),
                    transitions,
                },
                State {
                    id: unreachable.clone(),
                    observation: "unused".to_owned(),
                    transitions: BTreeMap::from([
                        (
                            "idle".to_owned(),
                            Transition {
                                to: unreachable.clone(),
                                emission: "waited".to_owned(),
                            },
                        ),
                        (
                            "step".to_owned(),
                            Transition {
                                to: unreachable,
                                emission: "done".to_owned(),
                            },
                        ),
                    ]),
                },
            ],
        }
    }

    fn equivalence_certificate(
        left: &Machine,
        right: &Machine,
        context: Option<ContextMonitor>,
    ) -> CertificateInput {
        let subject = CompareSubject {
            schema_version: COMPARE_SUBJECT_SCHEMA.to_owned(),
            left: left.clone(),
            right: right.clone(),
            context,
        };
        let subject_sha256 =
            cert::compare_subject_sha256(&subject).expect("fixture subject must hash");
        let relation_context = subject
            .context
            .as_ref()
            .map(|context| context.initial.clone());
        CertificateInput {
            schema_version: CERTIFICATE_CHECK_SCHEMA.to_owned(),
            claim: CertificateClaim::CompareEquivalenceRelation {
                relation: vec![RelationTuple {
                    left: left.initial.clone(),
                    right: right.initial.clone(),
                    context: relation_context,
                }],
                subject,
                subject_sha256,
                provenance: None,
            },
            expected_provenance: None,
        }
    }

    fn migration_node(
        id: &str,
        state_id: &str,
        writer: &str,
        cleanup: Option<CleanupState>,
    ) -> MigrationNode {
        MigrationNode {
            id: id.to_owned(),
            carriers: vec![MigrationNodeCarrier {
                id: "world".to_owned(),
                machine: migration_machine(state_id),
                writer: writer.to_owned(),
            }],
            cleanup,
        }
    }

    fn migration_edge(
        id: &str,
        source: &MigrationNode,
        target: &MigrationNode,
        cleanup_transition: CleanupTransition,
    ) -> MigrationEdge {
        let source_carrier = &source.carriers[0];
        let target_carrier = &target.carriers[0];
        let authority_transfer =
            (source_carrier.writer != target_carrier.writer).then(|| AuthorityTransfer {
                from_writer: source_carrier.writer.clone(),
                to_writer: target_carrier.writer.clone(),
            });
        MigrationEdge {
            id: id.to_owned(),
            source: source.id.clone(),
            target: target.id.clone(),
            operation: format!("migrate-{id}"),
            affected_carrier: "world".to_owned(),
            evidence: equivalence_certificate(
                &source_carrier.machine,
                &target_carrier.machine,
                None,
            ),
            authority_transfer,
            cleanup_transition,
            reverse_evidence: None,
        }
    }

    fn migration_graph() -> MigrationGraphInput {
        let nodes = vec![
            migration_node("n0", "s0", "legacy", None),
            migration_node(
                "n1",
                "s1",
                "rust",
                Some(CleanupState {
                    obligation: "remove-adapter".to_owned(),
                    rank: 2,
                }),
            ),
            migration_node(
                "n2",
                "s2",
                "rust",
                Some(CleanupState {
                    obligation: "remove-adapter".to_owned(),
                    rank: 1,
                }),
            ),
            migration_node("n3", "s3", "rust", None),
        ];
        let mut edges = vec![
            migration_edge(
                "e0",
                &nodes[0],
                &nodes[1],
                CleanupTransition::Introduce {
                    obligation: "remove-adapter".to_owned(),
                    to_rank: 2,
                },
            ),
            migration_edge(
                "e1",
                &nodes[1],
                &nodes[2],
                CleanupTransition::Progress {
                    obligation: "remove-adapter".to_owned(),
                    from_rank: 2,
                    to_rank: 1,
                },
            ),
            migration_edge(
                "e2",
                &nodes[2],
                &nodes[3],
                CleanupTransition::Discharge {
                    obligation: "remove-adapter".to_owned(),
                    from_rank: 1,
                },
            ),
            migration_edge("e3", &nodes[3], &nodes[0], CleanupTransition::Neutral),
        ];
        edges[0].reverse_evidence = Some(ReverseEvidence {
            operation: "restore-legacy".to_owned(),
            certificate: equivalence_certificate(
                &nodes[1].carriers[0].machine,
                &nodes[0].carriers[0].machine,
                None,
            ),
            authority_transfer: Some(AuthorityTransfer {
                from_writer: "rust".to_owned(),
                to_writer: "legacy".to_owned(),
            }),
        });
        MigrationGraphInput {
            schema_version: MIGRATION_GRAPH_SCHEMA.to_owned(),
            carrier_contracts: vec![MigrationCarrierContract {
                id: "world".to_owned(),
                required_context: None,
            }],
            nodes,
            edges,
        }
    }

    fn violation_code(input: &MigrationGraphInput) -> Option<&'static str> {
        validate_migration(input)
            .expect("well-shaped migration graph")
            .first_violation
            .map(|violation| violation.code)
    }

    #[test]
    fn pareto_reports_exact_dominator() {
        let result = pareto(&input()).expect("valid candidates");
        assert_eq!(result.frontier, vec!["tight"]);
        assert_eq!(result.dominated[0].id, "loose");
        assert_eq!(result.dominated[0].dominated_by, vec!["tight"]);
    }

    #[test]
    fn pin_violation_is_infeasible_not_a_metric_penalty() {
        let mut input = input();
        input.candidates[0]
            .assignment
            .insert("world".to_owned(), "ui".to_owned());
        let result = validate(&input).expect("valid schema");
        assert!(!result.candidates[0].feasible);
        assert_eq!(result.candidates[0].violations[0].code, "pin");
    }

    #[test]
    fn missing_metric_is_invalid_input() {
        let mut input = input();
        input.candidates[0].metrics.remove("headroom");
        let error = validate(&input).expect_err("missing metric must not become zero");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn equal_vectors_both_remain_nondominated() {
        let mut input = input();
        input.candidates[1].metrics = input.candidates[0].metrics.clone();
        let result = pareto(&input).expect("valid candidates");
        assert_eq!(result.frontier, vec!["loose", "tight"]);
        assert!(result.dominated.is_empty());
    }

    #[test]
    fn migration_accepts_strict_descent_reverse_evidence_and_a_clean_cycle() {
        let result = validate_migration(&migration_graph()).expect("valid migration graph");
        assert!(result.valid);
        assert_eq!(result.nodes_checked, 4);
        assert_eq!(result.edges_checked, 4);
        assert!(result.first_violation.is_none());
    }

    #[test]
    fn migration_structure_rejects_duplicate_carriers_zero_rank_and_action_drift() {
        let mut duplicate = migration_graph();
        duplicate.nodes[0].carriers.push(MigrationNodeCarrier {
            id: "world".to_owned(),
            machine: migration_machine("duplicate"),
            writer: "legacy".to_owned(),
        });
        assert_eq!(
            validate_migration(&duplicate)
                .expect_err("duplicate carrier must fail")
                .code,
            "invalid_input"
        );

        let mut zero_rank = migration_graph();
        zero_rank.nodes[1].cleanup.as_mut().expect("cleanup").rank = 0;
        assert_eq!(
            validate_migration(&zero_rank)
                .expect_err("zero rank must fail")
                .code,
            "invalid_input"
        );

        let mut action_drift = migration_graph();
        action_drift.nodes[2].carriers[0].machine.actions = vec!["other".to_owned()];
        for state in &mut action_drift.nodes[2].carriers[0].machine.states {
            state.transitions = BTreeMap::from([(
                "other".to_owned(),
                Transition {
                    to: state.id.clone(),
                    emission: "done".to_owned(),
                },
            )]);
        }
        assert_eq!(
            validate_migration(&action_drift)
                .expect_err("action drift must fail")
                .code,
            "invalid_input"
        );

        let mut missing_carrier = migration_graph();
        missing_carrier.nodes[0].carriers.clear();
        assert_eq!(
            validate_migration(&missing_carrier)
                .expect_err("incomplete carrier universe must fail")
                .code,
            "invalid_input"
        );

        let mut unknown_endpoint = migration_graph();
        unknown_endpoint.edges[0].source = "ghost".to_owned();
        assert_eq!(
            validate_migration(&unknown_endpoint)
                .expect_err("unknown endpoint must fail")
                .code,
            "invalid_input"
        );
    }

    #[test]
    fn migration_requires_exactly_the_named_changed_carrier() {
        let mut unchanged = migration_graph();
        unchanged.nodes[1].carriers[0].machine = unchanged.nodes[0].carriers[0].machine.clone();
        assert_eq!(
            violation_code(&unchanged),
            Some("changed_carrier_set_mismatch")
        );

        let mut multiple = migration_graph();
        multiple.carrier_contracts.push(MigrationCarrierContract {
            id: "audio".to_owned(),
            required_context: None,
        });
        for (index, node) in multiple.nodes.iter_mut().enumerate() {
            node.carriers.push(MigrationNodeCarrier {
                id: "audio".to_owned(),
                machine: migration_machine(if index == 1 { "a1" } else { "a0" }),
                writer: "rust".to_owned(),
            });
        }
        assert_eq!(
            violation_code(&multiple),
            Some("changed_carrier_set_mismatch")
        );
    }

    #[test]
    fn migration_binds_evidence_direction_context_and_local_acceptance() {
        let mut reversed = migration_graph();
        reversed.edges[0].evidence = equivalence_certificate(
            &reversed.nodes[1].carriers[0].machine,
            &reversed.nodes[0].carriers[0].machine,
            None,
        );
        assert_eq!(
            violation_code(&reversed),
            Some("evidence_source_binding_mismatch")
        );

        let mut wrong_context = migration_graph();
        wrong_context.carrier_contracts[0].required_context = Some(ContextMonitor {
            initial: "c".to_owned(),
            states: vec![ContextState {
                id: "c".to_owned(),
                transitions: BTreeMap::from([("step".to_owned(), "c".to_owned())]),
            }],
        });
        assert_eq!(
            violation_code(&wrong_context),
            Some("evidence_context_binding_mismatch")
        );

        let mut rejected = migration_graph();
        if let CertificateClaim::CompareEquivalenceRelation { relation, .. } =
            &mut rejected.edges[0].evidence.claim
        {
            relation[0].left = "unknown".to_owned();
        }
        assert_eq!(violation_code(&rejected), Some("evidence_rejected"));
    }

    #[test]
    fn migration_rejects_malformed_embedded_certificates_as_invalid_input() {
        let mut wrong_schema = migration_graph();
        wrong_schema.edges[0].evidence.schema_version = "wrong".to_owned();
        assert_eq!(
            validate_migration(&wrong_schema)
                .expect_err("nested certificate schema must remain a typed input error")
                .code,
            "invalid_input"
        );

        let mut invalid_subject = migration_graph();
        if let CertificateClaim::CompareEquivalenceRelation { subject, .. } =
            &mut invalid_subject.edges[0].evidence.claim
        {
            subject.left.initial = "unknown".to_owned();
        }
        assert_eq!(
            validate_migration(&invalid_subject)
                .expect_err("invalid nested subject must remain a typed input error")
                .code,
            "invalid_input"
        );

        let mut masked_by_same_edge = migration_graph();
        masked_by_same_edge.nodes[1].carriers[0].machine =
            masked_by_same_edge.nodes[0].carriers[0].machine.clone();
        masked_by_same_edge.edges[0].evidence.schema_version = "wrong".to_owned();
        assert_eq!(
            validate_migration(&masked_by_same_edge)
                .expect_err("same-edge semantic failure must not mask malformed evidence")
                .code,
            "invalid_input"
        );

        let mut masked_by_earlier_edge = migration_graph();
        if let CertificateClaim::CompareEquivalenceRelation { relation, .. } =
            &mut masked_by_earlier_edge.edges[0].evidence.claim
        {
            relation.clear();
        }
        masked_by_earlier_edge.edges[1].evidence.schema_version = "wrong".to_owned();
        assert_eq!(
            validate_migration(&masked_by_earlier_edge)
                .expect_err("earlier semantic failure must not mask later malformed evidence")
                .code,
            "invalid_input"
        );

        let mut masked_reverse = migration_graph();
        if let CertificateClaim::CompareEquivalenceRelation { relation, .. } =
            &mut masked_reverse.edges[0].evidence.claim
        {
            relation.clear();
        }
        masked_reverse.edges[0]
            .reverse_evidence
            .as_mut()
            .expect("reverse evidence")
            .certificate
            .schema_version = "wrong".to_owned();
        assert_eq!(
            validate_migration(&masked_reverse)
                .expect_err("forward rejection must not mask malformed reverse evidence")
                .code,
            "invalid_input"
        );
    }

    #[test]
    fn migration_binds_forward_and_reverse_authority_without_claiming_rollback() {
        let mut missing = migration_graph();
        missing.edges[0].authority_transfer = None;
        assert_eq!(
            violation_code(&missing),
            Some("authority_transfer_mismatch")
        );

        let mut spurious = migration_graph();
        spurious.edges[1].authority_transfer = Some(AuthorityTransfer {
            from_writer: "rust".to_owned(),
            to_writer: "rust".to_owned(),
        });
        assert_eq!(
            violation_code(&spurious),
            Some("authority_transfer_mismatch")
        );

        let mut reverse = migration_graph();
        reverse.edges[0]
            .reverse_evidence
            .as_mut()
            .expect("reverse evidence")
            .authority_transfer = None;
        assert_eq!(
            violation_code(&reverse),
            Some("reverse_authority_transfer_mismatch")
        );

        let mut outside = migration_graph();
        outside.carrier_contracts.push(MigrationCarrierContract {
            id: "audio".to_owned(),
            required_context: None,
        });
        for node in &mut outside.nodes {
            node.carriers.push(MigrationNodeCarrier {
                id: "audio".to_owned(),
                machine: migration_machine("audio"),
                writer: "rust".to_owned(),
            });
        }
        outside.nodes[1].carriers[1].writer = "other".to_owned();
        assert_eq!(
            violation_code(&outside),
            Some("authority_change_outside_affected_carrier")
        );
    }

    #[test]
    fn migration_binds_reverse_orientation_and_context() {
        let mut orientation = migration_graph();
        orientation.edges[0]
            .reverse_evidence
            .as_mut()
            .expect("reverse evidence")
            .certificate = orientation.edges[0].evidence.clone();
        assert_eq!(
            violation_code(&orientation),
            Some("evidence_source_binding_mismatch")
        );

        let mut wrong_context = migration_graph();
        wrong_context.carrier_contracts[0].required_context = Some(ContextMonitor {
            initial: "allowed".to_owned(),
            states: vec![ContextState {
                id: "allowed".to_owned(),
                transitions: BTreeMap::from([("step".to_owned(), "allowed".to_owned())]),
            }],
        });
        assert_eq!(
            violation_code(&wrong_context),
            Some("evidence_context_binding_mismatch")
        );
    }

    #[test]
    fn migration_rejects_plateau_ascent_swap_claim_drift_and_positive_sink() {
        let mut plateau = migration_graph();
        plateau.nodes[2].cleanup.as_mut().expect("cleanup").rank = 2;
        assert_eq!(
            violation_code(&plateau),
            Some("cleanup_rank_not_decreasing")
        );

        let mut ascent = migration_graph();
        ascent.nodes[2].cleanup.as_mut().expect("cleanup").rank = 3;
        assert_eq!(violation_code(&ascent), Some("cleanup_rank_not_decreasing"));

        let mut swap = migration_graph();
        swap.nodes[2].cleanup.as_mut().expect("cleanup").obligation = "different".to_owned();
        assert_eq!(violation_code(&swap), Some("cleanup_obligation_swap"));

        let mut claim = migration_graph();
        claim.edges[1].cleanup_transition = CleanupTransition::Neutral;
        assert_eq!(
            violation_code(&claim),
            Some("cleanup_transition_claim_mismatch")
        );

        let mut sink = migration_graph();
        sink.edges.retain(|edge| edge.source != "n2");
        let result = validate_migration(&sink).expect("well-shaped graph");
        assert_eq!(
            result.first_violation.expect("positive sink").code,
            "open_cleanup_sink"
        );
    }

    #[test]
    fn migration_uses_canonical_edge_order_and_is_declaration_order_invariant() {
        let mut left = migration_graph();
        left.edges[0].cleanup_transition = CleanupTransition::Neutral;
        left.edges[2].cleanup_transition = CleanupTransition::Neutral;
        left.edges[0].id = "z-invalid".to_owned();
        left.edges[2].id = "a-invalid".to_owned();
        let mut right = migration_graph();
        right.edges[0].cleanup_transition = CleanupTransition::Neutral;
        right.edges[2].cleanup_transition = CleanupTransition::Neutral;
        right.edges[0].id = "z-invalid".to_owned();
        right.edges[2].id = "a-invalid".to_owned();
        right.edges.reverse();
        right.nodes.reverse();
        right.carrier_contracts.reverse();

        let left = validate_migration(&left).expect("well-shaped graph");
        let right = validate_migration(&right).expect("well-shaped graph");
        assert_eq!(
            serde_json::to_value(left).expect("serialize"),
            serde_json::to_value(&right).expect("serialize")
        );
        assert_eq!(right.first_violation.expect("invalid edge").id, "a-invalid");
    }

    #[test]
    fn migration_context_declaration_order_does_not_change_binding() {
        let mut input = migration_graph();
        let mut context = ContextMonitor {
            initial: "live".to_owned(),
            states: vec![
                ContextState {
                    id: "live".to_owned(),
                    transitions: BTreeMap::from([("step".to_owned(), "live".to_owned())]),
                },
                ContextState {
                    id: "unreachable".to_owned(),
                    transitions: BTreeMap::from([("step".to_owned(), "unreachable".to_owned())]),
                },
            ],
        };
        for (edge, (source, target)) in input.edges.iter_mut().zip([(0, 1), (1, 2), (2, 3), (3, 0)])
        {
            edge.evidence = equivalence_certificate(
                &input.nodes[source].carriers[0].machine,
                &input.nodes[target].carriers[0].machine,
                Some(context.clone()),
            );
        }
        input.edges[0]
            .reverse_evidence
            .as_mut()
            .expect("reverse evidence")
            .certificate = equivalence_certificate(
            &input.nodes[1].carriers[0].machine,
            &input.nodes[0].carriers[0].machine,
            Some(context.clone()),
        );
        input.nodes[1].carriers[0].machine.actions.reverse();
        input.nodes[1].carriers[0].machine.states.reverse();
        context.states.reverse();
        input.carrier_contracts[0].required_context = Some(context);

        assert!(validate_migration(&input).expect("valid graph").valid);
    }
}
