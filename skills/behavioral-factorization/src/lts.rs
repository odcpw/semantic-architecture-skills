use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const LTS_SCHEMA: &str = "kerf.lts.v1";
pub const COMPARE_SCHEMA: &str = "kerf.compare.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LtsInput {
    pub schema_version: String,
    pub machine: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareInput {
    pub schema_version: String,
    pub left: Machine,
    pub right: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub initial: String,
    pub actions: Vec<String>,
    pub states: Vec<State>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub id: String,
    pub observation: String,
    #[serde(deserialize_with = "crate::report::deserialize_string_map_no_duplicates")]
    pub transitions: BTreeMap<String, Transition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub to: String,
    pub emission: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMonitor {
    pub initial: String,
    pub states: Vec<ContextState>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextState {
    pub id: String,
    #[serde(deserialize_with = "crate::report::deserialize_string_map_no_duplicates")]
    pub transitions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_product_states: usize,
    pub max_pairs: usize,
    pub max_witness_length: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_product_states: 1_000_000,
            max_pairs: 1_000_000,
            max_witness_length: 100_000,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ValidationSummary {
    pub machine_states: usize,
    pub actions: usize,
    pub context_states: usize,
    pub context_present: bool,
}

#[derive(Debug, Serialize)]
pub struct QuotientResult {
    pub algorithm: &'static str,
    pub context_present: bool,
    pub reachable_product_states: usize,
    pub unreachable_machine_states: Vec<String>,
    pub equivalence_class_count: usize,
    pub minimum_product_monitor_state_bits: u32,
    pub classes: Vec<QuotientClass>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct QuotientClass {
    pub id: usize,
    pub representative: ProductStateName,
    pub members: Vec<ProductStateName>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProductStateName {
    pub machine_state: String,
    pub context_state: String,
}

#[derive(Debug, Serialize)]
pub struct ComparisonResult {
    pub algorithm: &'static str,
    pub equivalent: bool,
    pub explored_pairs: usize,
    pub witness: Option<DistinguishingWitness>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct DistinguishingWitness {
    pub word: Vec<String>,
    pub length_actions: usize,
    pub mismatch: Mismatch,
}

#[derive(Debug, Serialize)]
pub struct Mismatch {
    pub kind: &'static str,
    pub left: String,
    pub right: String,
}

#[derive(Debug)]
pub(crate) struct IndexedMachine {
    pub(crate) actions: Vec<String>,
    pub(crate) initial: usize,
    pub(crate) states: Vec<IndexedState>,
}

#[derive(Debug)]
pub(crate) struct IndexedState {
    pub(crate) id: String,
    pub(crate) observation: String,
    pub(crate) transitions: Vec<IndexedTransition>,
}

#[derive(Debug)]
pub(crate) struct IndexedTransition {
    pub(crate) to: usize,
    pub(crate) emission: String,
}

#[derive(Debug)]
pub(crate) struct IndexedContext {
    pub(crate) initial: usize,
    pub(crate) states: Vec<IndexedContextState>,
}

#[derive(Debug)]
pub(crate) struct IndexedContextState {
    pub(crate) id: String,
    pub(crate) transitions: Vec<Option<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProductState {
    pub(crate) machine: usize,
    pub(crate) context: usize,
}

#[derive(Debug)]
pub(crate) struct ProductGraph {
    pub(crate) nodes: Vec<ProductState>,
    pub(crate) transitions: Vec<Vec<Option<(usize, String)>>>,
}

#[derive(Debug)]
pub(crate) struct ProductAccess {
    parents: Vec<Option<(usize, usize)>>,
    depths: Vec<usize>,
}

impl ProductAccess {
    pub(crate) fn access_word(&self, node_index: usize, actions: &[String]) -> Vec<String> {
        let mut action_indexes = Vec::new();
        let mut cursor = node_index;
        while let Some((parent, action)) = self.parents[cursor] {
            action_indexes.push(action);
            cursor = parent;
        }
        action_indexes.reverse();
        action_indexes
            .into_iter()
            .map(|action| actions[action].clone())
            .collect()
    }

    pub(crate) fn depth(&self, node_index: usize) -> usize {
        self.depths[node_index]
    }
}

#[derive(Debug)]
struct PairNode {
    left: usize,
    right: usize,
    context: usize,
    parent: Option<(usize, usize)>,
    depth: usize,
}

/// Validate a finite deterministic machine and optional context monitor.
///
/// # Errors
///
/// Returns [`ToolError`] when the schema, limits, machine, or context is invalid.
pub fn validate(input: &LtsInput) -> Result<ValidationSummary, ToolError> {
    require_schema(&input.schema_version, LTS_SCHEMA)?;
    validate_limits(input.limits)?;
    let machine = index_machine(&input.machine)?;
    let context = index_context(input.context.as_ref(), &machine.actions)?;
    Ok(ValidationSummary {
        machine_states: machine.states.len(),
        actions: machine.actions.len(),
        context_states: context.states.len(),
        context_present: input.context.is_some(),
    })
}

/// Compute the exact reachable observational quotient of the supplied model.
///
/// # Errors
///
/// Returns [`ToolError`] for invalid input or when the configured product-state
/// bound is reached.
pub fn quotient(input: &LtsInput) -> Result<QuotientResult, ToolError> {
    require_schema(&input.schema_version, LTS_SCHEMA)?;
    validate_limits(input.limits)?;
    let machine = index_machine(&input.machine)?;
    let context = index_context(input.context.as_ref(), &machine.actions)?;
    let graph = build_product(&machine, &context, input.limits.max_product_states)?;

    let mut classes = vec![0_usize; graph.nodes.len()];
    loop {
        let mut signatures = BTreeMap::<(String, Vec<(usize, String, usize)>), usize>::new();
        let mut next = vec![0_usize; graph.nodes.len()];
        for (node_index, node) in graph.nodes.iter().enumerate() {
            let transition_signature = graph.transitions[node_index]
                .iter()
                .enumerate()
                .filter_map(|(action_index, transition)| {
                    transition.as_ref().map(|(target, emission)| {
                        (action_index, emission.clone(), classes[*target])
                    })
                })
                .collect();
            let signature = (
                machine.states[node.machine].observation.clone(),
                transition_signature,
            );
            let next_id = signatures.len();
            next[node_index] = *signatures.entry(signature).or_insert(next_id);
        }
        if next == classes {
            break;
        }
        classes = next;
    }

    let mut raw_groups = BTreeMap::<usize, Vec<ProductStateName>>::new();
    for (node, class) in graph.nodes.iter().zip(&classes) {
        raw_groups
            .entry(*class)
            .or_default()
            .push(ProductStateName {
                machine_state: machine.states[node.machine].id.clone(),
                context_state: context.states[node.context].id.clone(),
            });
    }
    let mut groups: Vec<Vec<ProductStateName>> = raw_groups.into_values().collect();
    for group in &mut groups {
        group.sort();
    }
    groups.sort_by(|left, right| left[0].cmp(&right[0]));
    let classes: Vec<QuotientClass> = groups
        .into_iter()
        .enumerate()
        .map(|(id, members)| QuotientClass {
            id,
            representative: members[0].clone(),
            members,
        })
        .collect();

    let reached_machine: BTreeSet<usize> = graph.nodes.iter().map(|node| node.machine).collect();
    let mut unreachable_machine_states: Vec<String> = machine
        .states
        .iter()
        .enumerate()
        .filter(|(index, _)| !reached_machine.contains(index))
        .map(|(_, state)| state.id.clone())
        .collect();
    unreachable_machine_states.sort();

    Ok(QuotientResult {
        algorithm: "deterministic_partition_refinement_v1",
        context_present: input.context.is_some(),
        reachable_product_states: graph.nodes.len(),
        unreachable_machine_states,
        equivalence_class_count: classes.len(),
        minimum_product_monitor_state_bits: ceil_log2(classes.len()),
        classes,
        nonclaims: vec![
            "This is exact only for the supplied finite deterministic model and context monitor.",
            "The state-bit count includes the context monitor; it is not a lower bound on machine-owned state when context is external.",
            "The context is an output-blind action-language monitor, and residual admissible-action domains are part of the quotient behavior.",
            "It does not prove equivalence of source code that was abstracted into the model.",
            "Finite-prefix observations do not establish liveness, fairness, timing, or unmodeled effects.",
        ],
    })
}

/// Compare two finite machines and return a shortest distinguishing action word.
///
/// # Errors
///
/// Returns [`ToolError`] for invalid input or when a configured exploration bound
/// prevents an equivalence decision.
#[allow(clippy::too_many_lines)]
pub fn compare(input: &CompareInput) -> Result<ComparisonResult, ToolError> {
    require_schema(&input.schema_version, COMPARE_SCHEMA)?;
    validate_limits(input.limits)?;
    let left = index_machine(&input.left)?;
    let right = index_machine(&input.right)?;
    if left.actions != right.actions {
        return Err(ToolError::invalid(
            "left and right machines must declare the same canonically sorted action set",
        ));
    }
    let context = index_context(input.context.as_ref(), &left.actions)?;

    let start = (left.initial, right.initial, context.initial);
    let mut nodes = vec![PairNode {
        left: start.0,
        right: start.1,
        context: start.2,
        parent: None,
        depth: 0,
    }];
    let mut seen = BTreeMap::from([(start, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut truncated_by_length = false;

    let initial_left = &left.states[left.initial];
    let initial_right = &right.states[right.initial];
    if initial_left.observation != initial_right.observation {
        return Ok(comparison_failure(
            &nodes,
            &left.actions,
            0,
            None,
            "state_observation",
            initial_left.observation.clone(),
            initial_right.observation.clone(),
        ));
    }

    while let Some(node_index) = queue.pop_front() {
        let node_left = nodes[node_index].left;
        let node_right = nodes[node_index].right;
        let node_context = nodes[node_index].context;
        let node_depth = nodes[node_index].depth;
        let left_state = &left.states[node_left];
        let right_state = &right.states[node_right];
        if node_depth >= input.limits.max_witness_length {
            if context.states[node_context]
                .transitions
                .iter()
                .any(Option::is_some)
            {
                truncated_by_length = true;
            }
            continue;
        }

        for (action_index, context_target) in
            context.states[node_context].transitions.iter().enumerate()
        {
            let Some(context_target) = context_target else {
                continue;
            };
            let left_transition = &left_state.transitions[action_index];
            let right_transition = &right_state.transitions[action_index];
            if left_transition.emission != right_transition.emission {
                return Ok(comparison_failure(
                    &nodes,
                    &left.actions,
                    node_index,
                    Some(action_index),
                    "transition_emission",
                    left_transition.emission.clone(),
                    right_transition.emission.clone(),
                ));
            }
            let successor = (left_transition.to, right_transition.to, *context_target);
            let successor_left = &left.states[successor.0];
            let successor_right = &right.states[successor.1];
            if successor_left.observation != successor_right.observation {
                return Ok(comparison_failure(
                    &nodes,
                    &left.actions,
                    node_index,
                    Some(action_index),
                    "state_observation",
                    successor_left.observation.clone(),
                    successor_right.observation.clone(),
                ));
            }
            if let Entry::Vacant(entry) = seen.entry(successor) {
                if nodes.len() >= input.limits.max_pairs {
                    return Err(ToolError::resource_limit(format!(
                        "pair exploration reached max_pairs={} before equivalence was decided",
                        input.limits.max_pairs
                    )));
                }
                let successor_index = nodes.len();
                nodes.push(PairNode {
                    left: successor.0,
                    right: successor.1,
                    context: successor.2,
                    parent: Some((node_index, action_index)),
                    depth: node_depth + 1,
                });
                entry.insert(successor_index);
                queue.push_back(successor_index);
            }
        }
    }

    if truncated_by_length {
        return Err(ToolError::resource_limit(format!(
            "witness search reached max_witness_length={} before equivalence was decided",
            input.limits.max_witness_length
        )));
    }

    Ok(ComparisonResult {
        algorithm: "deterministic_product_bfs_v1",
        equivalent: true,
        explored_pairs: nodes.len(),
        witness: None,
        nonclaims: comparison_nonclaims(),
    })
}

pub(crate) fn require_schema(actual: &str, expected: &str) -> Result<(), ToolError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ToolError::invalid(format!(
            "unsupported schema_version {actual:?}; expected {expected:?}"
        )))
    }
}

fn validate_limits(limits: Limits) -> Result<(), ToolError> {
    if limits.max_product_states == 0 || limits.max_pairs == 0 {
        return Err(ToolError::invalid(
            "max_product_states and max_pairs must both be positive",
        ));
    }
    Ok(())
}

pub(crate) fn index_machine(machine: &Machine) -> Result<IndexedMachine, ToolError> {
    if machine.actions.is_empty() {
        return Err(ToolError::invalid("machine.actions must not be empty"));
    }
    if machine.states.is_empty() {
        return Err(ToolError::invalid("machine.states must not be empty"));
    }

    let mut actions = machine.actions.clone();
    actions.sort();
    if actions.iter().any(String::is_empty) {
        return Err(ToolError::invalid("action IDs must not be empty"));
    }
    if actions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ToolError::invalid("action IDs must be unique"));
    }

    let mut state_ids = BTreeMap::new();
    for (index, state) in machine.states.iter().enumerate() {
        if state.id.is_empty() {
            return Err(ToolError::invalid("state IDs must not be empty"));
        }
        if state_ids.insert(state.id.clone(), index).is_some() {
            return Err(ToolError::invalid(format!(
                "duplicate state ID {:?}",
                state.id
            )));
        }
    }
    let initial = state_ids.get(&machine.initial).copied().ok_or_else(|| {
        ToolError::invalid(format!(
            "initial state {:?} does not exist",
            machine.initial
        ))
    })?;

    let expected_actions: BTreeSet<&str> = actions.iter().map(String::as_str).collect();
    let mut states = Vec::with_capacity(machine.states.len());
    for state in &machine.states {
        let actual_actions: BTreeSet<&str> = state.transitions.keys().map(String::as_str).collect();
        if actual_actions != expected_actions {
            return Err(ToolError::invalid(format!(
                "state {:?} must define exactly one transition for every declared action",
                state.id
            )));
        }
        let mut transitions = Vec::with_capacity(actions.len());
        for action in &actions {
            let transition = &state.transitions[action];
            let to = state_ids.get(&transition.to).copied().ok_or_else(|| {
                ToolError::invalid(format!(
                    "state {:?}, action {:?} targets unknown state {:?}",
                    state.id, action, transition.to
                ))
            })?;
            transitions.push(IndexedTransition {
                to,
                emission: transition.emission.clone(),
            });
        }
        states.push(IndexedState {
            id: state.id.clone(),
            observation: state.observation.clone(),
            transitions,
        });
    }
    Ok(IndexedMachine {
        actions,
        initial,
        states,
    })
}

pub(crate) fn index_context(
    context: Option<&ContextMonitor>,
    actions: &[String],
) -> Result<IndexedContext, ToolError> {
    let Some(context) = context else {
        return Ok(IndexedContext {
            initial: 0,
            states: vec![IndexedContextState {
                id: "__all_actions__".to_owned(),
                transitions: vec![Some(0); actions.len()],
            }],
        });
    };
    if context.states.is_empty() {
        return Err(ToolError::invalid("context.states must not be empty"));
    }
    let mut ids = BTreeMap::new();
    for (index, state) in context.states.iter().enumerate() {
        if state.id.is_empty() {
            return Err(ToolError::invalid("context state IDs must not be empty"));
        }
        if ids.insert(state.id.clone(), index).is_some() {
            return Err(ToolError::invalid(format!(
                "duplicate context state ID {:?}",
                state.id
            )));
        }
    }
    let initial = ids.get(&context.initial).copied().ok_or_else(|| {
        ToolError::invalid(format!(
            "context initial state {:?} does not exist",
            context.initial
        ))
    })?;
    let action_indexes: BTreeMap<&str, usize> = actions
        .iter()
        .enumerate()
        .map(|(index, action)| (action.as_str(), index))
        .collect();
    let mut states = Vec::with_capacity(context.states.len());
    for state in &context.states {
        let mut transitions = vec![None; actions.len()];
        for (action, target) in &state.transitions {
            let action_index = action_indexes
                .get(action.as_str())
                .copied()
                .ok_or_else(|| {
                    ToolError::invalid(format!(
                        "context state {:?} mentions unknown action {:?}",
                        state.id, action
                    ))
                })?;
            let target_index = ids.get(target).copied().ok_or_else(|| {
                ToolError::invalid(format!(
                    "context state {:?}, action {:?} targets unknown context state {:?}",
                    state.id, action, target
                ))
            })?;
            transitions[action_index] = Some(target_index);
        }
        states.push(IndexedContextState {
            id: state.id.clone(),
            transitions,
        });
    }
    Ok(IndexedContext { initial, states })
}

pub(crate) fn build_product(
    machine: &IndexedMachine,
    context: &IndexedContext,
    max_product_states: usize,
) -> Result<ProductGraph, ToolError> {
    build_product_internal(machine, context, max_product_states, false).map(|(graph, _)| graph)
}

pub(crate) fn build_product_with_access(
    machine: &IndexedMachine,
    context: &IndexedContext,
    max_product_states: usize,
) -> Result<(ProductGraph, ProductAccess), ToolError> {
    let (graph, access) = build_product_internal(machine, context, max_product_states, true)?;
    let access = access.ok_or_else(|| {
        ToolError::internal("product exploration did not retain requested access paths")
    })?;
    Ok((graph, access))
}

fn build_product_internal(
    machine: &IndexedMachine,
    context: &IndexedContext,
    max_product_states: usize,
    retain_access: bool,
) -> Result<(ProductGraph, Option<ProductAccess>), ToolError> {
    let start = ProductState {
        machine: machine.initial,
        context: context.initial,
    };
    let mut nodes = vec![start];
    let mut indexes = BTreeMap::from([(start, 0_usize)]);
    let mut queue = VecDeque::from([0_usize]);
    let mut transitions = Vec::<Vec<Option<(usize, String)>>>::new();
    let mut parents = retain_access.then(|| vec![None]);
    let mut depths = retain_access.then(|| vec![0]);

    while let Some(node_index) = queue.pop_front() {
        let node = nodes[node_index];
        let mut row = vec![None; machine.actions.len()];
        for (action_index, context_target) in
            context.states[node.context].transitions.iter().enumerate()
        {
            let Some(context_target) = context_target else {
                continue;
            };
            let transition = &machine.states[node.machine].transitions[action_index];
            let successor = ProductState {
                machine: transition.to,
                context: *context_target,
            };
            let successor_index = if let Some(existing) = indexes.get(&successor) {
                *existing
            } else {
                if nodes.len() >= max_product_states {
                    return Err(ToolError::resource_limit(format!(
                        "reachable product reached max_product_states={max_product_states}"
                    )));
                }
                let index = nodes.len();
                nodes.push(successor);
                indexes.insert(successor, index);
                if let Some(parents) = &mut parents {
                    parents.push(Some((node_index, action_index)));
                }
                if let Some(depths) = &mut depths {
                    depths.push(depths[node_index] + 1);
                }
                queue.push_back(index);
                index
            };
            row[action_index] = Some((successor_index, transition.emission.clone()));
        }
        transitions.push(row);
    }

    let access = parents
        .zip(depths)
        .map(|(parents, depths)| ProductAccess { parents, depths });
    Ok((ProductGraph { nodes, transitions }, access))
}

fn comparison_failure(
    nodes: &[PairNode],
    actions: &[String],
    node_index: usize,
    final_action: Option<usize>,
    kind: &'static str,
    left: String,
    right: String,
) -> ComparisonResult {
    let mut action_indexes = Vec::new();
    let mut cursor = node_index;
    while let Some((parent, action)) = nodes[cursor].parent {
        action_indexes.push(action);
        cursor = parent;
    }
    action_indexes.reverse();
    let mut word: Vec<String> = action_indexes
        .into_iter()
        .map(|action| actions[action].clone())
        .collect();
    if let Some(action) = final_action {
        word.push(actions[action].clone());
    }
    ComparisonResult {
        algorithm: "deterministic_product_bfs_v1",
        equivalent: false,
        explored_pairs: nodes.len(),
        witness: Some(DistinguishingWitness {
            length_actions: word.len(),
            word,
            mismatch: Mismatch { kind, left, right },
        }),
        nonclaims: comparison_nonclaims(),
    }
}

fn comparison_nonclaims() -> Vec<&'static str> {
    vec![
        "Equivalence is relative to the supplied finite deterministic machines and context monitor.",
        "The result covers finite-prefix observations and emissions only.",
        "Unmodeled timing, fairness, liveness, RNG, identity, failure, ABI, and reset behavior are outside the result.",
    ]
}

fn ceil_log2(value: usize) -> u32 {
    if value <= 1 {
        0
    } else {
        usize::BITS - (value - 1).leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(id: &str, observation: &str, next: &str, emission: &str) -> State {
        State {
            id: id.to_owned(),
            observation: observation.to_owned(),
            transitions: BTreeMap::from([(
                "tick".to_owned(),
                Transition {
                    to: next.to_owned(),
                    emission: emission.to_owned(),
                },
            )]),
        }
    }

    fn machine(states: Vec<State>) -> Machine {
        Machine {
            initial: "a".to_owned(),
            actions: vec!["tick".to_owned()],
            states,
        }
    }

    fn machine_from_value(value: serde_json::Value) -> Machine {
        serde_json::from_value(value).expect("test machine must deserialize")
    }

    #[test]
    fn quotient_merges_future_indistinguishable_states() {
        let equivalent_tail = |id: &str| State {
            id: id.to_owned(),
            observation: "tail".to_owned(),
            transitions: BTreeMap::from([
                (
                    "left".to_owned(),
                    Transition {
                        to: id.to_owned(),
                        emission: "loop-left".to_owned(),
                    },
                ),
                (
                    "right".to_owned(),
                    Transition {
                        to: id.to_owned(),
                        emission: "loop-right".to_owned(),
                    },
                ),
            ]),
        };
        let input = LtsInput {
            schema_version: LTS_SCHEMA.to_owned(),
            machine: Machine {
                initial: "root".to_owned(),
                actions: vec!["right".to_owned(), "left".to_owned()],
                states: vec![
                    State {
                        id: "root".to_owned(),
                        observation: "root".to_owned(),
                        transitions: BTreeMap::from([
                            (
                                "left".to_owned(),
                                Transition {
                                    to: "tail-a".to_owned(),
                                    emission: "branch".to_owned(),
                                },
                            ),
                            (
                                "right".to_owned(),
                                Transition {
                                    to: "tail-b".to_owned(),
                                    emission: "branch".to_owned(),
                                },
                            ),
                        ]),
                    },
                    equivalent_tail("tail-a"),
                    equivalent_tail("tail-b"),
                    equivalent_tail("unused"),
                ],
            },
            context: None,
            limits: Limits::default(),
        };
        let result = quotient(&input).expect("valid quotient");
        assert_eq!(result.equivalence_class_count, 2);
        assert_eq!(result.unreachable_machine_states, vec!["unused"]);
        assert_eq!(result.minimum_product_monitor_state_bits, 1);
        assert!(result.classes.iter().any(|class| {
            class
                .members
                .iter()
                .map(|member| member.machine_state.as_str())
                .eq(["tail-a", "tail-b"])
        }));
    }

    #[test]
    fn quotient_distinguishes_residual_context_languages() {
        let input = LtsInput {
            schema_version: LTS_SCHEMA.to_owned(),
            machine: machine_from_value(json!({
                "initial": "only", "actions": ["a", "b"], "states": [{
                    "id": "only", "observation": "same", "transitions": {
                        "a": {"to": "only", "emission": "same"},
                        "b": {"to": "only", "emission": "same"}
                    }
                }]
            })),
            context: Some(ContextMonitor {
                initial: "branch".to_owned(),
                states: vec![
                    ContextState {
                        id: "branch".to_owned(),
                        transitions: BTreeMap::from([
                            ("a".to_owned(), "only-a".to_owned()),
                            ("b".to_owned(), "only-b".to_owned()),
                        ]),
                    },
                    ContextState {
                        id: "only-a".to_owned(),
                        transitions: BTreeMap::from([("a".to_owned(), "only-a".to_owned())]),
                    },
                    ContextState {
                        id: "only-b".to_owned(),
                        transitions: BTreeMap::from([("b".to_owned(), "only-b".to_owned())]),
                    },
                ],
            }),
            limits: Limits::default(),
        };
        let result = quotient(&input).expect("valid quotient");
        assert_eq!(result.reachable_product_states, 3);
        assert_eq!(result.equivalence_class_count, 3);
        assert_eq!(result.minimum_product_monitor_state_bits, 2);
    }

    #[test]
    fn compare_returns_shortest_emission_witness() {
        let left = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "same", "b", "left"),
        ]);
        let right = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "same", "b", "right"),
        ]);
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits::default(),
        })
        .expect("comparison completes");
        let witness = result.witness.expect("must distinguish");
        assert_eq!(witness.word, vec!["tick", "tick"]);
        assert_eq!(witness.mismatch.kind, "transition_emission");
    }

    #[test]
    fn compare_reports_zero_length_observation_witness() {
        let left = machine(vec![state("a", "left", "a", "same")]);
        let right = machine(vec![state("a", "right", "a", "same")]);
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits::default(),
        })
        .expect("comparison completes");
        let witness = result.witness.expect("must distinguish");
        assert!(witness.word.is_empty());
        assert_eq!(witness.length_actions, 0);
    }

    #[test]
    fn shorter_sibling_observation_beats_deeper_emission() {
        let left = machine_from_value(json!({
            "initial": "l0", "actions": ["a", "b"], "states": [
                {"id": "l0", "observation": "same", "transitions": {
                    "a": {"to": "la", "emission": "root"},
                    "b": {"to": "lb", "emission": "root"}}},
                {"id": "la", "observation": "same-a", "transitions": {
                    "a": {"to": "la", "emission": "left-aa"},
                    "b": {"to": "la", "emission": "same"}}},
                {"id": "lb", "observation": "left-b", "transitions": {
                    "a": {"to": "lb", "emission": "same"},
                    "b": {"to": "lb", "emission": "same"}}}
            ]
        }));
        let right = machine_from_value(json!({
            "initial": "r0", "actions": ["a", "b"], "states": [
                {"id": "r0", "observation": "same", "transitions": {
                    "a": {"to": "ra", "emission": "root"},
                    "b": {"to": "rb", "emission": "root"}}},
                {"id": "ra", "observation": "same-a", "transitions": {
                    "a": {"to": "ra", "emission": "right-aa"},
                    "b": {"to": "ra", "emission": "same"}}},
                {"id": "rb", "observation": "right-b", "transitions": {
                    "a": {"to": "rb", "emission": "same"},
                    "b": {"to": "rb", "emission": "same"}}}
            ]
        }));
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits::default(),
        })
        .expect("comparison completes");
        assert_eq!(result.witness.expect("witness").word, ["b"]);
    }

    #[test]
    fn canonical_word_order_wins_across_mismatch_kinds() {
        let left = machine_from_value(json!({
            "initial": "l0", "actions": ["b", "a"], "states": [
                {"id": "l0", "observation": "same", "transitions": {
                    "a": {"to": "la", "emission": "same"},
                    "b": {"to": "l0", "emission": "left-b"}}},
                {"id": "la", "observation": "left-a", "transitions": {
                    "a": {"to": "la", "emission": "same"},
                    "b": {"to": "la", "emission": "same"}}}
            ]
        }));
        let right = machine_from_value(json!({
            "initial": "r0", "actions": ["a", "b"], "states": [
                {"id": "r0", "observation": "same", "transitions": {
                    "a": {"to": "ra", "emission": "same"},
                    "b": {"to": "r0", "emission": "right-b"}}},
                {"id": "ra", "observation": "right-a", "transitions": {
                    "a": {"to": "ra", "emission": "same"},
                    "b": {"to": "ra", "emission": "same"}}}
            ]
        }));
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits::default(),
        })
        .expect("comparison completes");
        assert_eq!(result.witness.expect("witness").word, ["a"]);
    }

    #[test]
    fn available_observation_witness_precedes_pair_limit() {
        let left = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "left", "b", "same"),
        ]);
        let right = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "right", "b", "same"),
        ]);
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits {
                max_product_states: 1,
                max_pairs: 1,
                max_witness_length: 1,
            },
        })
        .expect("edge already contains a bounded witness");
        assert_eq!(result.witness.expect("witness").word, ["tick"]);
    }

    #[test]
    fn invalid_partial_machine_is_rejected() {
        let invalid = Machine {
            initial: "a".to_owned(),
            actions: vec!["tick".to_owned(), "cast".to_owned()],
            states: vec![state("a", "same", "a", "same")],
        };
        let error = index_machine(&invalid).expect_err("partial machine is invalid");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn length_limit_is_unknown_not_equivalent() {
        let left = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "same", "b", "left"),
        ]);
        let right = machine(vec![
            state("a", "same", "b", "same"),
            state("b", "same", "b", "right"),
        ]);
        let error = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: None,
            limits: Limits {
                max_witness_length: 1,
                ..Limits::default()
            },
        })
        .expect_err("bound must prevent an equivalence claim");
        assert_eq!(error.code, "resource_limit");
    }

    #[test]
    fn context_can_make_an_unavailable_difference_irrelevant() {
        let left = machine(vec![state("a", "same", "a", "left")]);
        let right = machine(vec![state("a", "same", "a", "right")]);
        let context = ContextMonitor {
            initial: "stopped".to_owned(),
            states: vec![ContextState {
                id: "stopped".to_owned(),
                transitions: BTreeMap::new(),
            }],
        };
        let result = compare(&CompareInput {
            schema_version: COMPARE_SCHEMA.to_owned(),
            left,
            right,
            context: Some(context),
            limits: Limits::default(),
        })
        .expect("comparison completes");
        assert!(result.equivalent);
    }
}
