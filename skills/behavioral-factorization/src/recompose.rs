use crate::factor::{BoundaryRepresentation, BoundaryTransition, REPRESENTATION_SCHEMA};
use crate::lts::{
    ContextMonitor, DistinguishingWitness, IndexedContext, IndexedMachine, Limits, Machine,
    Mismatch, index_context, index_machine, require_schema,
};
use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const RECOMPOSE_SCHEMA: &str = "kerf.recompose.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecomposeInput {
    pub schema_version: String,
    pub factors: Vec<NamedFactor>,
    pub observation_readout: Vec<ReadoutCase>,
    #[serde(
        default,
        deserialize_with = "crate::report::deserialize_string_map_no_duplicates"
    )]
    pub emission_readouts: BTreeMap<String, Vec<ReadoutCase>>,
    pub reference: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedFactor {
    pub id: String,
    pub representation: BoundaryRepresentation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadoutCase {
    #[serde(deserialize_with = "crate::report::deserialize_string_map_no_duplicates")]
    pub inputs: BTreeMap<String, String>,
    pub output: String,
}

#[derive(Debug, Serialize)]
pub struct RecomposeResult {
    pub algorithm: &'static str,
    pub verdict: &'static str,
    pub factor_count: usize,
    pub explored_composed_states: usize,
    pub explored_pairs: usize,
    pub witness: Option<DistinguishingWitness>,
    pub witness_minimality: Option<&'static str>,
    pub unproved: Option<UnprovedWitness>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct UnprovedWitness {
    pub word: Vec<String>,
    pub length_actions: usize,
    pub action: String,
    pub factors: Vec<UnprovedFactor>,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct UnprovedFactor {
    pub factor_id: String,
    pub factor_state: String,
    pub certified_context_state: String,
    pub outside_certified_context: bool,
    pub unconstrained_transition: bool,
}

#[derive(Debug)]
struct IndexedFactor {
    id: String,
    initial: usize,
    states: Vec<IndexedFactorState>,
    certified_context: IndexedContext,
}

#[derive(Debug)]
struct IndexedFactorState {
    id: String,
    observation: String,
    transitions: Vec<IndexedFactorTransition>,
}

#[derive(Debug)]
enum IndexedFactorTransition {
    Constrained { to: usize, emission: String },
    Unconstrained,
}

#[derive(Debug)]
struct IndexedReadouts {
    observation: BTreeMap<Vec<String>, String>,
    emissions: Vec<BTreeMap<Vec<String>, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComposedKey {
    factor_states: Vec<usize>,
    factor_contexts: Vec<usize>,
    context: usize,
}

#[derive(Debug)]
struct ComposedNode {
    key: ComposedKey,
    observation: String,
    parent: Option<(usize, usize)>,
}

#[derive(Debug)]
enum ComposedTransition {
    Proved {
        to: usize,
        emission: String,
    },
    Truncated {
        emission: String,
        successor_observation: String,
    },
    Unproved,
}

#[derive(Debug, Clone)]
struct UnprovedFactorIndex {
    factor_index: usize,
    outside_certified_context: bool,
    unconstrained_transition: bool,
}

#[derive(Debug, Clone)]
struct UnprovedCandidate {
    composed_node: usize,
    action: usize,
    factors: Vec<UnprovedFactorIndex>,
}

#[derive(Debug)]
struct ComposedGraph {
    nodes: Vec<ComposedNode>,
    transitions: Vec<Vec<Option<ComposedTransition>>>,
    first_unproved: Option<UnprovedCandidate>,
    product_truncated: bool,
}

#[derive(Debug)]
struct PairNode {
    composed: usize,
    reference: usize,
    parent: Option<(usize, usize)>,
    depth: usize,
}

/// Recompose supplied finite factors and compare them with a reference model.
///
/// # Errors
///
/// Returns [`ToolError`] for an invalid schema, factor, readout, reference, or
/// bound, and when a configured exploration bound prevents a decision.
pub fn check(input: &RecomposeInput) -> Result<RecomposeResult, ToolError> {
    require_schema(&input.schema_version, RECOMPOSE_SCHEMA)?;
    validate_limits(input.limits)?;
    let reference = index_machine(&input.reference)?;
    let context = index_context(input.context.as_ref(), &reference.actions)?;
    let factors = index_factors(&input.factors, &reference.actions)?;
    let readouts = index_readouts(input, &factors, &reference.actions)?;
    let graph = build_composed(
        &factors,
        &context,
        &reference.actions,
        &readouts,
        input.limits.max_product_states,
    )?;
    compare_with_reference(&factors, &graph, &reference, input.limits)
}

fn validate_limits(limits: Limits) -> Result<(), ToolError> {
    if limits.max_product_states == 0 || limits.max_pairs == 0 || limits.max_witness_length == 0 {
        return Err(ToolError::invalid(
            "max_product_states, max_pairs, and max_witness_length must all be positive",
        ));
    }
    Ok(())
}

fn index_factors(
    raw_factors: &[NamedFactor],
    actions: &[String],
) -> Result<Vec<IndexedFactor>, ToolError> {
    if raw_factors.is_empty() {
        return Err(ToolError::invalid("factors must not be empty"));
    }
    let mut ordered: Vec<&NamedFactor> = raw_factors.iter().collect();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    if ordered.iter().any(|factor| factor.id.is_empty()) {
        return Err(ToolError::invalid("factor IDs must not be empty"));
    }
    if ordered.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(ToolError::invalid("factor IDs must be unique"));
    }
    ordered
        .into_iter()
        .map(|factor| index_factor(factor, actions))
        .collect()
}

fn index_factor(factor: &NamedFactor, actions: &[String]) -> Result<IndexedFactor, ToolError> {
    require_schema(&factor.representation.schema_version, REPRESENTATION_SCHEMA)?;
    let mut factor_actions = factor.representation.actions.clone();
    factor_actions.sort();
    if factor_actions.iter().any(String::is_empty) {
        return Err(ToolError::invalid(format!(
            "factor {:?} has an empty action ID",
            factor.id
        )));
    }
    if factor_actions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ToolError::invalid(format!(
            "factor {:?} action IDs must be unique",
            factor.id
        )));
    }
    if factor_actions != actions {
        return Err(ToolError::invalid(format!(
            "factor {:?} must declare the same canonically sorted action set as the reference",
            factor.id
        )));
    }
    if factor.representation.states.is_empty() {
        return Err(ToolError::invalid(format!(
            "factor {:?} states must not be empty",
            factor.id
        )));
    }

    let mut state_ids = BTreeMap::new();
    for (index, state) in factor.representation.states.iter().enumerate() {
        if state.id.is_empty() {
            return Err(ToolError::invalid(format!(
                "factor {:?} has an empty state ID",
                factor.id
            )));
        }
        if state_ids.insert(state.id.clone(), index).is_some() {
            return Err(ToolError::invalid(format!(
                "factor {:?} has duplicate state ID {:?}",
                factor.id, state.id
            )));
        }
    }
    let initial = state_ids
        .get(&factor.representation.initial)
        .copied()
        .ok_or_else(|| {
            ToolError::invalid(format!(
                "factor {:?} initial state {:?} does not exist",
                factor.id, factor.representation.initial
            ))
        })?;
    let expected_actions: BTreeSet<&str> = actions.iter().map(String::as_str).collect();
    let mut states = Vec::with_capacity(factor.representation.states.len());
    for state in &factor.representation.states {
        let actual_actions: BTreeSet<&str> = state.transitions.keys().map(String::as_str).collect();
        if actual_actions != expected_actions {
            return Err(ToolError::invalid(format!(
                "factor {:?}, state {:?} must define exactly one coordinate for every action",
                factor.id, state.id
            )));
        }
        let mut transitions = Vec::with_capacity(actions.len());
        for action in actions {
            let transition = &state.transitions[action];
            transitions.push(match transition {
                BoundaryTransition::Constrained { to, emission } => {
                    let to = state_ids.get(to).copied().ok_or_else(|| {
                        ToolError::invalid(format!(
                            "factor {:?}, state {:?}, action {:?} targets unknown state {:?}",
                            factor.id, state.id, action, to
                        ))
                    })?;
                    IndexedFactorTransition::Constrained {
                        to,
                        emission: emission.clone(),
                    }
                }
                BoundaryTransition::Unconstrained => IndexedFactorTransition::Unconstrained,
            });
        }
        states.push(IndexedFactorState {
            id: state.id.clone(),
            observation: state.observation.clone(),
            transitions,
        });
    }
    let certified_context = index_context(
        factor.representation.certified_context.as_monitor(),
        actions,
    )?;
    Ok(IndexedFactor {
        id: factor.id.clone(),
        initial,
        states,
        certified_context,
    })
}

fn index_readouts(
    input: &RecomposeInput,
    factors: &[IndexedFactor],
    actions: &[String],
) -> Result<IndexedReadouts, ToolError> {
    let factor_ids: Vec<String> = factors.iter().map(|factor| factor.id.clone()).collect();
    let known_actions: BTreeSet<&str> = actions.iter().map(String::as_str).collect();
    if let Some(action) = input
        .emission_readouts
        .keys()
        .find(|action| !known_actions.contains(action.as_str()))
    {
        return Err(ToolError::invalid(format!(
            "emission_readouts mentions unknown action {action:?}"
        )));
    }
    let observation = index_readout_cases(
        &input.observation_readout,
        &factor_ids,
        "observation_readout",
    )?;
    let emissions = actions
        .iter()
        .map(|action| {
            index_readout_cases(
                input
                    .emission_readouts
                    .get(action)
                    .map_or(&[], Vec::as_slice),
                &factor_ids,
                &format!("emission_readouts[{action:?}]"),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IndexedReadouts {
        observation,
        emissions,
    })
}

fn index_readout_cases(
    cases: &[ReadoutCase],
    factor_ids: &[String],
    label: &str,
) -> Result<BTreeMap<Vec<String>, String>, ToolError> {
    let expected: BTreeSet<&str> = factor_ids.iter().map(String::as_str).collect();
    let mut indexed = BTreeMap::new();
    for case in cases {
        let actual: BTreeSet<&str> = case.inputs.keys().map(String::as_str).collect();
        if actual != expected {
            return Err(ToolError::invalid(format!(
                "{label} case inputs must name exactly every factor ID"
            )));
        }
        let signature: Vec<String> = factor_ids
            .iter()
            .map(|factor_id| case.inputs[factor_id].clone())
            .collect();
        if indexed.insert(signature, case.output.clone()).is_some() {
            return Err(ToolError::invalid(format!(
                "{label} contains a duplicate input signature"
            )));
        }
    }
    Ok(indexed)
}

#[allow(clippy::too_many_lines)]
fn build_composed(
    factors: &[IndexedFactor],
    context: &IndexedContext,
    actions: &[String],
    readouts: &IndexedReadouts,
    max_product_states: usize,
) -> Result<ComposedGraph, ToolError> {
    let start = ComposedKey {
        factor_states: factors.iter().map(|factor| factor.initial).collect(),
        factor_contexts: factors
            .iter()
            .map(|factor| factor.certified_context.initial)
            .collect(),
        context: context.initial,
    };
    let initial_observation = read_observation(&start, factors, readouts)?;
    let mut nodes = vec![ComposedNode {
        key: start.clone(),
        observation: initial_observation,
        parent: None,
    }];
    let mut indexes = BTreeMap::from([(start, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut transitions = Vec::new();
    let mut first_unproved = None;
    let mut product_truncated = false;

    while let Some(node_index) = queue.pop_front() {
        let key = nodes[node_index].key.clone();
        let mut row: Vec<Option<ComposedTransition>> = (0..actions.len()).map(|_| None).collect();
        for (action_index, context_target) in
            context.states[key.context].transitions.iter().enumerate()
        {
            let Some(context_target) = context_target else {
                continue;
            };
            let mut unproved_factors = Vec::new();
            let mut successor_states = Vec::with_capacity(factors.len());
            let mut successor_contexts = Vec::with_capacity(factors.len());
            let mut emissions = Vec::with_capacity(factors.len());
            for (factor_index, factor) in factors.iter().enumerate() {
                let state_index = key.factor_states[factor_index];
                let certified_context_target = factor.certified_context.states
                    [key.factor_contexts[factor_index]]
                    .transitions[action_index];
                let transition = &factor.states[state_index].transitions[action_index];
                let unconstrained_transition =
                    matches!(transition, IndexedFactorTransition::Unconstrained);
                let outside_certified_context = certified_context_target.is_none();
                if outside_certified_context || unconstrained_transition {
                    unproved_factors.push(UnprovedFactorIndex {
                        factor_index,
                        outside_certified_context,
                        unconstrained_transition,
                    });
                    continue;
                }
                let IndexedFactorTransition::Constrained { to, emission } = transition else {
                    return Err(ToolError::internal(
                        "a certified recomposition transition lost its constrained coordinate",
                    ));
                };
                let Some(certified_context_target) = certified_context_target else {
                    return Err(ToolError::internal(
                        "a certified recomposition transition lost its context target",
                    ));
                };
                successor_states.push(*to);
                successor_contexts.push(certified_context_target);
                emissions.push(emission.clone());
            }
            if !unproved_factors.is_empty() {
                first_unproved.get_or_insert_with(|| UnprovedCandidate {
                    composed_node: node_index,
                    action: action_index,
                    factors: unproved_factors.clone(),
                });
                row[action_index] = Some(ComposedTransition::Unproved);
                continue;
            }
            if successor_states.len() != factors.len()
                || successor_contexts.len() != factors.len()
                || emissions.len() != factors.len()
            {
                return Err(ToolError::internal(
                    "proved recomposition transition has incomplete factor coordinates",
                ));
            }
            let emission = readouts.emissions[action_index]
                .get(&emissions)
                .cloned()
                .ok_or_else(|| {
                    ToolError::invalid(format!(
                        "missing reachable emission readout for action {:?} and inputs {:?}",
                        actions[action_index], emissions
                    ))
                })?;
            let successor = ComposedKey {
                factor_states: successor_states,
                factor_contexts: successor_contexts,
                context: *context_target,
            };
            let successor_index = if let Some(existing) = indexes.get(&successor) {
                *existing
            } else {
                let observation = read_observation(&successor, factors, readouts)?;
                if nodes.len() >= max_product_states {
                    product_truncated = true;
                    row[action_index] = Some(ComposedTransition::Truncated {
                        emission,
                        successor_observation: observation,
                    });
                    continue;
                }
                let index = nodes.len();
                nodes.push(ComposedNode {
                    key: successor.clone(),
                    observation,
                    parent: Some((node_index, action_index)),
                });
                indexes.insert(successor, index);
                queue.push_back(index);
                index
            };
            row[action_index] = Some(ComposedTransition::Proved {
                to: successor_index,
                emission,
            });
        }
        transitions.push(row);
    }

    Ok(ComposedGraph {
        nodes,
        transitions,
        first_unproved,
        product_truncated,
    })
}

fn read_observation(
    key: &ComposedKey,
    factors: &[IndexedFactor],
    readouts: &IndexedReadouts,
) -> Result<String, ToolError> {
    let signature: Vec<String> = factors
        .iter()
        .enumerate()
        .map(|(index, factor)| factor.states[key.factor_states[index]].observation.clone())
        .collect();
    readouts
        .observation
        .get(&signature)
        .cloned()
        .ok_or_else(|| {
            ToolError::invalid(format!(
                "missing reachable observation readout for inputs {signature:?}"
            ))
        })
}

#[allow(clippy::too_many_lines)]
fn compare_with_reference(
    factors: &[IndexedFactor],
    graph: &ComposedGraph,
    reference: &IndexedMachine,
    limits: Limits,
) -> Result<RecomposeResult, ToolError> {
    let start = (0_usize, reference.initial);
    let mut nodes = vec![PairNode {
        composed: start.0,
        reference: start.1,
        parent: None,
        depth: 0,
    }];
    let mut seen = BTreeMap::from([(start, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut truncated_by_length = false;

    if graph.nodes[0].observation != reference.states[reference.initial].observation {
        return Ok(inequivalent_result(
            factors,
            graph,
            &nodes,
            &reference.actions,
            0,
            None,
            "state_observation",
            graph.nodes[0].observation.clone(),
            reference.states[reference.initial].observation.clone(),
        ));
    }

    while let Some(first_index) = queue.front().copied() {
        let layer_depth = nodes[first_index].depth;
        let mut pending = Vec::new();
        let mut pending_seen = BTreeSet::new();

        while queue
            .front()
            .is_some_and(|index| nodes[*index].depth == layer_depth)
        {
            let pair_index = queue
                .pop_front()
                .ok_or_else(|| ToolError::internal("comparison layer lost its queued pair"))?;
            let composed_state = nodes[pair_index].composed;
            let reference_state = nodes[pair_index].reference;
            if layer_depth >= limits.max_witness_length {
                if graph.transitions[composed_state]
                    .iter()
                    .any(Option::is_some)
                {
                    truncated_by_length = true;
                }
                continue;
            }
            for (action_index, transition) in graph.transitions[composed_state].iter().enumerate() {
                let Some(transition) = transition else {
                    continue;
                };
                let (successor, emission, successor_observation) = match transition {
                    ComposedTransition::Proved { to, emission } => {
                        (Some(*to), emission, graph.nodes[*to].observation.as_str())
                    }
                    ComposedTransition::Truncated {
                        emission,
                        successor_observation,
                    } => (None, emission, successor_observation.as_str()),
                    ComposedTransition::Unproved => continue,
                };
                let reference_transition =
                    &reference.states[reference_state].transitions[action_index];
                if emission != &reference_transition.emission {
                    return Ok(inequivalent_result(
                        factors,
                        graph,
                        &nodes,
                        &reference.actions,
                        pair_index,
                        Some(action_index),
                        "transition_emission",
                        emission.clone(),
                        reference_transition.emission.clone(),
                    ));
                }
                if successor_observation != reference.states[reference_transition.to].observation {
                    return Ok(inequivalent_result(
                        factors,
                        graph,
                        &nodes,
                        &reference.actions,
                        pair_index,
                        Some(action_index),
                        "state_observation",
                        successor_observation.to_owned(),
                        reference.states[reference_transition.to]
                            .observation
                            .clone(),
                    ));
                }
                if let Some(successor) = successor {
                    let successor = (successor, reference_transition.to);
                    if !seen.contains_key(&successor) && pending_seen.insert(successor) {
                        pending.push((successor, pair_index, action_index));
                    }
                }
            }
        }

        for (successor, parent, action) in pending {
            if nodes.len() >= limits.max_pairs {
                return Err(ToolError::resource_limit(format!(
                    "recomposition comparison reached max_pairs={} before a verdict",
                    limits.max_pairs
                )));
            }
            let successor_index = nodes.len();
            nodes.push(PairNode {
                composed: successor.0,
                reference: successor.1,
                parent: Some((parent, action)),
                depth: layer_depth + 1,
            });
            seen.insert(successor, successor_index);
            queue.push_back(successor_index);
        }
    }

    if truncated_by_length {
        return Err(ToolError::resource_limit(format!(
            "recomposition witness search reached max_witness_length={} before a verdict",
            limits.max_witness_length
        )));
    }
    if graph.product_truncated {
        return Err(ToolError::resource_limit(format!(
            "reachable composition reached max_product_states={} before a verdict",
            limits.max_product_states
        )));
    }
    if let Some(candidate) = graph.first_unproved.as_ref() {
        return Ok(unproved_result(
            factors,
            graph,
            &reference.actions,
            candidate,
            nodes.len(),
        ));
    }
    Ok(RecomposeResult {
        algorithm: "reachable_synchronous_recomposition_v1",
        verdict: "equivalent",
        factor_count: factors.len(),
        explored_composed_states: graph.nodes.len(),
        explored_pairs: nodes.len(),
        witness: None,
        witness_minimality: None,
        unproved: None,
        nonclaims: recomposition_nonclaims(),
    })
}

#[allow(clippy::too_many_arguments)]
fn inequivalent_result(
    factors: &[IndexedFactor],
    graph: &ComposedGraph,
    nodes: &[PairNode],
    actions: &[String],
    pair_node: usize,
    final_action: Option<usize>,
    kind: &'static str,
    left: String,
    right: String,
) -> RecomposeResult {
    let word = pair_access_word(nodes, actions, pair_node, final_action);
    let unproved = graph
        .first_unproved
        .as_ref()
        .map(|candidate| unproved_witness(factors, graph, actions, candidate));
    let witness_minimality = if graph.product_truncated {
        "proved_region_only"
    } else {
        match &unproved {
            Some(candidate) if shortlex_key(&candidate.word) < shortlex_key(&word) => {
                "proved_region_only"
            }
            _ => "completion_robust",
        }
    };
    RecomposeResult {
        algorithm: "reachable_synchronous_recomposition_v1",
        verdict: "inequivalent",
        factor_count: factors.len(),
        explored_composed_states: graph.nodes.len(),
        explored_pairs: nodes.len(),
        witness: Some(DistinguishingWitness {
            length_actions: word.len(),
            word,
            mismatch: Mismatch { kind, left, right },
        }),
        witness_minimality: Some(witness_minimality),
        unproved,
        nonclaims: recomposition_nonclaims(),
    }
}

fn unproved_result(
    factors: &[IndexedFactor],
    graph: &ComposedGraph,
    actions: &[String],
    candidate: &UnprovedCandidate,
    explored_pairs: usize,
) -> RecomposeResult {
    RecomposeResult {
        algorithm: "reachable_synchronous_recomposition_v1",
        verdict: "unproved",
        factor_count: factors.len(),
        explored_composed_states: graph.nodes.len(),
        explored_pairs,
        witness: None,
        witness_minimality: None,
        unproved: Some(unproved_witness(factors, graph, actions, candidate)),
        nonclaims: recomposition_nonclaims(),
    }
}

fn unproved_witness(
    factors: &[IndexedFactor],
    graph: &ComposedGraph,
    actions: &[String],
    candidate: &UnprovedCandidate,
) -> UnprovedWitness {
    let word = composed_access_word(
        &graph.nodes,
        actions,
        candidate.composed_node,
        Some(candidate.action),
    );
    let key = &graph.nodes[candidate.composed_node].key;
    let unproved_factors = candidate
        .factors
        .iter()
        .map(|issue| UnprovedFactor {
            factor_id: factors[issue.factor_index].id.clone(),
            factor_state: factors[issue.factor_index].states[key.factor_states[issue.factor_index]]
                .id
                .clone(),
            certified_context_state: factors[issue.factor_index].certified_context.states
                [key.factor_contexts[issue.factor_index]]
                .id
                .clone(),
            outside_certified_context: issue.outside_certified_context,
            unconstrained_transition: issue.unconstrained_transition,
        })
        .collect();
    UnprovedWitness {
        length_actions: word.len(),
        word,
        action: actions[candidate.action].clone(),
        factors: unproved_factors,
        reason: "factor_transition_not_certified",
    }
}

fn pair_access_word(
    nodes: &[PairNode],
    actions: &[String],
    node_index: usize,
    final_action: Option<usize>,
) -> Vec<String> {
    let mut action_indexes = Vec::new();
    let mut cursor = node_index;
    while let Some((parent, action)) = nodes[cursor].parent {
        action_indexes.push(action);
        cursor = parent;
    }
    action_indexes.reverse();
    if let Some(action) = final_action {
        action_indexes.push(action);
    }
    action_indexes
        .into_iter()
        .map(|action| actions[action].clone())
        .collect()
}

fn composed_access_word(
    nodes: &[ComposedNode],
    actions: &[String],
    node_index: usize,
    final_action: Option<usize>,
) -> Vec<String> {
    let mut action_indexes = Vec::new();
    let mut cursor = node_index;
    while let Some((parent, action)) = nodes[cursor].parent {
        action_indexes.push(action);
        cursor = parent;
    }
    action_indexes.reverse();
    if let Some(action) = final_action {
        action_indexes.push(action);
    }
    action_indexes
        .into_iter()
        .map(|action| actions[action].clone())
        .collect()
}

fn shortlex_key(word: &[String]) -> (usize, &[String]) {
    (word.len(), word)
}

fn recomposition_nonclaims() -> Vec<&'static str> {
    vec![
        "The verdict is exact only for the supplied finite factors, table readouts, reference, retained context, and bounds.",
        "An unproved witness identifies a reachable factor coordinate outside its certified context or left unconstrained; it is not evidence of inequivalence.",
        "An inequivalence witness is completion-robust unless witness_minimality is proved_region_only; an earlier unproved word or unexamined product frontier can then hide an earlier mismatch but cannot invalidate the reported mismatch.",
        "The operation does not infer factors or readouts and does not prove equivalence of source code abstracted into the models.",
        "Typed ports, hidden actions, feedback, delay, hierarchy, asynchrony, liveness, timing, causality, and unmodeled effects are outside the result.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::{FactorInput, check as check_factor};
    use serde_json::{Value, json};

    fn parse(value: Value) -> RecomposeInput {
        serde_json::from_value(value).expect("test recomposition input must deserialize")
    }

    fn one_factor_round_trip() -> RecomposeInput {
        let factor_input: FactorInput = serde_json::from_str(include_str!(
            "../examples/context-side-information.factor.json"
        ))
        .expect("factor example must deserialize");
        let factor_result = check_factor(&factor_input).expect("factor example must check");
        let representation = factor_result
            .representation
            .expect("lawful factor example must emit a representation");
        RecomposeInput {
            schema_version: RECOMPOSE_SCHEMA.to_owned(),
            factors: vec![NamedFactor {
                id: "boundary".to_owned(),
                representation,
            }],
            observation_readout: vec![ReadoutCase {
                inputs: BTreeMap::from([("boundary".to_owned(), "same".to_owned())]),
                output: "same".to_owned(),
            }],
            emission_readouts: BTreeMap::from([
                (
                    "a".to_owned(),
                    vec![ReadoutCase {
                        inputs: BTreeMap::from([("boundary".to_owned(), "step".to_owned())]),
                        output: "step".to_owned(),
                    }],
                ),
                (
                    "b".to_owned(),
                    vec![ReadoutCase {
                        inputs: BTreeMap::from([("boundary".to_owned(), "step".to_owned())]),
                        output: "step".to_owned(),
                    }],
                ),
            ]),
            reference: factor_input.machine,
            context: factor_input.context,
            limits: Limits::default(),
        }
    }

    fn two_factor_input() -> RecomposeInput {
        parse(json!({
            "schema_version": RECOMPOSE_SCHEMA,
            "factors": [
                {
                    "id": "x",
                    "representation": {
                        "schema_version": REPRESENTATION_SCHEMA,
                        "certified_context": null,
                        "initial": "x0",
                        "actions": ["tick"],
                        "states": [
                            {"id":"x0","observation":"off","transitions":{"tick":{"status":"constrained","to":"x1","emission":"rise-x"}}},
                            {"id":"x1","observation":"on","transitions":{"tick":{"status":"constrained","to":"x0","emission":"fall-x"}}}
                        ]
                    }
                },
                {
                    "id": "y",
                    "representation": {
                        "schema_version": REPRESENTATION_SCHEMA,
                        "certified_context": null,
                        "initial": "y0",
                        "actions": ["tick"],
                        "states": [
                            {"id":"y0","observation":"low","transitions":{"tick":{"status":"constrained","to":"y1","emission":"rise-y"}}},
                            {"id":"y1","observation":"high","transitions":{"tick":{"status":"constrained","to":"y0","emission":"fall-y"}}}
                        ]
                    }
                }
            ],
            "observation_readout": [
                {"inputs":{"x":"off","y":"low"},"output":"0"},
                {"inputs":{"x":"on","y":"high"},"output":"1"}
            ],
            "emission_readouts": {
                "tick": [
                    {"inputs":{"x":"rise-x","y":"rise-y"},"output":"up"},
                    {"inputs":{"x":"fall-x","y":"fall-y"},"output":"down"}
                ]
            },
            "reference": {
                "initial":"r0",
                "actions":["tick"],
                "states":[
                    {"id":"r0","observation":"0","transitions":{"tick":{"to":"r1","emission":"up"}}},
                    {"id":"r1","observation":"1","transitions":{"tick":{"to":"r0","emission":"down"}}}
                ]
            },
            "limits":{"max_product_states":16,"max_pairs":16,"max_witness_length":16}
        }))
    }

    fn shorter_sibling_input() -> RecomposeInput {
        parse(json!({
            "schema_version": RECOMPOSE_SCHEMA,
            "factors": [{
                "id":"f",
                "representation":{
                    "schema_version":REPRESENTATION_SCHEMA,
                    "certified_context":null,
                    "initial":"z0",
                    "actions":["b","a"],
                    "states":[
                        {"id":"z0","observation":"same","transitions":{
                            "a":{"status":"constrained","to":"z1","emission":"ok"},
                            "b":{"status":"constrained","to":"z0","emission":"wrong-b"}
                        }},
                        {"id":"z1","observation":"same","transitions":{
                            "a":{"status":"constrained","to":"z1","emission":"wrong-aa"},
                            "b":{"status":"constrained","to":"z1","emission":"ok"}
                        }}
                    ]
                }
            }],
            "observation_readout":[{"inputs":{"f":"same"},"output":"same"}],
            "emission_readouts":{
                "a":[
                    {"inputs":{"f":"ok"},"output":"ok"},
                    {"inputs":{"f":"wrong-aa"},"output":"left-aa"}
                ],
                "b":[
                    {"inputs":{"f":"ok"},"output":"ok"},
                    {"inputs":{"f":"wrong-b"},"output":"left-b"}
                ]
            },
            "reference":{
                "initial":"r0",
                "actions":["a","b"],
                "states":[
                    {"id":"r0","observation":"same","transitions":{
                        "a":{"to":"r1","emission":"ok"},
                        "b":{"to":"r0","emission":"right-b"}
                    }},
                    {"id":"r1","observation":"same","transitions":{
                        "a":{"to":"r1","emission":"right-aa"},
                        "b":{"to":"r1","emission":"ok"}
                    }}
                ]
            },
            "limits":{"max_product_states":16,"max_pairs":16,"max_witness_length":16}
        }))
    }

    fn truncated_frontier_input() -> RecomposeInput {
        parse(json!({
            "schema_version": RECOMPOSE_SCHEMA,
            "factors": [{
                "id":"f",
                "representation":{
                    "schema_version":REPRESENTATION_SCHEMA,
                    "certified_context":null,
                    "initial":"z0",
                    "actions":["a","b"],
                    "states":[
                        {"id":"z0","observation":"same","transitions":{
                            "a":{"status":"constrained","to":"z1","emission":"ok"},
                            "b":{"status":"constrained","to":"z1","emission":"ok"}
                        }},
                        {"id":"z1","observation":"same","transitions":{
                            "a":{"status":"constrained","to":"z2","emission":"ok"},
                            "b":{"status":"constrained","to":"z1","emission":"ok"}
                        }},
                        {"id":"z2","observation":"same","transitions":{
                            "a":{"status":"constrained","to":"z2","emission":"boom"},
                            "b":{"status":"constrained","to":"z2","emission":"ok"}
                        }}
                    ]
                }
            }],
            "observation_readout":[{"inputs":{"f":"same"},"output":"same"}],
            "emission_readouts":{
                "a":[
                    {"inputs":{"f":"ok"},"output":"ok"},
                    {"inputs":{"f":"boom"},"output":"left-boom"}
                ],
                "b":[{"inputs":{"f":"ok"},"output":"ok"}]
            },
            "reference":{
                "initial":"r0",
                "actions":["a","b"],
                "states":[
                    {"id":"r0","observation":"same","transitions":{
                        "a":{"to":"ra1","emission":"ok"},
                        "b":{"to":"rb1","emission":"ok"}
                    }},
                    {"id":"ra1","observation":"same","transitions":{
                        "a":{"to":"ra2","emission":"ok"},
                        "b":{"to":"ra1","emission":"ok"}
                    }},
                    {"id":"ra2","observation":"same","transitions":{
                        "a":{"to":"ra2","emission":"right-boom"},
                        "b":{"to":"ra2","emission":"ok"}
                    }},
                    {"id":"rb1","observation":"same","transitions":{
                        "a":{"to":"rb1","emission":"ok"},
                        "b":{"to":"rb2","emission":"ok"}
                    }},
                    {"id":"rb2","observation":"same","transitions":{
                        "a":{"to":"rb2","emission":"ok"},
                        "b":{"to":"rb2","emission":"right-b"}
                    }}
                ]
            },
            "limits":{"max_product_states":2,"max_pairs":32,"max_witness_length":16}
        }))
    }

    #[test]
    fn one_factor_consumes_factor_output_under_its_certified_context() {
        let input = one_factor_round_trip();
        assert_eq!(
            input.factors[0].representation.schema_version,
            REPRESENTATION_SCHEMA
        );
        assert!(
            input.factors[0]
                .representation
                .certified_context
                .as_monitor()
                .is_some()
        );
        let result = check(&input).expect("round trip must check");
        assert_eq!(result.verdict, "equivalent");
        assert_eq!(result.explored_composed_states, 3);
        assert!(result.witness.is_none());
        assert!(result.unproved.is_none());
    }

    #[test]
    fn widened_context_reports_shortest_uncertified_transition() {
        let mut input = one_factor_round_trip();
        input.context = None;
        let result = check(&input).expect("widened context must be analyzed");
        assert_eq!(result.verdict, "unproved");
        let unproved = result
            .unproved
            .expect("unproved result must carry evidence");
        assert_eq!(unproved.word, ["c"]);
        assert_eq!(unproved.action, "c");
        assert_eq!(unproved.factors.len(), 1);
        assert!(unproved.factors[0].outside_certified_context);
        assert!(unproved.factors[0].unconstrained_transition);
    }

    #[test]
    fn certified_context_blocks_stepwise_existential_false_equivalence() {
        let factor_input: FactorInput = serde_json::from_value(json!({
            "schema_version":"kerf.factor.v1",
            "machine":{
                "initial":"s0","actions":["a","b"],"states":[
                    {"id":"s0","observation":"same","transitions":{"a":{"to":"s1","emission":"go"},"b":{"to":"s2","emission":"go"}}},
                    {"id":"s1","observation":"same","transitions":{"a":{"to":"s1","emission":"x"},"b":{"to":"s1","emission":"y-not"}}},
                    {"id":"s2","observation":"same","transitions":{"a":{"to":"s2","emission":"x-not"},"b":{"to":"s2","emission":"y"}}}
                ]
            },
            "context":{"initial":"c0","states":[
                {"id":"c0","transitions":{"a":"ca","b":"cb"}},
                {"id":"ca","transitions":{"a":"ca"}},
                {"id":"cb","transitions":{"b":"cb"}}
            ]},
            "factor_map":[
                {"machine_state":"s0","context_state":"c0","boundary_state":"z0"},
                {"machine_state":"s1","context_state":"ca","boundary_state":"z1"},
                {"machine_state":"s2","context_state":"cb","boundary_state":"z1"}
            ],
            "limits":{"max_product_states":16}
        }))
        .expect("counterexample factor input must deserialize");
        let factor_result = check_factor(&factor_input).expect("factor input must check");
        let representation = factor_result
            .representation
            .expect("counterexample map must be lawful under its source context");
        assert!(representation.states.iter().all(|state| {
            state
                .transitions
                .values()
                .all(|transition| matches!(transition, BoundaryTransition::Constrained { .. }))
        }));
        let input = RecomposeInput {
            schema_version: RECOMPOSE_SCHEMA.to_owned(),
            factors: vec![NamedFactor {
                id: "f".to_owned(),
                representation,
            }],
            observation_readout: vec![ReadoutCase {
                inputs: BTreeMap::from([("f".to_owned(), "same".to_owned())]),
                output: "same".to_owned(),
            }],
            emission_readouts: BTreeMap::from([
                (
                    "a".to_owned(),
                    vec![
                        ReadoutCase {
                            inputs: BTreeMap::from([("f".to_owned(), "go".to_owned())]),
                            output: "go".to_owned(),
                        },
                        ReadoutCase {
                            inputs: BTreeMap::from([("f".to_owned(), "x".to_owned())]),
                            output: "x".to_owned(),
                        },
                    ],
                ),
                (
                    "b".to_owned(),
                    vec![
                        ReadoutCase {
                            inputs: BTreeMap::from([("f".to_owned(), "go".to_owned())]),
                            output: "go".to_owned(),
                        },
                        ReadoutCase {
                            inputs: BTreeMap::from([("f".to_owned(), "y".to_owned())]),
                            output: "y".to_owned(),
                        },
                    ],
                ),
            ]),
            reference: serde_json::from_value(json!({
                "initial":"t0","actions":["a","b"],"states":[
                    {"id":"t0","observation":"same","transitions":{"a":{"to":"t1","emission":"go"},"b":{"to":"t1","emission":"go"}}},
                    {"id":"t1","observation":"same","transitions":{"a":{"to":"t1","emission":"x"},"b":{"to":"t1","emission":"y"}}}
                ]
            }))
            .expect("counterexample reference must deserialize"),
            context: None,
            limits: Limits::default(),
        };
        let result = check(&input).expect("counterexample recomposition must check");
        assert_eq!(result.verdict, "unproved");
        let unproved = result
            .unproved
            .expect("counterexample must expose its hole");
        assert_eq!(unproved.word, ["a", "b"]);
        assert!(unproved.factors[0].outside_certified_context);
        assert!(!unproved.factors[0].unconstrained_transition);
    }

    #[test]
    fn genuine_two_factor_product_ignores_unreachable_cartesian_tuples() {
        let result = check(&two_factor_input()).expect("two-factor input must check");
        assert_eq!(result.verdict, "equivalent");
        assert_eq!(result.factor_count, 2);
        assert_eq!(result.explored_composed_states, 2);
        assert_eq!(result.explored_pairs, 2);
    }

    #[test]
    fn shorter_sibling_mismatch_wins_over_lexically_earlier_deeper_word() {
        let mut input = shorter_sibling_input();
        let result = check(&input).expect("negative input must check");
        assert_eq!(result.verdict, "inequivalent");
        assert_eq!(result.witness.expect("mismatch must exist").word, ["b"]);
        assert_eq!(result.witness_minimality, Some("completion_robust"));

        input.limits.max_pairs = 1;
        let result = check(&input).expect("available sibling mismatch must precede pair bound");
        assert_eq!(result.witness.expect("mismatch must exist").word, ["b"]);

        input.limits.max_pairs = 16;
        input.limits.max_product_states = 1;
        let result = check(&input).expect("available sibling mismatch must precede product bound");
        assert_eq!(result.witness.expect("mismatch must exist").word, ["b"]);
    }

    #[test]
    fn product_frontier_downgrades_shortestness_of_a_known_mismatch() {
        let mut input = truncated_frontier_input();
        let bounded = check(&input).expect("exposed mismatch must precede product exhaustion");
        assert_eq!(bounded.verdict, "inequivalent");
        assert_eq!(
            bounded.witness.expect("bounded mismatch must exist").word,
            ["b", "b", "b"]
        );
        assert_eq!(bounded.witness_minimality, Some("proved_region_only"));

        input.limits.max_product_states = 3;
        let complete = check(&input).expect("complete product must check");
        assert_eq!(complete.verdict, "inequivalent");
        assert_eq!(
            complete.witness.expect("complete mismatch must exist").word,
            ["a", "a", "a"]
        );
        assert_eq!(complete.witness_minimality, Some("completion_robust"));
    }

    #[test]
    fn exposed_hole_is_retained_beyond_the_witness_search_bound() {
        let mut input = truncated_frontier_input();
        input.limits.max_product_states = 3;
        input.limits.max_witness_length = 2;
        input.factors[0].representation.states[2]
            .transitions
            .insert("a".to_owned(), BoundaryTransition::Unconstrained);
        input.reference.states[0]
            .transitions
            .get_mut("b")
            .expect("initial b transition must exist")
            .emission = "right-b".to_owned();

        let result = check(&input).expect("known mismatch must precede witness exhaustion");
        assert_eq!(result.verdict, "inequivalent");
        assert_eq!(result.witness.expect("mismatch must exist").word, ["b"]);
        assert_eq!(result.witness_minimality, Some("completion_robust"));
        assert_eq!(
            result.unproved.expect("known hole must be retained").word,
            ["a", "a", "a"]
        );
    }

    #[test]
    fn initial_observation_mismatch_has_empty_word() {
        let mut input = two_factor_input();
        input.observation_readout[0].output = "wrong".to_owned();
        input.limits.max_product_states = 1;
        let result = check(&input).expect("initial mismatch must check");
        assert_eq!(result.verdict, "inequivalent");
        assert!(result.witness.expect("mismatch must exist").word.is_empty());
    }

    #[test]
    fn missing_and_duplicate_reachable_readouts_are_invalid() {
        let mut missing_observation = two_factor_input();
        missing_observation.observation_readout.pop();
        let error = check(&missing_observation).expect_err("missing observation must fail");
        assert_eq!(error.code, "invalid_input");
        assert!(
            error
                .message
                .contains("missing reachable observation readout")
        );

        let mut missing_emission = two_factor_input();
        missing_emission
            .emission_readouts
            .get_mut("tick")
            .expect("tick readouts must exist")
            .remove(0);
        let error = check(&missing_emission).expect_err("missing emission must fail");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("missing reachable emission readout"));

        let mut duplicate = two_factor_input();
        duplicate
            .observation_readout
            .push(duplicate.observation_readout[0].clone());
        let error = check(&duplicate).expect_err("duplicate signature must fail");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("duplicate input signature"));
    }

    #[test]
    fn invalid_factor_and_reference_coordinates_fail_closed() {
        let mut bad_target = two_factor_input();
        let BoundaryTransition::Constrained { to, .. } =
            bad_target.factors[0].representation.states[0]
                .transitions
                .get_mut("tick")
                .expect("tick coordinate must exist")
        else {
            panic!("tick coordinate must be constrained"); // ubs:ignore — test-only fixture assertion
        };
        *to = "ghost".to_owned();
        assert_eq!(
            check(&bad_target).expect_err("bad target must fail").code,
            "invalid_input"
        );

        let mut bad_alphabet = two_factor_input();
        bad_alphabet.factors[0]
            .representation
            .actions
            .push("extra".to_owned());
        assert_eq!(
            check(&bad_alphabet)
                .expect_err("alphabet mismatch must fail")
                .code,
            "invalid_input"
        );

        let mut bad_reference = two_factor_input();
        bad_reference.reference.initial = "ghost".to_owned();
        assert_eq!(
            check(&bad_reference)
                .expect_err("bad reference must fail")
                .code,
            "invalid_input"
        );

        let mut no_factors = two_factor_input();
        no_factors.factors.clear();
        assert_eq!(
            check(&no_factors)
                .expect_err("empty factors must fail")
                .code,
            "invalid_input"
        );
    }

    #[test]
    fn detached_or_legacy_factor_representations_are_rejected() {
        let raw: Value =
            serde_json::from_str(include_str!("../examples/two-factor.recompose.json"))
                .expect("example must parse");
        let mut missing_context = raw.clone();
        missing_context["factors"][0]["representation"]
            .as_object_mut()
            .expect("representation must be an object")
            .remove("certified_context");
        assert!(
            serde_json::from_value::<RecomposeInput>(missing_context).is_err(),
            "v2 must require an explicit certified_context, including null"
        );

        let mut legacy = raw;
        legacy["factors"][0]["representation"]["schema_version"] =
            json!("kerf.factor-representation.v1");
        let legacy = parse(legacy);
        let error = check(&legacy).expect_err("legacy representation must fail");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("unsupported schema_version"));
    }

    #[test]
    fn duplicate_raw_json_members_cannot_change_semantics_by_order() {
        let raw = include_str!("../examples/two-factor.recompose.json");
        let duplicate_readout = raw.replacen(
            r#""inputs": {"x": "off", "y": "low"}"#,
            r#""inputs": {"x": "off", "x": "on", "y": "low"}"#,
            1,
        );
        let error = serde_json::from_str::<RecomposeInput>(&duplicate_readout)
            .expect_err("duplicate readout member must fail");
        assert!(
            error
                .to_string()
                .contains("duplicate JSON object member \"x\"")
        );

        let duplicate_transition = raw.replacen(
            r#""tick": {"status": "constrained", "to": "x1", "emission": "rise-x"}"#,
            r#""tick": {"status": "constrained", "to": "x1", "emission": "rise-x"}, "tick": {"status": "constrained", "to": "x0", "emission": "forged"}"#,
            1,
        );
        let error = serde_json::from_str::<RecomposeInput>(&duplicate_transition)
            .expect_err("duplicate transition member must fail");
        assert!(
            error
                .to_string()
                .contains("duplicate JSON object member \"tick\"")
        );
    }

    #[test]
    fn declaration_order_does_not_change_the_result() {
        let input = two_factor_input();
        let expected = serde_json::to_value(check(&input).expect("base input must check"))
            .expect("result must serialize");
        let mut reordered = input;
        reordered.factors.reverse();
        for factor in &mut reordered.factors {
            factor.representation.actions.reverse();
            factor.representation.states.reverse();
        }
        reordered.reference.actions.reverse();
        reordered.reference.states.reverse();
        reordered.observation_readout.reverse();
        for cases in reordered.emission_readouts.values_mut() {
            cases.reverse();
        }
        let actual = serde_json::to_value(check(&reordered).expect("reordered input must check"))
            .expect("result must serialize");
        assert_eq!(actual, expected);

        let contextual = one_factor_round_trip();
        let expected =
            serde_json::to_value(check(&contextual).expect("contextual base input must check"))
                .expect("contextual result must serialize");
        let mut reordered = contextual;
        reordered
            .context
            .as_mut()
            .expect("proposed context must exist")
            .states
            .reverse();
        match &mut reordered.factors[0].representation.certified_context {
            crate::factor::CertifiedContext::Monitor(context) => context.states.reverse(),
            crate::factor::CertifiedContext::AllActions(()) => {
                panic!("certified context must exist") // ubs:ignore — test-only fixture assertion
            }
        }
        let actual =
            serde_json::to_value(check(&reordered).expect("reordered contextual input must check"))
                .expect("contextual result must serialize");
        assert_eq!(actual, expected);
    }

    #[test]
    fn zero_bounds_are_invalid_individually() {
        for limits in [
            Limits {
                max_product_states: 0,
                ..Limits::default()
            },
            Limits {
                max_pairs: 0,
                ..Limits::default()
            },
            Limits {
                max_witness_length: 0,
                ..Limits::default()
            },
        ] {
            let mut input = two_factor_input();
            input.limits = limits;
            assert_eq!(
                check(&input).expect_err("zero bound must fail").code,
                "invalid_input"
            );
        }
    }

    #[test]
    fn all_three_exploration_bounds_return_unknown() {
        let mut product = two_factor_input();
        product.limits.max_product_states = 1;
        assert_eq!(
            check(&product)
                .expect_err("product bound must stop exploration")
                .code,
            "resource_limit"
        );

        let mut pairs = two_factor_input();
        pairs.limits.max_pairs = 1;
        assert_eq!(
            check(&pairs)
                .expect_err("pair bound must stop comparison")
                .code,
            "resource_limit"
        );

        let mut witness = two_factor_input();
        witness.limits.max_witness_length = 1;
        assert_eq!(
            check(&witness)
                .expect_err("witness bound must stop comparison")
                .code,
            "resource_limit"
        );
    }

    #[test]
    fn proved_mismatch_is_not_hidden_by_an_earlier_hole() {
        let mut input = shorter_sibling_input();
        let factor = &mut input.factors[0].representation;
        factor.states[0]
            .transitions
            .insert("b".to_owned(), BoundaryTransition::Unconstrained);
        let result = check(&input).expect("partial negative input must check");
        assert_eq!(result.verdict, "inequivalent");
        assert_eq!(
            result.witness.as_ref().expect("mismatch must exist").word,
            ["a", "a"]
        );
        assert_eq!(result.witness_minimality, Some("proved_region_only"));
        assert_eq!(
            result.unproved.expect("earlier hole must be retained").word,
            ["b"]
        );
    }
}
