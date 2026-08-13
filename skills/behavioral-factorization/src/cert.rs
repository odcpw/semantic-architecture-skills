use crate::lts::{self, ContextMonitor, IndexedContext, IndexedMachine, Limits, Machine};
use crate::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const CERTIFICATE_CHECK_SCHEMA: &str = "kerf.certificate-check.v1";
pub const COMPARE_SUBJECT_SCHEMA: &str = "kerf.cert.compare-subject.v1";
pub const LTS_SUBJECT_SCHEMA: &str = "kerf.cert.lts-subject.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateInput {
    pub schema_version: String,
    pub claim: CertificateClaim,
    #[serde(default)]
    pub expected_provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CertificateClaim {
    CompareEquivalenceRelation {
        subject: CompareSubject,
        subject_sha256: String,
        relation: Vec<RelationTuple>,
        #[serde(default)]
        provenance: Option<Provenance>,
    },
    CompareInequivalenceWitness {
        subject: CompareSubject,
        subject_sha256: String,
        word: Vec<String>,
        mismatch: MismatchClaim,
        #[serde(default)]
        provenance: Option<Provenance>,
    },
    LtsTrace {
        subject: LtsSubject,
        subject_sha256: String,
        initial: TracePoint,
        steps: Vec<TraceStep>,
        #[serde(default)]
        provenance: Option<Provenance>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompareSubject {
    pub schema_version: String,
    pub left: Machine,
    pub right: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LtsSubject {
    pub schema_version: String,
    pub machine: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RelationTuple {
    pub left: String,
    pub right: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MismatchKind {
    StateObservation,
    TransitionEmission,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MismatchClaim {
    pub kind: MismatchKind,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TracePoint {
    pub state: String,
    pub context: Option<String>,
    pub observation: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceStep {
    pub action: String,
    pub emission: String,
    pub to: TracePoint,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub producer: String,
    pub operation: String,
    pub input_sha256: String,
    pub tool_version: String,
    #[serde(default)]
    pub limits: Option<Limits>,
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateKind {
    CompareEquivalenceRelation,
    CompareInequivalenceWitness,
    LtsTrace,
}

#[derive(Debug, Serialize)]
pub struct CertificateResult {
    pub algorithm: &'static str,
    pub kind: CertificateKind,
    pub accepted: bool,
    pub subject_schema: &'static str,
    pub subject_sha256: String,
    pub provenance: ProvenanceResult,
    pub rejection: Option<CertificateRejection>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceStatus {
    Absent,
    PresentUnchecked,
    Match,
    Mismatch,
}

#[derive(Debug, Serialize)]
pub struct ProvenanceResult {
    pub status: ProvenanceStatus,
    pub mismatch_fields: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct CertificateRejection {
    pub code: &'static str,
    pub details: BTreeMap<&'static str, Value>,
}

struct PairIndex {
    left: IndexedMachine,
    right: IndexedMachine,
    context: IndexedContext,
    context_present: bool,
    subject_sha256: String,
}

struct TraceIndex {
    machine: IndexedMachine,
    context: IndexedContext,
    context_present: bool,
    subject_sha256: String,
}

type PairState = (usize, usize, usize);

struct RelationNode {
    state: PairState,
    parent: Option<(usize, usize)>,
}

/// Check one finite certificate against its supplied semantic subject.
///
/// # Errors
///
/// Returns [`ToolError`] only when the request schema or semantic subject is
/// invalid. A well-formed but false or tampered certificate is a completed
/// result with `accepted: false`.
pub fn verify(input: &CertificateInput) -> Result<CertificateResult, ToolError> {
    lts::require_schema(&input.schema_version, CERTIFICATE_CHECK_SCHEMA)?;
    match &input.claim {
        CertificateClaim::CompareEquivalenceRelation {
            subject,
            subject_sha256,
            relation,
            provenance,
        } => {
            let indexed = prepare_pair(subject)?;
            let provenance_result =
                check_provenance(provenance.as_ref(), input.expected_provenance.as_ref());
            let rejection = verify_subject_binding(subject_sha256, &indexed.subject_sha256)
                .or_else(|| verify_relation(&indexed, relation));
            Ok(result(
                CertificateKind::CompareEquivalenceRelation,
                COMPARE_SUBJECT_SCHEMA,
                indexed.subject_sha256,
                provenance_result,
                rejection,
            ))
        }
        CertificateClaim::CompareInequivalenceWitness {
            subject,
            subject_sha256,
            word,
            mismatch,
            provenance,
        } => {
            let indexed = prepare_pair(subject)?;
            let provenance_result =
                check_provenance(provenance.as_ref(), input.expected_provenance.as_ref());
            let rejection = verify_subject_binding(subject_sha256, &indexed.subject_sha256)
                .or_else(|| verify_witness(&indexed, word, mismatch));
            Ok(result(
                CertificateKind::CompareInequivalenceWitness,
                COMPARE_SUBJECT_SCHEMA,
                indexed.subject_sha256,
                provenance_result,
                rejection,
            ))
        }
        CertificateClaim::LtsTrace {
            subject,
            subject_sha256,
            initial,
            steps,
            provenance,
        } => {
            let indexed = prepare_trace(subject)?;
            let provenance_result =
                check_provenance(provenance.as_ref(), input.expected_provenance.as_ref());
            let rejection = verify_subject_binding(subject_sha256, &indexed.subject_sha256)
                .or_else(|| verify_trace(&indexed, initial, steps));
            Ok(result(
                CertificateKind::LtsTrace,
                LTS_SUBJECT_SCHEMA,
                indexed.subject_sha256,
                provenance_result,
                rejection,
            ))
        }
    }
}

/// Compute the canonical semantic identity used by LTS trace certificates.
///
/// # Errors
///
/// Returns [`ToolError`] when the finite machine or context is invalid.
pub fn trace_subject_sha256(subject: &LtsSubject) -> Result<String, ToolError> {
    Ok(prepare_trace(subject)?.subject_sha256)
}

fn result(
    kind: CertificateKind,
    subject_schema: &'static str,
    subject_sha256: String,
    provenance: ProvenanceResult,
    rejection: Option<CertificateRejection>,
) -> CertificateResult {
    CertificateResult {
        algorithm: "local_finite_certificate_check_v1",
        kind,
        accepted: rejection.is_none(),
        subject_schema,
        subject_sha256,
        provenance,
        rejection,
        nonclaims: vec![
            "Validity is exact only for the supplied finite deterministic subject and context monitor.",
            "The subject hash is an identity binding, not a signature or authenticity claim.",
            "Producer provenance does not affect semantic certificate validity.",
            "A cross-machine relation certifies equivalence but is not claimed to be an equivalence relation on one common carrier.",
            "No source-code equivalence, shortest-witness, relation-minimality, liveness, fairness, timing, or unmodeled-effect claim is made.",
        ],
    }
}

fn prepare_pair(subject: &CompareSubject) -> Result<PairIndex, ToolError> {
    lts::require_schema(&subject.schema_version, COMPARE_SUBJECT_SCHEMA)?;
    let left = lts::index_machine(&subject.left)?;
    let right = lts::index_machine(&subject.right)?;
    if left.actions != right.actions {
        return Err(ToolError::invalid(
            "left and right machines must declare the same canonically sorted action set",
        ));
    }
    let context = lts::index_context(subject.context.as_ref(), &left.actions)?;
    let subject_sha256 = hash_pair_subject(&left, &right, &context, subject.context.is_some())?;
    Ok(PairIndex {
        left,
        right,
        context,
        context_present: subject.context.is_some(),
        subject_sha256,
    })
}

fn prepare_trace(subject: &LtsSubject) -> Result<TraceIndex, ToolError> {
    lts::require_schema(&subject.schema_version, LTS_SUBJECT_SCHEMA)?;
    let machine = lts::index_machine(&subject.machine)?;
    let context = lts::index_context(subject.context.as_ref(), &machine.actions)?;
    let subject_sha256 = hash_trace_subject(&machine, &context, subject.context.is_some())?;
    Ok(TraceIndex {
        machine,
        context,
        context_present: subject.context.is_some(),
        subject_sha256,
    })
}

fn verify_subject_binding(claimed: &str, actual: &str) -> Option<CertificateRejection> {
    if !is_canonical_sha256(claimed) {
        return Some(rejection(
            "subject_hash_invalid_format",
            [("claimed_sha256", json!(claimed))],
        ));
    }
    (claimed != actual).then(|| {
        rejection(
            "subject_hash_mismatch",
            [
                ("claimed_sha256", json!(claimed)),
                ("actual_sha256", json!(actual)),
            ],
        )
    })
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn verify_relation(
    indexed: &PairIndex,
    relation: &[RelationTuple],
) -> Option<CertificateRejection> {
    if relation.is_empty() {
        return Some(rejection(
            "certificate_malformed",
            [("reason", json!("relation must not be empty"))],
        ));
    }
    let mut ordered = relation.to_vec();
    ordered.sort();
    if let Some([duplicate, _]) = ordered.windows(2).find(|pair| pair[0] == pair[1]) {
        return Some(rejection(
            "duplicate_relation_tuple",
            [("tuple", json!(duplicate))],
        ));
    }

    let tuples = match resolve_relation(indexed, &ordered) {
        Ok(tuples) => tuples,
        Err(error) => return Some(error),
    };
    let relation_set: BTreeSet<PairState> = tuples.iter().map(|(_, triple)| *triple).collect();
    let initial = (
        indexed.left.initial,
        indexed.right.initial,
        indexed.context.initial,
    );
    if !relation_set.contains(&initial) {
        return Some(rejection(
            "missing_initial_tuple",
            [("expected_tuple", json!(public_tuple(indexed, initial)))],
        ));
    }

    let access = match verify_relation_closure(indexed, &relation_set, initial) {
        Ok(access) => access,
        Err(error) => return Some(error),
    };
    tuples
        .iter()
        .find(|(_, triple)| !access.contains_key(triple))
        .map(|(tuple, _)| rejection("unreachable_relation_tuple", [("tuple", json!(tuple))]))
}

fn resolve_relation(
    indexed: &PairIndex,
    ordered: &[RelationTuple],
) -> Result<Vec<(RelationTuple, PairState)>, CertificateRejection> {
    let left_ids = state_id_map(&indexed.left);
    let right_ids = state_id_map(&indexed.right);
    let context_ids = context_id_map(&indexed.context);
    let mut tuples = Vec::with_capacity(ordered.len());
    for tuple in ordered {
        let triple = resolve_relation_tuple(
            tuple,
            &left_ids,
            &right_ids,
            &context_ids,
            indexed.context_present,
        )?;
        tuples.push((tuple.clone(), triple));
    }
    Ok(tuples)
}

fn verify_relation_closure(
    indexed: &PairIndex,
    relation_set: &BTreeSet<PairState>,
    initial: PairState,
) -> Result<BTreeMap<PairState, usize>, CertificateRejection> {
    let mut nodes = vec![RelationNode {
        state: initial,
        parent: None,
    }];
    let mut access = BTreeMap::from([(initial, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    while let Some(node_index) = queue.pop_front() {
        let tuple = nodes[node_index].state;
        let left_state = &indexed.left.states[tuple.0];
        let right_state = &indexed.right.states[tuple.1];
        if left_state.observation != right_state.observation {
            let word = relation_access_word(&nodes, node_index, &indexed.left.actions);
            return Err(rejection(
                "observation_mismatch",
                [
                    ("tuple", json!(public_tuple(indexed, tuple))),
                    ("access_word", json!(word)),
                    ("left", json!(left_state.observation)),
                    ("right", json!(right_state.observation)),
                ],
            ));
        }
        for (action_index, context_target) in indexed.context.states[tuple.2]
            .transitions
            .iter()
            .enumerate()
        {
            let Some(context_target) = context_target else {
                continue;
            };
            let action = &indexed.left.actions[action_index];
            let left_transition = &left_state.transitions[action_index];
            let right_transition = &right_state.transitions[action_index];
            if left_transition.emission != right_transition.emission {
                let word = relation_access_word(&nodes, node_index, &indexed.left.actions);
                let mut exposure = word.clone();
                exposure.push(action.clone());
                return Err(rejection(
                    "emission_mismatch",
                    [
                        ("tuple", json!(public_tuple(indexed, tuple))),
                        ("access_word", json!(word)),
                        ("action", json!(action)),
                        ("exposure_word", json!(exposure)),
                        ("left", json!(left_transition.emission)),
                        ("right", json!(right_transition.emission)),
                    ],
                ));
            }
            let successor = (left_transition.to, right_transition.to, *context_target);
            if !relation_set.contains(&successor) {
                let word = relation_access_word(&nodes, node_index, &indexed.left.actions);
                return Err(rejection(
                    "missing_closure_tuple",
                    [
                        ("tuple", json!(public_tuple(indexed, tuple))),
                        ("access_word", json!(word)),
                        ("action", json!(action)),
                        (
                            "expected_successor",
                            json!(public_tuple(indexed, successor)),
                        ),
                    ],
                ));
            }
            if let Entry::Vacant(entry) = access.entry(successor) {
                let successor_index = nodes.len();
                entry.insert(successor_index);
                nodes.push(RelationNode {
                    state: successor,
                    parent: Some((node_index, action_index)),
                });
                queue.push_back(successor_index);
            }
        }
    }
    Ok(access)
}

fn relation_access_word(
    nodes: &[RelationNode],
    mut node_index: usize,
    actions: &[String],
) -> Vec<String> {
    let mut action_indexes = Vec::new();
    while let Some((parent, action)) = nodes[node_index].parent {
        action_indexes.push(action);
        node_index = parent;
    }
    action_indexes.reverse();
    action_indexes
        .into_iter()
        .map(|action| actions[action].clone())
        .collect()
}

fn resolve_relation_tuple(
    tuple: &RelationTuple,
    left_ids: &BTreeMap<&str, usize>,
    right_ids: &BTreeMap<&str, usize>,
    context_ids: &BTreeMap<&str, usize>,
    context_present: bool,
) -> Result<(usize, usize, usize), CertificateRejection> {
    let Some(left) = left_ids.get(tuple.left.as_str()).copied() else {
        return Err(rejection(
            "unknown_relation_state",
            [("side", json!("left")), ("state", json!(tuple.left))],
        ));
    };
    let Some(right) = right_ids.get(tuple.right.as_str()).copied() else {
        return Err(rejection(
            "unknown_relation_state",
            [("side", json!("right")), ("state", json!(tuple.right))],
        ));
    };
    let context = match (context_present, tuple.context.as_deref()) {
        (false, None) => 0,
        (false, Some(_)) | (true, None) => {
            return Err(rejection(
                "tuple_context_shape_mismatch",
                [("tuple", json!(tuple))],
            ));
        }
        (true, Some(id)) => {
            let Some(context) = context_ids.get(id).copied() else {
                return Err(rejection(
                    "unknown_relation_context",
                    [("context", json!(id))],
                ));
            };
            context
        }
    };
    Ok((left, right, context))
}

fn verify_witness(
    indexed: &PairIndex,
    word: &[String],
    claimed: &MismatchClaim,
) -> Option<CertificateRejection> {
    if word.is_empty() && claimed.kind == MismatchKind::TransitionEmission {
        return Some(rejection("transition_witness_requires_nonempty_word", []));
    }

    let mut left = indexed.left.initial;
    let mut right = indexed.right.initial;
    let mut context = indexed.context.initial;
    let initial = observation_mismatch(indexed, left, right);
    if let Some(actual) = initial {
        if word.is_empty() && claimed == &actual {
            return None;
        }
        return Some(rejection(
            if word.is_empty() {
                "claimed_mismatch_values_mismatch"
            } else {
                "earlier_observation_mismatch"
            },
            [("actual_mismatch", json!(actual)), ("prefix", json!([]))],
        ));
    }
    if word.is_empty() {
        return Some(rejection("word_not_distinguishing", []));
    }

    for (step_index, action) in word.iter().enumerate() {
        let Ok(action_index) = indexed.left.actions.binary_search(action) else {
            return Some(rejection(
                "witness_unknown_action",
                [("step_index", json!(step_index)), ("action", json!(action))],
            ));
        };
        let Some(context_target) = indexed.context.states[context].transitions[action_index] else {
            return Some(rejection(
                "witness_action_disabled",
                [("step_index", json!(step_index)), ("action", json!(action))],
            ));
        };
        let left_transition = &indexed.left.states[left].transitions[action_index];
        let right_transition = &indexed.right.states[right].transitions[action_index];
        let final_step = step_index + 1 == word.len();
        if left_transition.emission != right_transition.emission {
            let actual = MismatchClaim {
                kind: MismatchKind::TransitionEmission,
                left: left_transition.emission.clone(),
                right: right_transition.emission.clone(),
            };
            if final_step && claimed == &actual {
                return None;
            }
            return Some(rejection(
                if final_step {
                    "claimed_mismatch_values_mismatch"
                } else {
                    "earlier_emission_mismatch"
                },
                [
                    ("step_index", json!(step_index)),
                    ("actual_mismatch", json!(actual)),
                ],
            ));
        }
        left = left_transition.to;
        right = right_transition.to;
        context = context_target;
        if let Some(actual) = observation_mismatch(indexed, left, right) {
            if final_step && claimed == &actual {
                return None;
            }
            return Some(rejection(
                if final_step {
                    "claimed_mismatch_values_mismatch"
                } else {
                    "earlier_observation_mismatch"
                },
                [
                    ("step_index", json!(step_index)),
                    ("actual_mismatch", json!(actual)),
                ],
            ));
        }
        if final_step {
            return Some(rejection("word_not_distinguishing", []));
        }
    }
    Some(rejection("word_not_distinguishing", []))
}

fn observation_mismatch(indexed: &PairIndex, left: usize, right: usize) -> Option<MismatchClaim> {
    let left_observation = &indexed.left.states[left].observation;
    let right_observation = &indexed.right.states[right].observation;
    (left_observation != right_observation).then(|| MismatchClaim {
        kind: MismatchKind::StateObservation,
        left: left_observation.clone(),
        right: right_observation.clone(),
    })
}

fn verify_trace(
    indexed: &TraceIndex,
    initial: &TracePoint,
    steps: &[TraceStep],
) -> Option<CertificateRejection> {
    if let Some(error) = verify_trace_initial(indexed, initial) {
        return Some(error);
    }
    let mut machine = indexed.machine.initial;
    let mut context = indexed.context.initial;
    for (step_index, step) in steps.iter().enumerate() {
        match verify_trace_step(indexed, machine, context, step_index, step) {
            Ok((next_machine, next_context)) => {
                machine = next_machine;
                context = next_context;
            }
            Err(error) => return Some(error),
        }
    }
    None
}

fn verify_trace_initial(
    indexed: &TraceIndex,
    initial: &TracePoint,
) -> Option<CertificateRejection> {
    let actual_state = &indexed.machine.states[indexed.machine.initial];
    if initial.state != actual_state.id {
        return Some(rejection(
            "trace_initial_state_mismatch",
            [
                ("expected", json!(actual_state.id)),
                ("actual", json!(initial.state)),
            ],
        ));
    }
    let actual_context = public_context(
        indexed.context_present,
        &indexed.context,
        indexed.context.initial,
    );
    if initial.context != actual_context {
        return Some(rejection(
            "trace_initial_context_mismatch",
            [
                ("expected", json!(actual_context)),
                ("actual", json!(initial.context)),
            ],
        ));
    }
    if initial.observation != actual_state.observation {
        return Some(rejection(
            "trace_initial_observation_mismatch",
            [
                ("expected", json!(actual_state.observation)),
                ("actual", json!(initial.observation)),
            ],
        ));
    }
    None
}

fn verify_trace_step(
    indexed: &TraceIndex,
    machine: usize,
    context: usize,
    step_index: usize,
    step: &TraceStep,
) -> Result<(usize, usize), CertificateRejection> {
    let Ok(action_index) = indexed.machine.actions.binary_search(&step.action) else {
        return Err(rejection(
            "trace_unknown_action",
            [
                ("step_index", json!(step_index)),
                ("action", json!(step.action)),
            ],
        ));
    };
    let Some(context_target) = indexed.context.states[context].transitions[action_index] else {
        return Err(rejection(
            "trace_action_disabled",
            [
                ("step_index", json!(step_index)),
                ("action", json!(step.action)),
            ],
        ));
    };
    let transition = &indexed.machine.states[machine].transitions[action_index];
    if step.emission != transition.emission {
        return Err(rejection(
            "trace_emission_mismatch",
            [
                ("step_index", json!(step_index)),
                ("expected", json!(transition.emission)),
                ("actual", json!(step.emission)),
            ],
        ));
    }
    let target = &indexed.machine.states[transition.to];
    if step.to.state != target.id {
        return Err(rejection(
            "trace_successor_mismatch",
            [
                ("step_index", json!(step_index)),
                ("expected", json!(target.id)),
                ("actual", json!(step.to.state)),
            ],
        ));
    }
    let expected_context =
        public_context(indexed.context_present, &indexed.context, context_target);
    if step.to.context != expected_context {
        return Err(rejection(
            "trace_context_successor_mismatch",
            [
                ("step_index", json!(step_index)),
                ("expected", json!(expected_context)),
                ("actual", json!(step.to.context)),
            ],
        ));
    }
    if step.to.observation != target.observation {
        return Err(rejection(
            "trace_observation_mismatch",
            [
                ("step_index", json!(step_index)),
                ("expected", json!(target.observation)),
                ("actual", json!(step.to.observation)),
            ],
        ));
    }
    Ok((transition.to, context_target))
}

fn state_id_map(machine: &IndexedMachine) -> BTreeMap<&str, usize> {
    machine
        .states
        .iter()
        .enumerate()
        .map(|(index, state)| (state.id.as_str(), index))
        .collect()
}

fn context_id_map(context: &IndexedContext) -> BTreeMap<&str, usize> {
    context
        .states
        .iter()
        .enumerate()
        .map(|(index, state)| (state.id.as_str(), index))
        .collect()
}

fn public_tuple(indexed: &PairIndex, tuple: (usize, usize, usize)) -> RelationTuple {
    RelationTuple {
        left: indexed.left.states[tuple.0].id.clone(),
        right: indexed.right.states[tuple.1].id.clone(),
        context: public_context(indexed.context_present, &indexed.context, tuple.2),
    }
}

fn public_context(present: bool, context: &IndexedContext, state: usize) -> Option<String> {
    present.then(|| context.states[state].id.clone())
}

fn check_provenance(
    actual: Option<&Provenance>,
    expected: Option<&Provenance>,
) -> ProvenanceResult {
    match (actual, expected) {
        (None, None) => ProvenanceResult {
            status: ProvenanceStatus::Absent,
            mismatch_fields: Vec::new(),
        },
        (Some(_), None) => ProvenanceResult {
            status: ProvenanceStatus::PresentUnchecked,
            mismatch_fields: Vec::new(),
        },
        (None, Some(_)) => ProvenanceResult {
            status: ProvenanceStatus::Mismatch,
            mismatch_fields: vec!["missing_provenance"],
        },
        (Some(actual), Some(expected)) => {
            let mut fields = Vec::new();
            if actual.producer != expected.producer {
                fields.push("producer");
            }
            if actual.operation != expected.operation {
                fields.push("operation");
            }
            if actual.input_sha256 != expected.input_sha256 {
                fields.push("input_sha256");
            }
            if actual.tool_version != expected.tool_version {
                fields.push("tool_version");
            }
            if actual.limits != expected.limits {
                fields.push("limits");
            }
            if actual.revision != expected.revision {
                fields.push("revision");
            }
            ProvenanceResult {
                status: if fields.is_empty() {
                    ProvenanceStatus::Match
                } else {
                    ProvenanceStatus::Mismatch
                },
                mismatch_fields: fields,
            }
        }
    }
}

fn rejection<const N: usize>(
    code: &'static str,
    details: [(&'static str, Value); N],
) -> CertificateRejection {
    CertificateRejection {
        code,
        details: details.into_iter().collect(),
    }
}

fn hash_pair_subject(
    left: &IndexedMachine,
    right: &IndexedMachine,
    context: &IndexedContext,
    context_present: bool,
) -> Result<String, ToolError> {
    let mut bytes = b"kerf.cert.subject.v1\0pair\0".to_vec();
    encode_actions(&mut bytes, &left.actions)?;
    encode_machine(&mut bytes, left)?;
    encode_machine(&mut bytes, right)?;
    encode_context(&mut bytes, context, context_present)?;
    Ok(sha256_hex(&bytes))
}

pub(crate) fn canonical_machine_encoding(machine: &Machine) -> Result<Vec<u8>, ToolError> {
    let indexed = lts::index_machine(machine)?;
    let mut bytes = b"kerf.cert.machine.v1\0".to_vec();
    encode_actions(&mut bytes, &indexed.actions)?;
    encode_machine(&mut bytes, &indexed)?;
    Ok(bytes)
}

#[cfg(test)]
pub(crate) fn compare_subject_sha256(subject: &CompareSubject) -> Result<String, ToolError> {
    Ok(prepare_pair(subject)?.subject_sha256)
}

pub(crate) fn canonical_context_encoding(
    context: Option<&ContextMonitor>,
    actions: &[String],
) -> Result<Vec<u8>, ToolError> {
    let indexed = lts::index_context(context, actions)?;
    let mut bytes = b"kerf.cert.context.v1\0".to_vec();
    encode_actions(&mut bytes, actions)?;
    encode_context(&mut bytes, &indexed, context.is_some())?;
    Ok(bytes)
}

fn hash_trace_subject(
    machine: &IndexedMachine,
    context: &IndexedContext,
    context_present: bool,
) -> Result<String, ToolError> {
    let mut bytes = b"kerf.cert.subject.v1\0single\0".to_vec();
    encode_actions(&mut bytes, &machine.actions)?;
    encode_machine(&mut bytes, machine)?;
    encode_context(&mut bytes, context, context_present)?;
    Ok(sha256_hex(&bytes))
}

fn encode_actions(bytes: &mut Vec<u8>, actions: &[String]) -> Result<(), ToolError> {
    push_usize(bytes, actions.len())?;
    for action in actions {
        push_string(bytes, action)?;
    }
    Ok(())
}

fn encode_machine(bytes: &mut Vec<u8>, machine: &IndexedMachine) -> Result<(), ToolError> {
    let (order, canonical) = canonical_order(machine.states.iter().map(|state| state.id.as_str()));
    push_usize(bytes, machine.states.len())?;
    push_usize(bytes, canonical[machine.initial])?;
    for old_index in order {
        let state = &machine.states[old_index];
        push_string(bytes, &state.id)?;
        push_string(bytes, &state.observation)?;
        for transition in &state.transitions {
            push_usize(bytes, canonical[transition.to])?;
            push_string(bytes, &transition.emission)?;
        }
    }
    Ok(())
}

fn encode_context(
    bytes: &mut Vec<u8>,
    context: &IndexedContext,
    present: bool,
) -> Result<(), ToolError> {
    bytes.push(u8::from(present));
    if !present {
        return Ok(());
    }
    let (order, canonical) = canonical_order(context.states.iter().map(|state| state.id.as_str()));
    push_usize(bytes, context.states.len())?;
    push_usize(bytes, canonical[context.initial])?;
    for old_index in order {
        let state = &context.states[old_index];
        push_string(bytes, &state.id)?;
        for transition in &state.transitions {
            match transition {
                None => bytes.push(0),
                Some(target) => {
                    bytes.push(1);
                    push_usize(bytes, canonical[*target])?;
                }
            }
        }
    }
    Ok(())
}

fn canonical_order<'a>(ids: impl Iterator<Item = &'a str>) -> (Vec<usize>, Vec<usize>) {
    let mut order: Vec<(usize, &'a str)> = ids.enumerate().collect();
    order.sort_by_key(|(_, id)| *id);
    let mut canonical = vec![0; order.len()];
    for (canonical_index, (old_index, _)) in order.iter().enumerate() {
        canonical[*old_index] = canonical_index;
    }
    (
        order.into_iter().map(|(index, _)| index).collect(),
        canonical,
    )
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), ToolError> {
    push_usize(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn push_usize(bytes: &mut Vec<u8>, value: usize) -> Result<(), ToolError> {
    let value = u64::try_from(value)
        .map_err(|_| ToolError::internal("subject is too large for the v1 u64 length encoding"))?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &Value) -> CertificateInput {
        serde_json::from_value(value.clone()).expect("test certificate must deserialize")
    }

    fn rebind(value: &mut Value) -> String {
        value["claim"]["subject_sha256"] = json!("");
        let hash = verify(&parse(value))
            .expect("test subject must validate")
            .subject_sha256;
        value["claim"]["subject_sha256"] = json!(hash);
        hash
    }

    fn check(value: &Value) -> CertificateResult {
        verify(&parse(value)).expect("test subject must validate")
    }

    fn rejection_code(result: &CertificateResult) -> Option<&str> {
        result.rejection.as_ref().map(|rejection| rejection.code)
    }

    fn relation_certificate() -> Value {
        let mut value = json!({
            "schema_version": CERTIFICATE_CHECK_SCHEMA,
            "claim": {
                "kind": "compare_equivalence_relation",
                "subject": {
                    "schema_version": COMPARE_SUBJECT_SCHEMA,
                    "left": {
                        "initial": "l0",
                        "actions": ["b", "a"],
                        "states": [
                            {"id": "l0", "observation": "idle", "transitions": {
                                "a": {"to": "l1", "emission": "enter"},
                                "b": {"to": "l0", "emission": "idle"}
                            }},
                            {"id": "l1", "observation": "active", "transitions": {
                                "a": {"to": "l1", "emission": "hidden-left"},
                                "b": {"to": "l0", "emission": "leave"}
                            }}
                        ]
                    },
                    "right": {
                        "initial": "r0",
                        "actions": ["a", "b"],
                        "states": [
                            {"id": "r0", "observation": "idle", "transitions": {
                                "a": {"to": "r1", "emission": "enter"},
                                "b": {"to": "r0", "emission": "idle"}
                            }},
                            {"id": "r1", "observation": "active", "transitions": {
                                "a": {"to": "r0", "emission": "hidden-right"},
                                "b": {"to": "r0", "emission": "leave"}
                            }}
                        ]
                    },
                    "context": {
                        "initial": "c0",
                        "states": [
                            {"id": "c0", "transitions": {"a": "c1", "b": "c0"}},
                            {"id": "c1", "transitions": {"b": "c0"}}
                        ]
                    }
                },
                "subject_sha256": "",
                "relation": [
                    {"left": "l0", "right": "r0", "context": "c0"},
                    {"left": "l1", "right": "r1", "context": "c1"}
                ]
            }
        });
        rebind(&mut value);
        value
    }

    fn witness_certificate() -> Value {
        let machine = |state: &str, differing_emission: &str| {
            json!({
                "initial": state,
                "actions": ["b", "a"],
                "states": [{
                    "id": state,
                    "observation": "same",
                    "transitions": {
                        "a": {"to": state, "emission": "same"},
                        "b": {"to": state, "emission": differing_emission}
                    }
                }]
            })
        };
        let mut value = json!({
            "schema_version": CERTIFICATE_CHECK_SCHEMA,
            "claim": {
                "kind": "compare_inequivalence_witness",
                "subject": {
                    "schema_version": COMPARE_SUBJECT_SCHEMA,
                    "left": machine("left", "left-value"),
                    "right": machine("right", "right-value"),
                    "context": null
                },
                "subject_sha256": "",
                "word": ["b"],
                "mismatch": {
                    "kind": "transition_emission",
                    "left": "left-value",
                    "right": "right-value"
                }
            }
        });
        rebind(&mut value);
        value
    }

    fn trace_certificate() -> Value {
        let mut value = json!({
            "schema_version": CERTIFICATE_CHECK_SCHEMA,
            "claim": {
                "kind": "lts_trace",
                "subject": {
                    "schema_version": LTS_SUBJECT_SCHEMA,
                    "machine": {
                        "initial": "s0",
                        "actions": ["reset", "go"],
                        "states": [
                            {"id": "s0", "observation": "idle", "transitions": {
                                "go": {"to": "s1", "emission": "enter"},
                                "reset": {"to": "s0", "emission": "idle"}
                            }},
                            {"id": "s1", "observation": "active", "transitions": {
                                "go": {"to": "s1", "emission": "loop"},
                                "reset": {"to": "s0", "emission": "leave"}
                            }}
                        ]
                    },
                    "context": {
                        "initial": "c0",
                        "states": [
                            {"id": "c0", "transitions": {"go": "c1"}},
                            {"id": "c1", "transitions": {"reset": "c0"}}
                        ]
                    }
                },
                "subject_sha256": "",
                "initial": {"state": "s0", "context": "c0", "observation": "idle"},
                "steps": [
                    {"action": "go", "emission": "enter", "to": {
                        "state": "s1", "context": "c1", "observation": "active"
                    }},
                    {"action": "reset", "emission": "leave", "to": {
                        "state": "s0", "context": "c0", "observation": "idle"
                    }}
                ]
            }
        });
        rebind(&mut value);
        value
    }

    #[test]
    fn relation_accepts_context_screened_behavior_and_is_order_invariant() {
        let value = relation_certificate();
        let result = check(&value);
        assert!(result.accepted);
        let expected_hash = result.subject_sha256;

        let mut reordered = value;
        reordered["claim"]["subject"]["left"]["actions"]
            .as_array_mut()
            .expect("actions must be an array")
            .reverse();
        reordered["claim"]["subject"]["left"]["states"]
            .as_array_mut()
            .expect("states must be an array")
            .reverse();
        reordered["claim"]["subject"]["right"]["actions"]
            .as_array_mut()
            .expect("actions must be an array")
            .reverse();
        reordered["claim"]["subject"]["right"]["states"]
            .as_array_mut()
            .expect("states must be an array")
            .reverse();
        reordered["claim"]["subject"]["context"]["states"]
            .as_array_mut()
            .expect("states must be an array")
            .reverse();
        reordered["claim"]["relation"]
            .as_array_mut()
            .expect("relation must be an array")
            .reverse();
        let result = check(&reordered);
        assert!(result.accepted);
        assert_eq!(result.subject_sha256, expected_hash);
    }

    #[test]
    fn relation_rejects_changed_subject_and_broken_local_laws() {
        let original = relation_certificate();

        let mut malformed_hash = original.clone();
        malformed_hash["claim"]["subject_sha256"] = json!(
            malformed_hash["claim"]["subject_sha256"]
                .as_str()
                .expect("hash must be a string")
                .to_ascii_uppercase()
        );
        assert_eq!(
            rejection_code(&check(&malformed_hash)),
            Some("subject_hash_invalid_format")
        );

        let mut changed = original.clone();
        changed["claim"]["subject"]["right"]["states"][0]["observation"] = json!("changed");
        assert_eq!(
            rejection_code(&check(&changed)),
            Some("subject_hash_mismatch")
        );

        rebind(&mut changed);
        assert_eq!(
            rejection_code(&check(&changed)),
            Some("observation_mismatch")
        );

        let mut deep_observation = relation_certificate();
        deep_observation["claim"]["subject"]["right"]["states"][1]["observation"] =
            json!("changed");
        rebind(&mut deep_observation);
        let result = check(&deep_observation);
        assert_eq!(rejection_code(&result), Some("observation_mismatch"));
        assert_eq!(
            result
                .rejection
                .expect("mismatch must have a rejection")
                .details["access_word"],
            json!(["a"])
        );

        let mut emission = original;
        emission["claim"]["subject"]["right"]["states"][0]["transitions"]["b"]["emission"] =
            json!("changed");
        rebind(&mut emission);
        assert_eq!(rejection_code(&check(&emission)), Some("emission_mismatch"));
    }

    #[test]
    fn relation_rejects_missing_duplicate_unknown_and_unreachable_tuples() {
        let value = relation_certificate();

        let mut missing = value.clone();
        missing["claim"]["relation"]
            .as_array_mut()
            .expect("relation must be an array")
            .pop();
        assert_eq!(
            rejection_code(&check(&missing)),
            Some("missing_closure_tuple")
        );

        let mut missing_initial = value.clone();
        missing_initial["claim"]["relation"]
            .as_array_mut()
            .expect("relation must be an array")
            .remove(0);
        assert_eq!(
            rejection_code(&check(&missing_initial)),
            Some("missing_initial_tuple")
        );

        let mut duplicate = value.clone();
        let tuple = duplicate["claim"]["relation"][0].clone();
        duplicate["claim"]["relation"]
            .as_array_mut()
            .expect("relation must be an array")
            .push(tuple);
        assert_eq!(
            rejection_code(&check(&duplicate)),
            Some("duplicate_relation_tuple")
        );

        let mut unknown = value.clone();
        unknown["claim"]["relation"][0]["left"] = json!("ghost");
        assert_eq!(
            rejection_code(&check(&unknown)),
            Some("unknown_relation_state")
        );

        let mut wrong_context_shape = value.clone();
        wrong_context_shape["claim"]["relation"][0]["context"] = Value::Null;
        assert_eq!(
            rejection_code(&check(&wrong_context_shape)),
            Some("tuple_context_shape_mismatch")
        );

        let mut unreachable = value;
        unreachable["claim"]["relation"]
            .as_array_mut()
            .expect("relation must be an array")
            .push(json!({"left": "l0", "right": "r1", "context": "c0"}));
        assert_eq!(
            rejection_code(&check(&unreachable)),
            Some("unreachable_relation_tuple")
        );
    }

    #[test]
    fn witness_replay_accepts_exact_mismatch_and_rejects_tampering() {
        let value = witness_certificate();
        assert!(check(&value).accepted);

        let mut initial_observation = value.clone();
        initial_observation["claim"]["subject"]["right"]["states"][0]["observation"] =
            json!("different");
        initial_observation["claim"]["word"] = json!([]);
        initial_observation["claim"]["mismatch"] = json!({
            "kind": "state_observation",
            "left": "same",
            "right": "different"
        });
        rebind(&mut initial_observation);
        assert!(check(&initial_observation).accepted);

        let mut mismatch = value.clone();
        mismatch["claim"]["mismatch"]["right"] = json!("forged");
        assert_eq!(
            rejection_code(&check(&mismatch)),
            Some("claimed_mismatch_values_mismatch")
        );

        let mut padded = value.clone();
        padded["claim"]["word"] = json!(["b", "a"]);
        assert_eq!(
            rejection_code(&check(&padded)),
            Some("earlier_emission_mismatch")
        );

        let mut nondistinguishing = value;
        nondistinguishing["claim"]["word"] = json!(["a"]);
        assert_eq!(
            rejection_code(&check(&nondistinguishing)),
            Some("word_not_distinguishing")
        );

        let mut disabled = witness_certificate();
        disabled["claim"]["subject"]["context"] = json!({
            "initial": "only-a",
            "states": [{"id": "only-a", "transitions": {"a": "only-a"}}]
        });
        rebind(&mut disabled);
        assert_eq!(
            rejection_code(&check(&disabled)),
            Some("witness_action_disabled")
        );
        disabled["claim"]["word"] = json!(["b", "ghost"]);
        assert_eq!(
            rejection_code(&check(&disabled)),
            Some("witness_action_disabled")
        );
    }

    #[test]
    fn trace_replay_checks_every_recorded_coordinate() {
        let value = trace_certificate();
        assert!(check(&value).accepted);

        let mut zero_step = value.clone();
        zero_step["claim"]["steps"] = json!([]);
        assert!(check(&zero_step).accepted);

        let mut bad_initial = value.clone();
        bad_initial["claim"]["initial"]["state"] = json!("forged");
        assert_eq!(
            rejection_code(&check(&bad_initial)),
            Some("trace_initial_state_mismatch")
        );

        let cases = [
            ("emission", "trace_emission_mismatch"),
            ("state", "trace_successor_mismatch"),
            ("context", "trace_context_successor_mismatch"),
            ("observation", "trace_observation_mismatch"),
        ];
        for (coordinate, code) in cases {
            let mut tampered = value.clone();
            if coordinate == "emission" {
                tampered["claim"]["steps"][0]["emission"] = json!("forged");
            } else {
                tampered["claim"]["steps"][0]["to"][coordinate] = json!("forged");
            }
            assert_eq!(rejection_code(&check(&tampered)), Some(code));
        }

        let mut disabled = value;
        disabled["claim"]["steps"][0]["action"] = json!("reset");
        assert_eq!(
            rejection_code(&check(&disabled)),
            Some("trace_action_disabled")
        );
    }

    #[test]
    fn provenance_mismatch_is_orthogonal_to_semantic_acceptance() {
        let mut value = witness_certificate();
        let actual = json!({
            "producer": "kerf",
            "operation": "distinguish",
            "input_sha256": "input-hash",
            "tool_version": "0.1.0",
            "limits": null,
            "revision": "old"
        });
        value["claim"]["provenance"] = actual.clone();
        value["expected_provenance"] = actual;
        value["expected_provenance"]["revision"] = json!("new");
        let result = check(&value);
        assert!(result.accepted);
        assert_eq!(result.provenance.status, ProvenanceStatus::Mismatch);
        assert_eq!(result.provenance.mismatch_fields, ["revision"]);
    }

    #[test]
    fn invalid_semantic_subject_is_an_input_error() {
        let mut value = witness_certificate();
        value["claim"]["subject"]["left"]["actions"] = json!([]);
        let error = verify(&parse(&value)).expect_err("invalid subject must be an input error");
        assert_eq!(error.code, "invalid_input");
    }
}
