use crate::lts::{
    ContextMonitor, Machine, ProductAccess, ProductGraph, build_product_with_access, index_context,
    index_machine, require_schema,
};
use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const FACTOR_SCHEMA: &str = "kerf.factor.v1";
pub const REPRESENTATION_SCHEMA: &str = "kerf.factor-representation.v2";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorInput {
    pub schema_version: String,
    pub machine: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
    pub factor_map: Vec<FactorMapEntry>,
    #[serde(default)]
    pub limits: FactorLimits,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FactorMapEntry {
    pub machine_state: String,
    pub context_state: Option<String>,
    pub boundary_state: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FactorLimits {
    pub max_product_states: usize,
}

impl Default for FactorLimits {
    fn default() -> Self {
        Self {
            max_product_states: 1_000_000,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FactorResult {
    pub algorithm: &'static str,
    pub lawful: bool,
    pub context_retained: bool,
    pub reachable_product_states: usize,
    pub unreachable_machine_states: Vec<String>,
    pub boundary_state_count: usize,
    pub representation: Option<BoundaryRepresentation>,
    pub witness: Option<FactorWitness>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryRepresentation {
    pub schema_version: String,
    pub certified_context: CertifiedContext,
    pub initial: String,
    pub actions: Vec<String>,
    pub states: Vec<BoundaryState>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CertifiedContext {
    Monitor(ContextMonitor),
    AllActions(()),
}

impl CertifiedContext {
    #[must_use]
    pub fn as_monitor(&self) -> Option<&ContextMonitor> {
        match self {
            Self::Monitor(context) => Some(context),
            Self::AllActions(()) => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryState {
    pub id: String,
    pub observation: String,
    #[serde(deserialize_with = "crate::report::deserialize_string_map_no_duplicates")]
    pub transitions: BTreeMap<String, BoundaryTransition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoundaryTransition {
    Constrained { to: String, emission: String },
    Unconstrained,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FactorSource {
    pub machine_state: String,
    pub context_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FactorWitness {
    pub law: &'static str,
    pub boundary_state: String,
    pub action: Option<String>,
    pub exposure_length_actions: usize,
    pub left: FactorEvidence,
    pub right: FactorEvidence,
}

#[derive(Debug, Serialize)]
pub struct FactorEvidence {
    pub source: FactorSource,
    pub access_word: Vec<String>,
    pub value: String,
}

#[derive(Debug)]
struct Conflict {
    law: &'static str,
    boundary_state: String,
    action_index: Option<usize>,
    left_node: usize,
    right_node: usize,
    left_value: String,
    right_value: String,
}

/// Check whether a supplied boundary-state map is a lawful finite factor.
///
/// # Errors
///
/// Returns [`ToolError`] when the schema, finite model, map domain, or bound is
/// invalid, or when the reachable machine-by-context product exceeds its bound.
pub fn check(input: &FactorInput) -> Result<FactorResult, ToolError> {
    require_schema(&input.schema_version, FACTOR_SCHEMA)?;
    if input.limits.max_product_states == 0 {
        return Err(ToolError::invalid(
            "limits.max_product_states must be positive",
        ));
    }

    let machine = index_machine(&input.machine)?;
    let context = index_context(input.context.as_ref(), &machine.actions)?;
    let factor_map = validate_factor_map_structure(input, &machine, &context)?;
    let (graph, access) =
        build_product_with_access(&machine, &context, input.limits.max_product_states)?;
    validate_factor_map_domain(
        input.context.is_some(),
        &factor_map,
        &machine,
        &context,
        &graph,
    )?;
    let mut boundary_by_node = Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        let source = source_for_node(input.context.is_some(), node, &machine, &context);
        let boundary_state = factor_map.get(&source).ok_or_else(|| {
            ToolError::internal(format!(
                "validated factor map lost reachable source {source:?}"
            ))
        })?;
        boundary_by_node.push(boundary_state.clone());
    }

    evaluate_partition(
        &machine,
        input.context.as_ref(),
        &context,
        &graph,
        &access,
        &boundary_by_node,
    )
}

pub(crate) fn evaluate_partition(
    machine: &crate::lts::IndexedMachine,
    certified_context: Option<&ContextMonitor>,
    context: &crate::lts::IndexedContext,
    graph: &ProductGraph,
    access: &ProductAccess,
    boundary_by_node: &[String],
) -> Result<FactorResult, ToolError> {
    let context_present = certified_context.is_some();
    let unreachable_machine_states = unreachable_machine_states(machine, graph);
    let groups = partition_groups(graph, boundary_by_node)?;

    let conflicts = find_conflicts(machine, graph, boundary_by_node, &groups, false);
    if let Some(conflict) = conflicts
        .into_iter()
        .min_by(|left, right| compare_conflicts(left, right, access))
    {
        return Ok(FactorResult {
            algorithm: "context_retained_factor_map_v1",
            lawful: false,
            context_retained: true,
            reachable_product_states: graph.nodes.len(),
            unreachable_machine_states,
            boundary_state_count: groups.len(),
            representation: None,
            witness: Some(witness_from_conflict(
                conflict,
                context_present,
                machine,
                context,
                graph,
                access,
            )),
            nonclaims: factor_nonclaims(),
        });
    }

    let representation =
        build_representation(machine, certified_context, graph, boundary_by_node, &groups);
    Ok(FactorResult {
        algorithm: "context_retained_factor_map_v1",
        lawful: true,
        context_retained: true,
        reachable_product_states: graph.nodes.len(),
        unreachable_machine_states,
        boundary_state_count: groups.len(),
        representation: Some(representation),
        witness: None,
        nonclaims: factor_nonclaims(),
    })
}

pub(crate) fn admit_partition(
    machine: &crate::lts::IndexedMachine,
    certified_context: Option<&ContextMonitor>,
    graph: &ProductGraph,
    boundary_by_node: &[String],
) -> Result<Option<BoundaryRepresentation>, ToolError> {
    let groups = partition_groups(graph, boundary_by_node)?;
    if find_conflicts(machine, graph, boundary_by_node, &groups, true).is_empty() {
        Ok(Some(build_representation(
            machine,
            certified_context,
            graph,
            boundary_by_node,
            &groups,
        )))
    } else {
        Ok(None)
    }
}

fn partition_groups(
    graph: &ProductGraph,
    boundary_by_node: &[String],
) -> Result<BTreeMap<String, Vec<usize>>, ToolError> {
    if boundary_by_node.len() != graph.nodes.len() {
        return Err(ToolError::internal(
            "factor partition length does not match reachable product",
        ));
    }
    if boundary_by_node.iter().any(String::is_empty) {
        return Err(ToolError::internal(
            "factor partition contains an empty boundary-state ID",
        ));
    }

    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (node_index, boundary_state) in boundary_by_node.iter().enumerate() {
        groups
            .entry(boundary_state.clone())
            .or_default()
            .push(node_index);
    }
    Ok(groups)
}

fn validate_factor_map_structure(
    input: &FactorInput,
    machine: &crate::lts::IndexedMachine,
    context: &crate::lts::IndexedContext,
) -> Result<BTreeMap<FactorSource, String>, ToolError> {
    let machine_ids: BTreeSet<&str> = machine
        .states
        .iter()
        .map(|state| state.id.as_str())
        .collect();
    let context_ids: BTreeSet<&str> = context
        .states
        .iter()
        .map(|state| state.id.as_str())
        .collect();
    let mut supplied = BTreeMap::new();
    let mut entries: Vec<&FactorMapEntry> = input.factor_map.iter().collect();
    entries.sort_by(|left, right| {
        (
            &left.machine_state,
            &left.context_state,
            &left.boundary_state,
        )
            .cmp(&(
                &right.machine_state,
                &right.context_state,
                &right.boundary_state,
            ))
    });

    for entry in entries {
        if entry.boundary_state.is_empty() {
            return Err(ToolError::invalid("boundary-state IDs must not be empty"));
        }
        if !machine_ids.contains(entry.machine_state.as_str()) {
            return Err(ToolError::invalid(format!(
                "factor_map mentions unknown machine state {:?}",
                entry.machine_state
            )));
        }
        match (input.context.is_some(), entry.context_state.as_deref()) {
            (false, Some(context_state)) => {
                return Err(ToolError::invalid(format!(
                    "factor_map context_state must be null without a context; found {context_state:?}"
                )));
            }
            (true, None) => {
                return Err(ToolError::invalid(
                    "factor_map context_state must name a state when context is present",
                ));
            }
            (true, Some(context_state)) if !context_ids.contains(context_state) => {
                return Err(ToolError::invalid(format!(
                    "factor_map mentions unknown context state {context_state:?}"
                )));
            }
            _ => {}
        }
        let source = FactorSource {
            machine_state: entry.machine_state.clone(),
            context_state: entry.context_state.clone(),
        };
        if supplied
            .insert(source.clone(), entry.boundary_state.clone())
            .is_some()
        {
            return Err(ToolError::invalid(format!(
                "duplicate factor_map source {source:?}"
            )));
        }
    }

    let initial_source = FactorSource {
        machine_state: machine.states[machine.initial].id.clone(),
        context_state: input
            .context
            .as_ref()
            .map(|_| context.states[context.initial].id.clone()),
    };
    if !supplied.contains_key(&initial_source) {
        return Err(ToolError::invalid(format!(
            "factor_map is missing reachable initial source {initial_source:?}"
        )));
    }

    Ok(supplied)
}

fn validate_factor_map_domain(
    context_present: bool,
    supplied: &BTreeMap<FactorSource, String>,
    machine: &crate::lts::IndexedMachine,
    context: &crate::lts::IndexedContext,
    graph: &ProductGraph,
) -> Result<(), ToolError> {
    let reachable: BTreeSet<FactorSource> = graph
        .nodes
        .iter()
        .map(|node| source_for_node(context_present, node, machine, context))
        .collect();
    if let Some(extra) = supplied.keys().find(|source| !reachable.contains(*source)) {
        return Err(ToolError::invalid(format!(
            "factor_map source {extra:?} is not reachable under the supplied context"
        )));
    }
    if let Some(missing) = reachable
        .iter()
        .find(|source| !supplied.contains_key(*source))
    {
        return Err(ToolError::invalid(format!(
            "factor_map is missing reachable source {missing:?}"
        )));
    }
    Ok(())
}

pub(crate) fn source_for_node(
    context_present: bool,
    node: &crate::lts::ProductState,
    machine: &crate::lts::IndexedMachine,
    context: &crate::lts::IndexedContext,
) -> FactorSource {
    FactorSource {
        machine_state: machine.states[node.machine].id.clone(),
        context_state: context_present.then(|| context.states[node.context].id.clone()),
    }
}

fn unreachable_machine_states(
    machine: &crate::lts::IndexedMachine,
    graph: &ProductGraph,
) -> Vec<String> {
    let reached: BTreeSet<usize> = graph.nodes.iter().map(|node| node.machine).collect();
    let mut unreachable: Vec<String> = machine
        .states
        .iter()
        .enumerate()
        .filter(|(index, _)| !reached.contains(index))
        .map(|(_, state)| state.id.clone())
        .collect();
    unreachable.sort();
    unreachable
}

fn find_conflicts(
    machine: &crate::lts::IndexedMachine,
    graph: &ProductGraph,
    boundary_by_node: &[String],
    groups: &BTreeMap<String, Vec<usize>>,
    stop_after_first: bool,
) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    for (boundary_state, members) in groups {
        let first_node = members[0];
        let first_observation = &machine.states[graph.nodes[first_node].machine].observation;
        for &node_index in &members[1..] {
            let observation = &machine.states[graph.nodes[node_index].machine].observation;
            if observation != first_observation {
                conflicts.push(Conflict {
                    law: "observation_not_constant",
                    boundary_state: boundary_state.clone(),
                    action_index: None,
                    left_node: first_node,
                    right_node: node_index,
                    left_value: first_observation.clone(),
                    right_value: observation.clone(),
                });
                if stop_after_first {
                    return conflicts;
                }
            }
        }

        for action_index in 0..machine.actions.len() {
            let enabled: Vec<(usize, usize, &String)> = members
                .iter()
                .filter_map(|&node_index| {
                    graph.transitions[node_index][action_index]
                        .as_ref()
                        .map(|(target, emission)| (node_index, *target, emission))
                })
                .collect();
            let Some((first_enabled_node, first_target, first_emission)) = enabled.first() else {
                continue;
            };
            let first_successor = &boundary_by_node[*first_target];
            for (node_index, target, emission) in &enabled[1..] {
                if *emission != *first_emission {
                    conflicts.push(Conflict {
                        law: "emission_not_constant",
                        boundary_state: boundary_state.clone(),
                        action_index: Some(action_index),
                        left_node: *first_enabled_node,
                        right_node: *node_index,
                        left_value: (*first_emission).clone(),
                        right_value: (*emission).clone(),
                    });
                    if stop_after_first {
                        return conflicts;
                    }
                }
                let successor = &boundary_by_node[*target];
                if successor != first_successor {
                    conflicts.push(Conflict {
                        law: "successor_not_constant",
                        boundary_state: boundary_state.clone(),
                        action_index: Some(action_index),
                        left_node: *first_enabled_node,
                        right_node: *node_index,
                        left_value: first_successor.clone(),
                        right_value: successor.clone(),
                    });
                    if stop_after_first {
                        return conflicts;
                    }
                }
            }
        }
    }
    conflicts
}

fn compare_conflicts(left: &Conflict, right: &Conflict, access: &ProductAccess) -> Ordering {
    conflict_key(left, access).cmp(&conflict_key(right, access))
}

fn conflict_key(
    conflict: &Conflict,
    access: &ProductAccess,
) -> (usize, usize, usize, Option<usize>, u8, String) {
    let law_rank = match conflict.law {
        "observation_not_constant" => 0,
        "emission_not_constant" => 1,
        "successor_not_constant" => 2,
        _ => 3,
    };
    let exposure_length = access
        .depth(conflict.left_node)
        .max(access.depth(conflict.right_node))
        + usize::from(conflict.action_index.is_some());
    // FIFO exploration over sorted actions assigns node indices in canonical
    // shortest-word shortlex order, so the indices encode the documented
    // access-word tie-break without rebuilding words for every comparison.
    (
        exposure_length,
        conflict.left_node,
        conflict.right_node,
        conflict.action_index,
        law_rank,
        conflict.boundary_state.clone(),
    )
}

fn witness_from_conflict(
    conflict: Conflict,
    context_present: bool,
    machine: &crate::lts::IndexedMachine,
    context: &crate::lts::IndexedContext,
    graph: &ProductGraph,
    access: &ProductAccess,
) -> FactorWitness {
    let left_word = access.access_word(conflict.left_node, &machine.actions);
    let right_word = access.access_word(conflict.right_node, &machine.actions);
    let exposure_length_actions =
        left_word.len().max(right_word.len()) + usize::from(conflict.action_index.is_some());
    FactorWitness {
        law: conflict.law,
        boundary_state: conflict.boundary_state,
        action: conflict
            .action_index
            .map(|index| machine.actions[index].clone()),
        exposure_length_actions,
        left: FactorEvidence {
            source: source_for_node(
                context_present,
                &graph.nodes[conflict.left_node],
                machine,
                context,
            ),
            access_word: left_word,
            value: conflict.left_value,
        },
        right: FactorEvidence {
            source: source_for_node(
                context_present,
                &graph.nodes[conflict.right_node],
                machine,
                context,
            ),
            access_word: right_word,
            value: conflict.right_value,
        },
    }
}

fn build_representation(
    machine: &crate::lts::IndexedMachine,
    certified_context: Option<&ContextMonitor>,
    graph: &ProductGraph,
    boundary_by_node: &[String],
    groups: &BTreeMap<String, Vec<usize>>,
) -> BoundaryRepresentation {
    let states = groups
        .iter()
        .map(|(boundary_state, members)| {
            let observation = machine.states[graph.nodes[members[0]].machine]
                .observation
                .clone();
            let transitions = machine
                .actions
                .iter()
                .enumerate()
                .map(|(action_index, action)| {
                    let constrained = members.iter().find_map(|&node_index| {
                        graph.transitions[node_index][action_index].as_ref().map(
                            |(target, emission)| BoundaryTransition::Constrained {
                                to: boundary_by_node[*target].clone(),
                                emission: emission.clone(),
                            },
                        )
                    });
                    (
                        action.clone(),
                        constrained.unwrap_or(BoundaryTransition::Unconstrained),
                    )
                })
                .collect();
            BoundaryState {
                id: boundary_state.clone(),
                observation,
                transitions,
            }
        })
        .collect();
    BoundaryRepresentation {
        schema_version: REPRESENTATION_SCHEMA.to_owned(),
        certified_context: canonical_context(certified_context)
            .map_or(CertifiedContext::AllActions(()), CertifiedContext::Monitor),
        initial: boundary_by_node[0].clone(),
        actions: machine.actions.clone(),
        states,
    }
}

fn canonical_context(context: Option<&ContextMonitor>) -> Option<ContextMonitor> {
    context.map(|context| {
        let mut states = context.states.clone();
        states.sort_by(|left, right| left.id.cmp(&right.id));
        ContextMonitor {
            initial: context.initial.clone(),
            states,
        }
    })
}

fn factor_nonclaims() -> Vec<&'static str> {
    vec![
        "The result is exact only for the supplied finite deterministic machine, retained context monitor, map, and bound.",
        "The context remains external side information; enlarging or changing it requires a new check.",
        "The factor map is supplied, not inferred, and the result does not claim minimum boundary-state cardinality.",
        "It does not prove equivalence of source code abstracted into the model.",
        "Finite-prefix observations do not establish liveness, fairness, timing, causality, or unmodeled effects.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn parse_input(value: Value) -> FactorInput {
        serde_json::from_value(value).expect("test factor input must deserialize")
    }

    fn context_side_information_input() -> FactorInput {
        parse_input(json!({
            "schema_version": FACTOR_SCHEMA,
            "machine": {
                "initial": "live",
                "actions": ["c", "b", "a"],
                "states": [
                    {"id": "live", "observation": "same", "transitions": {
                        "a": {"to": "live", "emission": "step"},
                        "b": {"to": "live", "emission": "step"},
                        "c": {"to": "live", "emission": "hidden"}
                    }},
                    {"id": "unused", "observation": "same", "transitions": {
                        "a": {"to": "unused", "emission": "step"},
                        "b": {"to": "unused", "emission": "step"},
                        "c": {"to": "unused", "emission": "hidden"}
                    }}
                ]
            },
            "context": {
                "initial": "branch",
                "states": [
                    {"id": "branch", "transitions": {
                        "a": "only-a", "b": "only-b"
                    }},
                    {"id": "only-a", "transitions": {"a": "only-a"}},
                    {"id": "only-b", "transitions": {"b": "only-b"}}
                ]
            },
            "factor_map": [
                {"machine_state": "live", "context_state": "branch", "boundary_state": "z"},
                {"machine_state": "live", "context_state": "only-a", "boundary_state": "z"},
                {"machine_state": "live", "context_state": "only-b", "boundary_state": "z"}
            ],
            "limits": {"max_product_states": 16}
        }))
    }

    fn branching_input(
        left_observation: &str,
        right_observation: &str,
        left_emission: &str,
        right_emission: &str,
        right_successor: &str,
    ) -> FactorInput {
        let mut value = json!({
            "schema_version": FACTOR_SCHEMA,
            "machine": {
                "initial": "root",
                "actions": ["b", "a"],
                "states": [
                    {"id": "root", "observation": "root", "transitions": {
                        "a": {"to": "left", "emission": "branch"},
                        "b": {"to": "right", "emission": "branch"}
                    }},
                    {"id": "left", "observation": left_observation, "transitions": {
                        "a": {"to": "left", "emission": left_emission},
                        "b": {"to": "left", "emission": "same"}
                    }},
                    {"id": "right", "observation": right_observation, "transitions": {
                        "a": {"to": right_successor, "emission": right_emission},
                        "b": {"to": "right", "emission": "same"}
                    }},
                    {"id": "sink", "observation": "same", "transitions": {
                        "a": {"to": "sink", "emission": "same"},
                        "b": {"to": "sink", "emission": "same"}
                    }}
                ]
            },
            "factor_map": [
                {"machine_state": "root", "context_state": null, "boundary_state": "root-z"},
                {"machine_state": "left", "context_state": null, "boundary_state": "merged-z"},
                {"machine_state": "right", "context_state": null, "boundary_state": "merged-z"}
            ],
            "limits": {"max_product_states": 16}
        });
        if right_successor == "sink" {
            value["factor_map"]
                .as_array_mut()
                .expect("factor map is an array")
                .push(json!({
                    "machine_state": "sink",
                    "context_state": null,
                    "boundary_state": "sink-z"
                }));
        }
        parse_input(value)
    }

    #[test]
    fn retained_context_allows_different_enabled_action_domains() {
        let result = check(&context_side_information_input()).expect("factor check completes");
        assert!(result.lawful);
        assert!(result.context_retained);
        assert_eq!(result.reachable_product_states, 3);
        assert_eq!(result.unreachable_machine_states, ["unused"]);
        assert_eq!(result.boundary_state_count, 1);
        assert!(result.witness.is_none());
        let representation = result
            .representation
            .expect("lawful map has a representation");
        assert_eq!(representation.initial, "z");
        assert_eq!(representation.actions, ["a", "b", "c"]);
        assert!(matches!(
            representation.states[0].transitions["a"],
            BoundaryTransition::Constrained { .. }
        ));
        assert!(matches!(
            representation.states[0].transitions["b"],
            BoundaryTransition::Constrained { .. }
        ));
        assert!(matches!(
            representation.states[0].transitions["c"],
            BoundaryTransition::Unconstrained
        ));
    }

    #[test]
    fn observation_conflict_returns_minimum_access_pair() {
        let result = check(&branching_input("left", "right", "same", "same", "right"))
            .expect("factor check completes");
        assert!(!result.lawful);
        let witness = result.witness.expect("conflict has a witness");
        assert_eq!(witness.law, "observation_not_constant");
        assert_eq!(witness.action, None);
        assert_eq!(witness.exposure_length_actions, 1);
        assert_eq!(witness.left.access_word, ["a"]);
        assert_eq!(witness.right.access_word, ["b"]);
    }

    #[test]
    fn emission_conflict_names_the_shared_enabled_action() {
        let result = check(&branching_input("same", "same", "left", "right", "right"))
            .expect("factor check completes");
        let witness = result.witness.expect("conflict has a witness");
        assert_eq!(witness.law, "emission_not_constant");
        assert_eq!(witness.action.as_deref(), Some("a"));
        assert_eq!(witness.exposure_length_actions, 2);
        assert_eq!(witness.left.value, "left");
        assert_eq!(witness.right.value, "right");
    }

    #[test]
    fn successor_conflict_names_the_induced_boundary_states() {
        let result = check(&branching_input("same", "same", "same", "same", "sink"))
            .expect("factor check completes");
        let witness = result.witness.expect("conflict has a witness");
        assert_eq!(witness.law, "successor_not_constant");
        assert_eq!(witness.action.as_deref(), Some("a"));
        assert_eq!(witness.left.value, "merged-z");
        assert_eq!(witness.right.value, "sink-z");
    }

    #[test]
    fn map_domain_must_be_exact_and_named() {
        let base = context_side_information_input();

        let mut missing = base.clone();
        missing.factor_map.pop();
        assert_eq!(
            check(&missing).expect_err("missing source must fail").code,
            "invalid_input"
        );

        let mut duplicate = base.clone();
        duplicate.factor_map.push(duplicate.factor_map[0].clone());
        assert_eq!(
            check(&duplicate)
                .expect_err("duplicate source must fail")
                .code,
            "invalid_input"
        );

        let mut unreachable = base.clone();
        unreachable.factor_map.push(FactorMapEntry {
            machine_state: "unused".to_owned(),
            context_state: Some("branch".to_owned()),
            boundary_state: "extra".to_owned(),
        });
        let error = check(&unreachable).expect_err("unreachable source must fail");
        assert!(error.message.contains("not reachable"));

        let mut unknown_machine = base.clone();
        unknown_machine.factor_map[0].machine_state = "ghost".to_owned();
        let error = check(&unknown_machine).expect_err("unknown machine state must fail");
        assert!(error.message.contains("unknown machine state"));

        let mut unknown_context = base.clone();
        unknown_context.factor_map[0].context_state = Some("ghost".to_owned());
        let error = check(&unknown_context).expect_err("unknown context state must fail");
        assert!(error.message.contains("unknown context state"));

        let mut empty_boundary = base;
        empty_boundary.factor_map[0].boundary_state.clear();
        let error = check(&empty_boundary).expect_err("empty boundary state must fail");
        assert!(error.message.contains("must not be empty"));
    }

    #[test]
    fn declaration_order_does_not_change_the_result() {
        let original = context_side_information_input();
        let mut reordered = original.clone();
        reordered.machine.states.reverse();
        reordered.machine.actions.reverse();
        reordered
            .context
            .as_mut()
            .expect("context is present")
            .states
            .reverse();
        reordered.factor_map.reverse();
        assert_eq!(
            serde_json::to_value(check(&original).expect("original check"))
                .expect("result serializes"),
            serde_json::to_value(check(&reordered).expect("reordered check"))
                .expect("result serializes")
        );
    }

    #[test]
    fn zero_product_bound_is_invalid() {
        let mut input = context_side_information_input();
        input.limits.max_product_states = 0;
        let error = check(&input).expect_err("zero bound must fail");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn exhausted_product_bound_is_unknown() {
        let mut input = context_side_information_input();
        input.limits.max_product_states = 2;
        let error = check(&input).expect_err("bound exhaustion must not prove a result");
        assert_eq!(error.code, "resource_limit");
    }

    #[test]
    fn structural_map_error_precedes_product_bound_exhaustion() {
        let mut input = context_side_information_input();
        input.limits.max_product_states = 2;
        input.factor_map[0].machine_state = "ghost".to_owned();
        let error = check(&input).expect_err("known-invalid input must not become unknown");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("unknown machine state"));

        let mut missing_initial = context_side_information_input();
        missing_initial.limits.max_product_states = 2;
        missing_initial
            .factor_map
            .retain(|entry| entry.context_state.as_deref() != Some("branch"));
        let error = check(&missing_initial).expect_err("missing initial source is already known");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("missing reachable initial source"));
    }

    #[test]
    fn witness_tie_break_prefers_shorter_access_trace_before_lexical_word() {
        fn state(id: &str, observation: &str, a: &str, b: &str, c: &str) -> Value {
            json!({
                "id": id,
                "observation": observation,
                "transitions": {
                    "a": {"to": a, "emission": "same"},
                    "b": {"to": b, "emission": "same"},
                    "c": {"to": c, "emission": "same"}
                }
            })
        }

        let input = parse_input(json!({
            "schema_version": FACTOR_SCHEMA,
            "machine": {
                "initial": "root",
                "actions": ["c", "b", "a"],
                "states": [
                    state("root", "root", "p", "dead", "x"),
                    state("p", "p", "p", "u", "w"),
                    state("dead", "dead", "dead", "dead", "dead"),
                    state("x", "one", "x", "v", "x"),
                    state("u", "one", "u", "u", "u"),
                    state("w", "zero", "w", "w", "w"),
                    state("v", "zero", "v", "v", "v")
                ]
            },
            "factor_map": [
                {"machine_state": "root", "context_state": null, "boundary_state": "root-z"},
                {"machine_state": "p", "context_state": null, "boundary_state": "p-z"},
                {"machine_state": "dead", "context_state": null, "boundary_state": "dead-z"},
                {"machine_state": "x", "context_state": null, "boundary_state": "short-z"},
                {"machine_state": "v", "context_state": null, "boundary_state": "short-z"},
                {"machine_state": "u", "context_state": null, "boundary_state": "lexical-z"},
                {"machine_state": "w", "context_state": null, "boundary_state": "lexical-z"}
            ]
        }));
        let result = check(&input).expect("factor check completes");
        let witness = result.witness.expect("two fibres conflict");
        assert_eq!(witness.boundary_state, "short-z");
        assert_eq!(witness.left.access_word, ["c"]);
        assert_eq!(witness.right.access_word, ["c", "b"]);
    }
}
