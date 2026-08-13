use crate::lts::{
    ContextMonitor, IndexedContext, IndexedMachine, Machine, ProductAccess, ProductGraph,
    build_product_with_access, index_context, index_machine, require_schema,
};
use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MORPHISM_SCHEMA: &str = "kerf.morphism.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphismInput {
    pub schema_version: String,
    pub morphism: MorphismDefinition,
    pub scope: MorphismScope,
    #[serde(default)]
    pub claims: Vec<MorphismClaim>,
    #[serde(default)]
    pub section: Option<SectionDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphismDefinition {
    pub source: Machine,
    pub target: Machine,
    pub state_map: Vec<MorphismMapEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphismMapEntry {
    pub source_state: String,
    pub target_state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MorphismScope {
    AllStates,
    ReachableUnderContext {
        #[serde(default)]
        context: Option<ContextMonitor>,
        limits: MorphismLimits,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MorphismLimits {
    pub max_product_states: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MorphismClaim {
    CarrierSurjectivity,
    ExactIsomorphism,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionDefinition {
    pub state_map: Vec<SectionMapEntry>,
    #[serde(default)]
    pub claims: Vec<SectionClaim>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionMapEntry {
    pub target_state: String,
    pub source_state: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SectionClaim {
    AsMorphism,
    TwoSidedInverse,
}

#[derive(Debug, Serialize)]
pub struct MorphismResult {
    pub algorithm: &'static str,
    pub scope: &'static str,
    pub requested_claims: Vec<&'static str>,
    pub morphism: MorphismLawResult,
    pub carrier_surjectivity: Option<SurjectivityResult>,
    pub section: Option<SectionResult>,
    pub exact_isomorphism: Option<IsomorphismResult>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct MorphismLawResult {
    pub direction: &'static str,
    pub holds: bool,
    pub checked_scoped_states: usize,
    pub unchecked_domain_states: Vec<String>,
    pub witness: Option<MorphismWitness>,
}

#[derive(Debug, Serialize)]
pub struct MorphismWitness {
    pub basis: &'static str,
    pub word: Option<Vec<String>>,
    pub length_actions: Option<usize>,
    pub law: &'static str,
    pub domain_state: String,
    pub codomain_state: String,
    pub context_state: Option<String>,
    pub action: Option<String>,
    pub left: WitnessValue,
    pub right: WitnessValue,
}

#[derive(Debug, Serialize)]
pub struct WitnessValue {
    pub expression: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurjectivityResult {
    pub holds: bool,
    pub uncovered_target: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SectionResult {
    pub right_inverse: InverseLawResult,
    pub as_morphism: Option<MorphismLawResult>,
    pub two_sided_inverse: Option<TwoSidedInverseResult>,
}

#[derive(Debug, Serialize)]
pub struct InverseLawResult {
    pub holds: bool,
    pub witness: Option<InverseWitness>,
}

#[derive(Debug, Serialize)]
pub struct InverseWitness {
    pub law: &'static str,
    pub state: String,
    pub mapped_state: String,
    pub expected_state: String,
    pub actual_state: String,
}

#[derive(Debug, Serialize)]
pub struct TwoSidedInverseResult {
    pub holds: bool,
    pub right_inverse_holds: bool,
    pub left_inverse: InverseLawResult,
}

#[derive(Debug, Serialize)]
pub struct IsomorphismResult {
    pub holds: bool,
    pub morphism_holds: bool,
    pub injectivity: InjectivityResult,
    pub surjectivity: SurjectivityResult,
}

#[derive(Debug, Serialize)]
pub struct InjectivityResult {
    pub holds: bool,
    pub collision: Option<InjectivityCollision>,
}

#[derive(Debug, Serialize)]
pub struct InjectivityCollision {
    pub target_state: String,
    pub source_states: [String; 2],
}

#[derive(Debug)]
struct ClaimSet {
    carrier_surjectivity: bool,
    exact_isomorphism: bool,
}

#[derive(Debug)]
struct SectionClaimSet {
    as_morphism: bool,
    two_sided_inverse: bool,
}

#[derive(Debug)]
struct IndexedSection {
    state_map: Vec<usize>,
    claims: SectionClaimSet,
}

/// Check one supplied total function as a finite machine morphism.
///
/// # Errors
///
/// Returns [`ToolError`] for an invalid schema, machine, map, claim, or bound.
/// A reachable-product bound reached before closure is a resource-limit error.
pub fn check(input: &MorphismInput) -> Result<MorphismResult, ToolError> {
    require_schema(&input.schema_version, MORPHISM_SCHEMA)?;
    let claims = validate_claims(&input.claims)?;
    if claims.exact_isomorphism && !matches!(input.scope, MorphismScope::AllStates) {
        return Err(ToolError::invalid(
            "exact_isomorphism requires scope.kind=\"all_states\"",
        ));
    }
    validate_scope(&input.scope)?;

    let source = index_machine(&input.morphism.source)?;
    let target = index_machine(&input.morphism.target)?;
    if source.actions != target.actions {
        return Err(ToolError::invalid(
            "source and target machines must declare the same canonically sorted action set",
        ));
    }
    let forward = index_forward_map(&input.morphism.state_map, &source, &target)?;
    let indexed_section = input
        .section
        .as_ref()
        .map(|definition| index_section(definition, &source, &target))
        .transpose()?;

    let morphism = check_direction(
        &source,
        &target,
        &forward,
        &input.scope,
        "source_to_target",
        "base_morphism",
    )?;
    let surjectivity = carrier_surjectivity(&forward, &target);
    let carrier_surjectivity = claims.carrier_surjectivity.then(|| surjectivity.clone());

    let section = indexed_section
        .as_ref()
        .map(|section| check_section(section, &source, &target, &forward, &input.scope))
        .transpose()?;

    let exact_isomorphism = claims.exact_isomorphism.then(|| {
        let injectivity = carrier_injectivity(&forward, &source, &target);
        IsomorphismResult {
            holds: morphism.holds && injectivity.holds && surjectivity.holds,
            morphism_holds: morphism.holds,
            injectivity,
            surjectivity,
        }
    });

    let mut requested_claims = Vec::new();
    if claims.carrier_surjectivity {
        requested_claims.push("carrier_surjectivity");
    }
    if claims.exact_isomorphism {
        requested_claims.push("exact_isomorphism");
    }

    Ok(MorphismResult {
        algorithm: "finite_pointed_coalgebra_morphism_v1",
        scope: scope_name(&input.scope),
        requested_claims,
        morphism,
        carrier_surjectivity,
        section,
        exact_isomorphism,
        nonclaims: morphism_nonclaims(),
    })
}

#[must_use]
pub fn applied_limits(input: &MorphismInput) -> Option<MorphismLimits> {
    match input.scope {
        MorphismScope::AllStates => None,
        MorphismScope::ReachableUnderContext { limits, .. } => Some(limits),
    }
}

fn validate_scope(scope: &MorphismScope) -> Result<(), ToolError> {
    if let MorphismScope::ReachableUnderContext { limits, .. } = scope
        && limits.max_product_states == 0
    {
        return Err(ToolError::invalid(
            "scope.limits.max_product_states must be positive",
        ));
    }
    Ok(())
}

fn validate_claims(raw: &[MorphismClaim]) -> Result<ClaimSet, ToolError> {
    let claims: BTreeSet<_> = raw.iter().copied().collect();
    if claims.len() != raw.len() {
        return Err(ToolError::invalid("morphism claims must be unique"));
    }
    Ok(ClaimSet {
        carrier_surjectivity: claims.contains(&MorphismClaim::CarrierSurjectivity),
        exact_isomorphism: claims.contains(&MorphismClaim::ExactIsomorphism),
    })
}

fn validate_section_claims(raw: &[SectionClaim]) -> Result<SectionClaimSet, ToolError> {
    let claims: BTreeSet<_> = raw.iter().copied().collect();
    if claims.len() != raw.len() {
        return Err(ToolError::invalid("section claims must be unique"));
    }
    Ok(SectionClaimSet {
        as_morphism: claims.contains(&SectionClaim::AsMorphism),
        two_sided_inverse: claims.contains(&SectionClaim::TwoSidedInverse),
    })
}

fn index_section(
    definition: &SectionDefinition,
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> Result<IndexedSection, ToolError> {
    Ok(IndexedSection {
        state_map: index_section_map(&definition.state_map, source, target)?,
        claims: validate_section_claims(&definition.claims)?,
    })
}

fn index_forward_map(
    entries: &[MorphismMapEntry],
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> Result<Vec<usize>, ToolError> {
    let source_ids = state_indexes(source);
    let target_ids = state_indexes(target);
    let mut result = vec![None; source.states.len()];
    for entry in entries {
        if entry.source_state.is_empty() || entry.target_state.is_empty() {
            return Err(ToolError::invalid("state-map IDs must not be empty"));
        }
        let source_index = source_ids
            .get(entry.source_state.as_str())
            .copied()
            .ok_or_else(|| {
                ToolError::invalid(format!(
                    "morphism map names unknown source state {:?}",
                    entry.source_state
                ))
            })?;
        let target_index = target_ids
            .get(entry.target_state.as_str())
            .copied()
            .ok_or_else(|| {
                ToolError::invalid(format!(
                    "morphism map names unknown target state {:?}",
                    entry.target_state
                ))
            })?;
        if result[source_index].replace(target_index).is_some() {
            return Err(ToolError::invalid(format!(
                "morphism map repeats source state {:?}",
                entry.source_state
            )));
        }
    }
    complete_map(result, source, "morphism")
}

fn index_section_map(
    entries: &[SectionMapEntry],
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> Result<Vec<usize>, ToolError> {
    let source_ids = state_indexes(source);
    let target_ids = state_indexes(target);
    let mut result = vec![None; target.states.len()];
    for entry in entries {
        if entry.source_state.is_empty() || entry.target_state.is_empty() {
            return Err(ToolError::invalid("section-map IDs must not be empty"));
        }
        let target_index = target_ids
            .get(entry.target_state.as_str())
            .copied()
            .ok_or_else(|| {
                ToolError::invalid(format!(
                    "section map names unknown target state {:?}",
                    entry.target_state
                ))
            })?;
        let source_index = source_ids
            .get(entry.source_state.as_str())
            .copied()
            .ok_or_else(|| {
                ToolError::invalid(format!(
                    "section map names unknown source state {:?}",
                    entry.source_state
                ))
            })?;
        if result[target_index].replace(source_index).is_some() {
            return Err(ToolError::invalid(format!(
                "section map repeats target state {:?}",
                entry.target_state
            )));
        }
    }
    complete_map(result, target, "section")
}

fn complete_map(
    raw: Vec<Option<usize>>,
    domain: &IndexedMachine,
    map_name: &str,
) -> Result<Vec<usize>, ToolError> {
    let mut missing: Vec<String> = raw
        .iter()
        .enumerate()
        .filter(|(_, target)| target.is_none())
        .map(|(index, _)| domain.states[index].id.clone())
        .collect();
    missing.sort();
    if !missing.is_empty() {
        return Err(ToolError::invalid(format!(
            "{map_name} map must contain exactly one entry for every domain state; missing {missing:?}"
        )));
    }
    raw.into_iter()
        .map(|target| {
            target.ok_or_else(|| ToolError::internal("validated total map contains a hole"))
        })
        .collect()
}

fn state_indexes(machine: &IndexedMachine) -> BTreeMap<&str, usize> {
    machine
        .states
        .iter()
        .enumerate()
        .map(|(index, state)| (state.id.as_str(), index))
        .collect()
}

fn check_direction(
    domain: &IndexedMachine,
    codomain: &IndexedMachine,
    state_map: &[usize],
    scope: &MorphismScope,
    direction: &'static str,
    phase: &'static str,
) -> Result<MorphismLawResult, ToolError> {
    match scope {
        MorphismScope::AllStates => Ok(check_all_states(domain, codomain, state_map, direction)),
        MorphismScope::ReachableUnderContext { context, limits } => {
            let indexed_context = index_context(context.as_ref(), &domain.actions)?;
            let (graph, access) =
                build_product_with_access(domain, &indexed_context, limits.max_product_states)
                    .map_err(|error| resource_error_for_phase(error, phase))?;
            Ok(check_reachable(
                domain,
                codomain,
                state_map,
                &indexed_context,
                &graph,
                &access,
                direction,
            ))
        }
    }
}

fn check_all_states(
    domain: &IndexedMachine,
    codomain: &IndexedMachine,
    state_map: &[usize],
    direction: &'static str,
) -> MorphismLawResult {
    if state_map[domain.initial] != codomain.initial {
        return failed_law(
            direction,
            domain.states.len(),
            Vec::new(),
            initial_witness(domain, codomain, state_map, None),
        );
    }

    let mut ordered: Vec<usize> = (0..domain.states.len()).collect();
    ordered.sort_by(|left, right| domain.states[*left].id.cmp(&domain.states[*right].id));
    for domain_index in ordered {
        let codomain_index = state_map[domain_index];
        let domain_state = &domain.states[domain_index];
        let codomain_state = &codomain.states[codomain_index];
        if domain_state.observation != codomain_state.observation {
            return failed_law(
                direction,
                domain.states.len(),
                Vec::new(),
                observation_witness(
                    domain_state,
                    codomain_state,
                    None,
                    None,
                    "canonical_all_state_coordinate",
                ),
            );
        }
        for (action_index, action) in domain.actions.iter().enumerate() {
            let domain_transition = &domain_state.transitions[action_index];
            let codomain_transition = &codomain_state.transitions[action_index];
            if domain_transition.emission != codomain_transition.emission {
                return failed_law(
                    direction,
                    domain.states.len(),
                    Vec::new(),
                    emission_witness(
                        domain_state,
                        codomain_state,
                        action,
                        &domain_transition.emission,
                        &codomain_transition.emission,
                        None,
                        None,
                        "canonical_all_state_coordinate",
                    ),
                );
            }
            let mapped_successor = state_map[domain_transition.to];
            if mapped_successor != codomain_transition.to {
                return failed_law(
                    direction,
                    domain.states.len(),
                    Vec::new(),
                    commutation_witness(
                        domain_state,
                        codomain_state,
                        action,
                        &codomain.states[mapped_successor].id,
                        &codomain.states[codomain_transition.to].id,
                        None,
                        None,
                        "canonical_all_state_coordinate",
                    ),
                );
            }
        }
    }

    MorphismLawResult {
        direction,
        holds: true,
        checked_scoped_states: domain.states.len(),
        unchecked_domain_states: Vec::new(),
        witness: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn check_reachable(
    domain: &IndexedMachine,
    codomain: &IndexedMachine,
    state_map: &[usize],
    context: &IndexedContext,
    graph: &ProductGraph,
    access: &ProductAccess,
    direction: &'static str,
) -> MorphismLawResult {
    let unchecked = unchecked_states(domain, graph);
    if let Some(witness) = reachable_initial_witness(domain, codomain, state_map, context) {
        return failed_law(direction, graph.nodes.len(), unchecked, witness);
    }

    for (node_index, node) in graph.nodes.iter().enumerate() {
        let domain_state = &domain.states[node.machine];
        let codomain_state = &codomain.states[state_map[node.machine]];
        for (action_index, transition) in graph.transitions[node_index].iter().enumerate() {
            let Some((successor_node, _)) = transition else {
                continue;
            };
            let action = &domain.actions[action_index];
            let successor = graph.nodes[*successor_node];
            let successor_domain = &domain.states[successor.machine];
            let successor_codomain = &codomain.states[state_map[successor.machine]];
            if successor_domain.observation != successor_codomain.observation {
                return failed_law(
                    direction,
                    graph.nodes.len(),
                    unchecked,
                    observation_witness(
                        successor_domain,
                        successor_codomain,
                        Some(extended_access_word(
                            access,
                            node_index,
                            &domain.actions,
                            action,
                        )),
                        Some(context.states[successor.context].id.clone()),
                        "shortest_reachable_word",
                    ),
                );
            }

            let domain_transition = &domain_state.transitions[action_index];
            let codomain_transition = &codomain_state.transitions[action_index];
            if domain_transition.emission != codomain_transition.emission {
                return failed_law(
                    direction,
                    graph.nodes.len(),
                    unchecked,
                    emission_witness(
                        domain_state,
                        codomain_state,
                        action,
                        &domain_transition.emission,
                        &codomain_transition.emission,
                        Some(extended_access_word(
                            access,
                            node_index,
                            &domain.actions,
                            action,
                        )),
                        Some(context.states[node.context].id.clone()),
                        "shortest_reachable_word",
                    ),
                );
            }
            let mapped_successor = state_map[domain_transition.to];
            if mapped_successor != codomain_transition.to {
                return failed_law(
                    direction,
                    graph.nodes.len(),
                    unchecked,
                    commutation_witness(
                        domain_state,
                        codomain_state,
                        action,
                        &codomain.states[mapped_successor].id,
                        &codomain.states[codomain_transition.to].id,
                        Some(extended_access_word(
                            access,
                            node_index,
                            &domain.actions,
                            action,
                        )),
                        Some(context.states[node.context].id.clone()),
                        "shortest_reachable_word",
                    ),
                );
            }
        }
    }

    MorphismLawResult {
        direction,
        holds: true,
        checked_scoped_states: graph.nodes.len(),
        unchecked_domain_states: unchecked,
        witness: None,
    }
}

fn extended_access_word(
    access: &ProductAccess,
    node_index: usize,
    actions: &[String],
    final_action: &str,
) -> Vec<String> {
    let mut word = access.access_word(node_index, actions);
    word.push(final_action.to_owned());
    word
}

fn reachable_initial_witness(
    domain: &IndexedMachine,
    codomain: &IndexedMachine,
    state_map: &[usize],
    context: &IndexedContext,
) -> Option<MorphismWitness> {
    let context_state = Some(context.states[context.initial].id.clone());
    if state_map[domain.initial] != codomain.initial {
        return Some(initial_witness(domain, codomain, state_map, context_state));
    }
    let initial_domain = &domain.states[domain.initial];
    let initial_codomain = &codomain.states[state_map[domain.initial]];
    (initial_domain.observation != initial_codomain.observation).then(|| {
        observation_witness(
            initial_domain,
            initial_codomain,
            Some(Vec::new()),
            context_state,
            "shortest_reachable_word",
        )
    })
}

fn failed_law(
    direction: &'static str,
    checked_scoped_states: usize,
    unchecked_domain_states: Vec<String>,
    witness: MorphismWitness,
) -> MorphismLawResult {
    MorphismLawResult {
        direction,
        holds: false,
        checked_scoped_states,
        unchecked_domain_states,
        witness: Some(witness),
    }
}

fn initial_witness(
    domain: &IndexedMachine,
    codomain: &IndexedMachine,
    state_map: &[usize],
    context_state: Option<String>,
) -> MorphismWitness {
    MorphismWitness {
        basis: "initial_state",
        word: Some(Vec::new()),
        length_actions: Some(0),
        law: "initial_preservation",
        domain_state: domain.states[domain.initial].id.clone(),
        codomain_state: codomain.states[state_map[domain.initial]].id.clone(),
        context_state,
        action: None,
        left: WitnessValue {
            expression: "map(domain.initial)",
            value: codomain.states[state_map[domain.initial]].id.clone(),
        },
        right: WitnessValue {
            expression: "codomain.initial",
            value: codomain.states[codomain.initial].id.clone(),
        },
    }
}

fn observation_witness(
    domain_state: &crate::lts::IndexedState,
    codomain_state: &crate::lts::IndexedState,
    word: Option<Vec<String>>,
    context_state: Option<String>,
    basis: &'static str,
) -> MorphismWitness {
    let length_actions = word.as_ref().map(Vec::len);
    MorphismWitness {
        basis,
        word,
        length_actions,
        law: "observation_preservation",
        domain_state: domain_state.id.clone(),
        codomain_state: codomain_state.id.clone(),
        context_state,
        action: None,
        left: WitnessValue {
            expression: "domain.observation",
            value: domain_state.observation.clone(),
        },
        right: WitnessValue {
            expression: "codomain.observation_after_map",
            value: codomain_state.observation.clone(),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn emission_witness(
    domain_state: &crate::lts::IndexedState,
    codomain_state: &crate::lts::IndexedState,
    action: &str,
    domain_emission: &str,
    codomain_emission: &str,
    word: Option<Vec<String>>,
    context_state: Option<String>,
    basis: &'static str,
) -> MorphismWitness {
    let length_actions = word.as_ref().map(Vec::len);
    MorphismWitness {
        basis,
        word,
        length_actions,
        law: "emission_preservation",
        domain_state: domain_state.id.clone(),
        codomain_state: codomain_state.id.clone(),
        context_state,
        action: Some(action.to_owned()),
        left: WitnessValue {
            expression: "domain.emission",
            value: domain_emission.to_owned(),
        },
        right: WitnessValue {
            expression: "codomain.emission_after_map",
            value: codomain_emission.to_owned(),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn commutation_witness(
    domain_state: &crate::lts::IndexedState,
    codomain_state: &crate::lts::IndexedState,
    action: &str,
    mapped_domain_successor: &str,
    codomain_successor: &str,
    word: Option<Vec<String>>,
    context_state: Option<String>,
    basis: &'static str,
) -> MorphismWitness {
    let length_actions = word.as_ref().map(Vec::len);
    MorphismWitness {
        basis,
        word,
        length_actions,
        law: "transition_commutation",
        domain_state: domain_state.id.clone(),
        codomain_state: codomain_state.id.clone(),
        context_state,
        action: Some(action.to_owned()),
        left: WitnessValue {
            expression: "map(domain.successor)",
            value: mapped_domain_successor.to_owned(),
        },
        right: WitnessValue {
            expression: "codomain.successor_after_map",
            value: codomain_successor.to_owned(),
        },
    }
}

fn unchecked_states(domain: &IndexedMachine, graph: &ProductGraph) -> Vec<String> {
    let reached: BTreeSet<usize> = graph.nodes.iter().map(|node| node.machine).collect();
    let mut unchecked: Vec<String> = domain
        .states
        .iter()
        .enumerate()
        .filter(|(index, _)| !reached.contains(index))
        .map(|(_, state)| state.id.clone())
        .collect();
    unchecked.sort();
    unchecked
}

fn carrier_surjectivity(forward: &[usize], target: &IndexedMachine) -> SurjectivityResult {
    let image: BTreeSet<usize> = forward.iter().copied().collect();
    let mut target_order: Vec<usize> = (0..target.states.len()).collect();
    target_order.sort_by(|left, right| target.states[*left].id.cmp(&target.states[*right].id));
    let uncovered_target = target_order
        .into_iter()
        .find(|index| !image.contains(index))
        .map(|index| target.states[index].id.clone());
    SurjectivityResult {
        holds: uncovered_target.is_none(),
        uncovered_target,
    }
}

fn carrier_injectivity(
    forward: &[usize],
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> InjectivityResult {
    let mut fibres = BTreeMap::<String, Vec<String>>::new();
    for (source_index, target_index) in forward.iter().copied().enumerate() {
        fibres
            .entry(target.states[target_index].id.clone())
            .or_default()
            .push(source.states[source_index].id.clone());
    }
    for sources in fibres.values_mut() {
        sources.sort();
    }
    let collision = fibres
        .into_iter()
        .find(|(_, sources)| sources.len() > 1)
        .map(|(target_state, sources)| InjectivityCollision {
            target_state,
            source_states: [sources[0].clone(), sources[1].clone()],
        });
    InjectivityResult {
        holds: collision.is_none(),
        collision,
    }
}

fn check_section(
    section: &IndexedSection,
    source: &IndexedMachine,
    target: &IndexedMachine,
    forward: &[usize],
    scope: &MorphismScope,
) -> Result<SectionResult, ToolError> {
    let right_inverse = right_inverse(forward, &section.state_map, source, target);
    let as_morphism = section
        .claims
        .as_morphism
        .then(|| {
            check_direction(
                target,
                source,
                &section.state_map,
                scope,
                "target_to_source",
                "section_as_morphism",
            )
        })
        .transpose()?;
    let two_sided_inverse = section.claims.two_sided_inverse.then(|| {
        let left_inverse = left_inverse(forward, &section.state_map, source, target);
        TwoSidedInverseResult {
            holds: right_inverse.holds && left_inverse.holds,
            right_inverse_holds: right_inverse.holds,
            left_inverse,
        }
    });
    Ok(SectionResult {
        right_inverse,
        as_morphism,
        two_sided_inverse,
    })
}

fn right_inverse(
    forward: &[usize],
    reverse: &[usize],
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> InverseLawResult {
    let mut ordered: Vec<usize> = (0..target.states.len()).collect();
    ordered.sort_by(|left, right| target.states[*left].id.cmp(&target.states[*right].id));
    for target_index in ordered {
        let source_index = reverse[target_index];
        let actual_target = forward[source_index];
        if actual_target != target_index {
            return InverseLawResult {
                holds: false,
                witness: Some(InverseWitness {
                    law: "right_inverse",
                    state: target.states[target_index].id.clone(),
                    mapped_state: source.states[source_index].id.clone(),
                    expected_state: target.states[target_index].id.clone(),
                    actual_state: target.states[actual_target].id.clone(),
                }),
            };
        }
    }
    InverseLawResult {
        holds: true,
        witness: None,
    }
}

fn left_inverse(
    forward: &[usize],
    reverse: &[usize],
    source: &IndexedMachine,
    target: &IndexedMachine,
) -> InverseLawResult {
    let mut ordered: Vec<usize> = (0..source.states.len()).collect();
    ordered.sort_by(|left, right| source.states[*left].id.cmp(&source.states[*right].id));
    for source_index in ordered {
        let target_index = forward[source_index];
        let actual_source = reverse[target_index];
        if actual_source != source_index {
            return InverseLawResult {
                holds: false,
                witness: Some(InverseWitness {
                    law: "left_inverse",
                    state: source.states[source_index].id.clone(),
                    mapped_state: target.states[target_index].id.clone(),
                    expected_state: source.states[source_index].id.clone(),
                    actual_state: source.states[actual_source].id.clone(),
                }),
            };
        }
    }
    InverseLawResult {
        holds: true,
        witness: None,
    }
}

fn resource_error_for_phase(error: ToolError, phase: &str) -> ToolError {
    if error.code == "resource_limit" {
        ToolError::resource_limit(format!("{phase}: {}", error.message))
    } else {
        error
    }
}

fn scope_name(scope: &MorphismScope) -> &'static str {
    match scope {
        MorphismScope::AllStates => "all_states",
        MorphismScope::ReachableUnderContext { .. } => "reachable_under_context",
    }
}

fn morphism_nonclaims() -> Vec<&'static str> {
    vec![
        "The result is exact only for the supplied finite deterministic machines, maps, scope, context, and bound.",
        "A reachable-under-context result checks the lifted map on the reachable contextual product and makes no claim about unchecked machine states or disabled actions.",
        "Carrier surjectivity and section inverse laws are global set-theoretic claims; they do not strengthen a reachable-only behavioral theorem.",
        "The checker does not infer maps, simulations, action renamings, source-program semantics, liveness, fairness, timing, causality, or unmodeled effects.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lts::{ContextState, State, Transition};

    fn state(id: &str, observation: &str, transitions: &[(&str, &str, &str)]) -> State {
        State {
            id: id.to_owned(),
            observation: observation.to_owned(),
            transitions: transitions
                .iter()
                .map(|(action, to, emission)| {
                    (
                        (*action).to_owned(),
                        Transition {
                            to: (*to).to_owned(),
                            emission: (*emission).to_owned(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn machine(initial: &str, actions: &[&str], states: Vec<State>) -> Machine {
        Machine {
            initial: initial.to_owned(),
            actions: actions.iter().map(|action| (*action).to_owned()).collect(),
            states,
        }
    }

    fn forward(entries: &[(&str, &str)]) -> Vec<MorphismMapEntry> {
        entries
            .iter()
            .map(|(source_state, target_state)| MorphismMapEntry {
                source_state: (*source_state).to_owned(),
                target_state: (*target_state).to_owned(),
            })
            .collect()
    }

    fn reverse(entries: &[(&str, &str)]) -> Vec<SectionMapEntry> {
        entries
            .iter()
            .map(|(target_state, source_state)| SectionMapEntry {
                target_state: (*target_state).to_owned(),
                source_state: (*source_state).to_owned(),
            })
            .collect()
    }

    fn all_states_input(
        source: Machine,
        target: Machine,
        state_map: Vec<MorphismMapEntry>,
    ) -> MorphismInput {
        MorphismInput {
            schema_version: MORPHISM_SCHEMA.to_owned(),
            morphism: MorphismDefinition {
                source,
                target,
                state_map,
            },
            scope: MorphismScope::AllStates,
            claims: Vec::new(),
            section: None,
        }
    }

    fn reachable_input(
        source: Machine,
        target: Machine,
        state_map: Vec<MorphismMapEntry>,
        context: Option<ContextMonitor>,
        max_product_states: usize,
    ) -> MorphismInput {
        MorphismInput {
            schema_version: MORPHISM_SCHEMA.to_owned(),
            morphism: MorphismDefinition {
                source,
                target,
                state_map,
            },
            scope: MorphismScope::ReachableUnderContext {
                context,
                limits: MorphismLimits { max_product_states },
            },
            claims: Vec::new(),
            section: None,
        }
    }

    fn isomorphic_pair() -> (Machine, Machine, Vec<MorphismMapEntry>) {
        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "zero", &[("a", "s1", "up")]),
                state("s1", "one", &[("a", "s0", "down")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "zero", &[("a", "t1", "up")]),
                state("t1", "one", &[("a", "t0", "down")]),
            ],
        );
        (source, target, forward(&[("s0", "t0"), ("s1", "t1")]))
    }

    #[test]
    fn exact_isomorphism_and_supplied_inverse_are_separate_green_claims() {
        let (source, target, state_map) = isomorphic_pair();
        let mut input = all_states_input(source, target, state_map);
        input.claims = vec![
            MorphismClaim::ExactIsomorphism,
            MorphismClaim::CarrierSurjectivity,
        ];
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::AsMorphism, SectionClaim::TwoSidedInverse],
        });

        let result = check(&input).expect("isomorphism must check");
        assert!(result.morphism.holds);
        assert!(
            result
                .carrier_surjectivity
                .as_ref()
                .is_some_and(|claim| claim.holds)
        );
        assert!(
            result
                .exact_isomorphism
                .as_ref()
                .is_some_and(|claim| claim.holds)
        );
        let section = result.section.expect("section must be reported");
        assert!(section.right_inverse.holds);
        assert!(section.as_morphism.is_some_and(|claim| claim.holds));
        assert!(section.two_sided_inverse.is_some_and(|claim| claim.holds));
    }

    #[test]
    fn reachable_scope_ignores_unreachable_and_disabled_laws_but_all_states_does_not() {
        let source = machine(
            "live",
            &["b", "a"],
            vec![
                state(
                    "live",
                    "ok",
                    &[("a", "live", "step"), ("b", "bad", "source-only")],
                ),
                state("bad", "source-bad", &[("a", "bad", "x"), ("b", "bad", "x")]),
            ],
        );
        let target = machine(
            "only",
            &["a", "b"],
            vec![state(
                "only",
                "ok",
                &[("a", "only", "step"), ("b", "only", "target-only")],
            )],
        );
        let state_map = forward(&[("live", "only"), ("bad", "only")]);
        let context = ContextMonitor {
            initial: "permit-a".to_owned(),
            states: vec![ContextState {
                id: "permit-a".to_owned(),
                transitions: BTreeMap::from([("a".to_owned(), "permit-a".to_owned())]),
            }],
        };
        let reachable = reachable_input(
            source.clone(),
            target.clone(),
            state_map.clone(),
            Some(context),
            4,
        );
        let reachable_result = check(&reachable).expect("reachable check must run");
        assert!(reachable_result.morphism.holds);
        assert_eq!(
            reachable_result.morphism.unchecked_domain_states,
            vec!["bad"]
        );

        let all = all_states_input(source, target, state_map);
        let all_result = check(&all).expect("all-state check must run");
        assert!(!all_result.morphism.holds);
        let witness = all_result.morphism.witness.expect("failure has witness");
        assert_eq!(witness.basis, "canonical_all_state_coordinate");
        assert_eq!(witness.domain_state, "bad");
        assert!(witness.word.is_none());
    }

    #[test]
    fn initial_preservation_failure_has_empty_word() {
        let source = machine("s0", &["a"], vec![state("s0", "x", &[("a", "s0", "e")])]);
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "x", &[("a", "t0", "e")]),
                state("t1", "x", &[("a", "t1", "e")]),
            ],
        );
        let input = reachable_input(source, target, forward(&[("s0", "t1")]), None, 2);
        let result = check(&input).expect("semantic failure is an analysis");
        let witness = result.morphism.witness.expect("failure has witness");
        assert_eq!(witness.law, "initial_preservation");
        assert_eq!(witness.word, Some(Vec::new()));
    }

    #[test]
    fn successor_observation_precedes_edge_laws_at_the_same_word() {
        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "start", &[("a", "s1", "source-emission")]),
                state("s1", "source-observation", &[("a", "s1", "e")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "start", &[("a", "t1", "target-emission")]),
                state("t1", "target-observation", &[("a", "t1", "e")]),
            ],
        );
        let input = reachable_input(
            source,
            target,
            forward(&[("s0", "t0"), ("s1", "t1")]),
            None,
            4,
        );
        let result = check(&input).expect("semantic failure is an analysis");
        let witness = result.morphism.witness.expect("failure has witness");
        assert_eq!(witness.law, "observation_preservation");
        assert_eq!(witness.word, Some(vec!["a".to_owned()]));
    }

    #[test]
    fn sorted_actions_select_the_lexical_shortest_edge_failure() {
        let source = machine(
            "s0",
            &["b", "a"],
            vec![state(
                "s0",
                "same",
                &[("a", "s0", "bad-a"), ("b", "s0", "bad-b")],
            )],
        );
        let target = machine(
            "t0",
            &["a", "b"],
            vec![state(
                "t0",
                "same",
                &[("a", "t0", "good-a"), ("b", "t0", "good-b")],
            )],
        );
        let input = reachable_input(source, target, forward(&[("s0", "t0")]), None, 2);
        let result = check(&input).expect("semantic failure is an analysis");
        let witness = result.morphism.witness.expect("failure has witness");
        assert_eq!(witness.law, "emission_preservation");
        assert_eq!(witness.word, Some(vec!["a".to_owned()]));
    }

    #[test]
    fn shorter_observation_witness_beats_a_lexically_earlier_deeper_emission() {
        let source = machine(
            "s0",
            &["a", "b"],
            vec![
                state("s0", "root", &[("a", "s1", "ok"), ("b", "s2", "ok")]),
                state("s1", "same", &[("a", "s1", "bad"), ("b", "s1", "ok")]),
                state("s2", "source-diff", &[("a", "s2", "ok"), ("b", "s2", "ok")]),
            ],
        );
        let target = machine(
            "t0",
            &["a", "b"],
            vec![
                state("t0", "root", &[("a", "t1", "ok"), ("b", "t2", "ok")]),
                state("t1", "same", &[("a", "t1", "good"), ("b", "t1", "ok")]),
                state("t2", "target-diff", &[("a", "t2", "ok"), ("b", "t2", "ok")]),
            ],
        );
        let input = reachable_input(
            source,
            target,
            forward(&[("s0", "t0"), ("s1", "t1"), ("s2", "t2")]),
            None,
            4,
        );
        let witness = check(&input)
            .expect("semantic failure is an analysis")
            .morphism
            .witness
            .expect("failure has witness");
        assert_eq!(witness.law, "observation_preservation");
        assert_eq!(witness.word, Some(vec!["b".to_owned()]));
    }

    #[test]
    fn emission_precedes_commutation_at_the_same_word() {
        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "same", &[("a", "s1", "source")]),
                state("s1", "same", &[("a", "s1", "same")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "same", &[("a", "t0", "target")]),
                state("t1", "same", &[("a", "t1", "same")]),
            ],
        );
        let input = reachable_input(
            source,
            target,
            forward(&[("s0", "t0"), ("s1", "t1")]),
            None,
            3,
        );
        let witness = check(&input)
            .expect("semantic failure is an analysis")
            .morphism
            .witness
            .expect("failure has witness");
        assert_eq!(witness.law, "emission_preservation");
        assert_eq!(witness.word, Some(vec!["a".to_owned()]));
    }

    #[test]
    fn commutation_failure_is_distinct_from_observation_and_emission() {
        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "same", &[("a", "s1", "same")]),
                state("s1", "same", &[("a", "s1", "same")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "same", &[("a", "t0", "same")]),
                state("t1", "same", &[("a", "t1", "same")]),
            ],
        );
        let input = reachable_input(
            source,
            target,
            forward(&[("s0", "t0"), ("s1", "t1")]),
            None,
            3,
        );
        let result = check(&input).expect("semantic failure is an analysis");
        let witness = result.morphism.witness.expect("failure has witness");
        assert_eq!(witness.law, "transition_commutation");
        assert_eq!(witness.word, Some(vec!["a".to_owned()]));
        assert_eq!(witness.left.value, "t1");
        assert_eq!(witness.right.value, "t0");
    }

    #[test]
    fn product_bound_is_unknown_below_closure_and_exact_at_closure() {
        let (source, target, state_map) = isomorphic_pair();
        let below = reachable_input(source.clone(), target.clone(), state_map.clone(), None, 1);
        let error = check(&below).expect_err("truncated traversal must be unknown");
        assert_eq!(error.code, "resource_limit");
        assert!(error.message.starts_with("base_morphism:"));

        let exact = reachable_input(source, target, state_map, None, 2);
        let result = check(&exact).expect("exact closure at bound must pass");
        assert!(result.morphism.holds);
        assert_eq!(result.morphism.checked_scoped_states, 2);
    }

    #[test]
    fn zero_reachable_bound_is_invalid() {
        let (source, target, state_map) = isomorphic_pair();
        let input = reachable_input(source, target, state_map, None, 0);
        let error = check(&input).expect_err("zero bound must be invalid");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn carrier_surjectivity_is_global_and_reports_lexical_target() {
        let source = machine(
            "s0",
            &["a"],
            vec![state("s0", "same", &[("a", "s0", "same")])],
        );
        let target = machine(
            "z",
            &["a"],
            vec![
                state("z", "same", &[("a", "z", "same")]),
                state("b", "same", &[("a", "b", "same")]),
                state("a", "same", &[("a", "a", "same")]),
            ],
        );
        let mut input = reachable_input(source, target, forward(&[("s0", "z")]), None, 2);
        input.claims = vec![MorphismClaim::CarrierSurjectivity];
        let result = check(&input).expect("surjectivity must check");
        assert!(result.morphism.holds);
        let claim = result
            .carrier_surjectivity
            .expect("surjectivity must be reported");
        assert!(!claim.holds);
        assert_eq!(claim.uncovered_target.as_deref(), Some("a"));
    }

    #[test]
    fn global_surjectivity_may_use_a_state_unchecked_by_reachable_laws() {
        let source = machine(
            "live",
            &["a"],
            vec![
                state("live", "zero", &[("a", "live", "stay")]),
                state("hidden", "one", &[("a", "hidden", "hidden")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "zero", &[("a", "t0", "stay")]),
                state("t1", "different", &[("a", "t1", "different")]),
            ],
        );
        let mut input = reachable_input(
            source,
            target,
            forward(&[("live", "t0"), ("hidden", "t1")]),
            None,
            2,
        );
        input.claims = vec![MorphismClaim::CarrierSurjectivity];
        let result = check(&input).expect("separate claims must check");
        assert!(result.morphism.holds);
        assert_eq!(result.morphism.unchecked_domain_states, vec!["hidden"]);
        assert!(result.carrier_surjectivity.is_some_and(|claim| claim.holds));
    }

    #[test]
    fn section_right_inverse_as_morphism_and_two_sided_are_independent() {
        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "zero", &[("a", "u1", "step")]),
                state("u1", "one", &[("a", "u1", "stay")]),
                state("u2", "one", &[("a", "u1", "stay")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "zero", &[("a", "t1", "step")]),
                state("t1", "one", &[("a", "t1", "stay")]),
            ],
        );
        let mut input = all_states_input(
            source,
            target,
            forward(&[("s0", "t0"), ("u1", "t1"), ("u2", "t1")]),
        );
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "u2")]),
            claims: vec![SectionClaim::AsMorphism, SectionClaim::TwoSidedInverse],
        });
        let result = check(&input).expect("section claims must check");
        assert!(result.morphism.holds);
        let section = result.section.expect("section must be reported");
        assert!(section.right_inverse.holds);
        assert!(!section.as_morphism.expect("requested").holds);
        let two_sided = section.two_sided_inverse.expect("requested");
        assert!(!two_sided.holds);
        assert_eq!(
            two_sided
                .left_inverse
                .witness
                .as_ref()
                .map(|witness| witness.state.as_str()),
            Some("u1")
        );
    }

    #[test]
    fn right_inverse_failure_reports_least_target() {
        let source = machine(
            "s0",
            &["a"],
            vec![state("s0", "same", &[("a", "s0", "same")])],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "same", &[("a", "t0", "same")]),
                state("a", "same", &[("a", "a", "same")]),
            ],
        );
        let mut input = all_states_input(source, target, forward(&[("s0", "t0")]));
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("a", "s0")]),
            claims: Vec::new(),
        });
        let result = check(&input).expect("right inverse must check");
        let witness = result
            .section
            .expect("section must be reported")
            .right_inverse
            .witness
            .expect("failure has witness");
        assert_eq!(witness.state, "a");
        assert_eq!(witness.actual_state, "t0");
    }

    #[test]
    fn reverse_reachable_traversal_has_its_own_bound_phase() {
        let (source, target, state_map) = isomorphic_pair();
        let mut input = reachable_input(source, target, state_map, None, 1);
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::AsMorphism],
        });
        let error = check(&input).expect_err("base traversal itself exceeds one state");
        assert!(error.message.starts_with("base_morphism:"));

        let source = machine(
            "s0",
            &["a"],
            vec![
                state("s0", "same", &[("a", "s0", "same")]),
                state("s1", "same", &[("a", "s1", "same")]),
            ],
        );
        let target = machine(
            "t0",
            &["a"],
            vec![
                state("t0", "same", &[("a", "t1", "same")]),
                state("t1", "same", &[("a", "t1", "same")]),
            ],
        );
        let mut reverse_limited = reachable_input(
            source,
            target,
            forward(&[("s0", "t0"), ("s1", "t1")]),
            Some(ContextMonitor {
                initial: "c".to_owned(),
                states: vec![ContextState {
                    id: "c".to_owned(),
                    transitions: BTreeMap::from([("a".to_owned(), "c".to_owned())]),
                }],
            }),
            1,
        );
        reverse_limited.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::AsMorphism],
        });
        let error = check(&reverse_limited).expect_err("reverse traversal must exhaust");
        assert!(error.message.starts_with("section_as_morphism:"));

        let (source, target, state_map) = isomorphic_pair();
        let mut exact = reachable_input(source, target, state_map, None, 2);
        exact.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::AsMorphism],
        });
        let result = check(&exact).expect("both traversals close exactly at two");
        assert_eq!(result.morphism.checked_scoped_states, 2);
        assert_eq!(
            result
                .section
                .and_then(|section| section.as_morphism)
                .map(|claim| claim.checked_scoped_states),
            Some(2)
        );
    }

    #[test]
    fn invalid_section_structure_precedes_product_exhaustion() {
        let (source, target, state_map) = isomorphic_pair();
        let mut input = reachable_input(source, target, state_map, None, 1);
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0")]),
            claims: vec![SectionClaim::AsMorphism],
        });
        let error = check(&input).expect_err("invalid total map must win before traversal");
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("missing [\"t1\"]"));
    }

    #[test]
    fn exact_isomorphism_rejects_reachable_scope() {
        let (source, target, state_map) = isomorphic_pair();
        let mut input = reachable_input(source, target, state_map, None, 2);
        input.claims = vec![MorphismClaim::ExactIsomorphism];
        let error = check(&input).expect_err("reachable exact isomorphism is not defined");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn noninjective_exact_claim_has_canonical_collision() {
        let source = machine(
            "a",
            &["x"],
            vec![
                state("b", "same", &[("x", "b", "same")]),
                state("a", "same", &[("x", "a", "same")]),
            ],
        );
        let target = machine("t", &["x"], vec![state("t", "same", &[("x", "t", "same")])]);
        let mut input = all_states_input(source, target, forward(&[("b", "t"), ("a", "t")]));
        input.claims = vec![MorphismClaim::ExactIsomorphism];
        let result = check(&input).expect("exact claim must check");
        let exact = result.exact_isomorphism.expect("claim must be reported");
        assert!(!exact.holds);
        let collision = exact.injectivity.collision.expect("collision required");
        assert_eq!(collision.target_state, "t");
        assert_eq!(collision.source_states, ["a", "b"]);
    }

    #[test]
    fn nonsurjective_exact_claim_reports_its_global_component() {
        let source = machine("s", &["x"], vec![state("s", "same", &[("x", "s", "same")])]);
        let target = machine(
            "t0",
            &["x"],
            vec![
                state("t0", "same", &[("x", "t0", "same")]),
                state("t1", "other", &[("x", "t1", "other")]),
            ],
        );
        let mut input = all_states_input(source, target, forward(&[("s", "t0")]));
        input.claims = vec![MorphismClaim::ExactIsomorphism];
        let exact = check(&input)
            .expect("exact claim must check")
            .exact_isomorphism
            .expect("claim must be reported");
        assert!(exact.morphism_holds);
        assert!(exact.injectivity.holds);
        assert!(!exact.surjectivity.holds);
        assert_eq!(exact.surjectivity.uncovered_target.as_deref(), Some("t1"));
        assert!(!exact.holds);
    }

    #[test]
    fn map_and_claim_structure_is_strict() {
        let (source, target, state_map) = isomorphic_pair();
        let mut missing = all_states_input(source.clone(), target.clone(), state_map.clone());
        missing.morphism.state_map.pop();
        assert_eq!(
            check(&missing).expect_err("missing map entry").code,
            "invalid_input"
        );

        let mut duplicate = all_states_input(source.clone(), target.clone(), state_map.clone());
        duplicate.morphism.state_map.push(MorphismMapEntry {
            source_state: "s0".to_owned(),
            target_state: "t0".to_owned(),
        });
        assert_eq!(
            check(&duplicate).expect_err("duplicate map entry").code,
            "invalid_input"
        );

        let mut duplicate_claim = all_states_input(source, target, state_map);
        duplicate_claim.claims = vec![
            MorphismClaim::CarrierSurjectivity,
            MorphismClaim::CarrierSurjectivity,
        ];
        assert_eq!(
            check(&duplicate_claim).expect_err("duplicate claim").code,
            "invalid_input"
        );

        let (source, target, state_map) = isomorphic_pair();
        let mut unknown = all_states_input(source, target, state_map);
        unknown.morphism.state_map[0].target_state = "ghost".to_owned();
        assert_eq!(
            check(&unknown).expect_err("unknown target state").code,
            "invalid_input"
        );

        let (source, target, state_map) = isomorphic_pair();
        let mut unknown_source = all_states_input(source, target, state_map);
        unknown_source.morphism.state_map[0].source_state = "ghost".to_owned();
        assert_eq!(
            check(&unknown_source)
                .expect_err("unknown source state")
                .code,
            "invalid_input"
        );

        let (source, target, state_map) = isomorphic_pair();
        let mut empty = all_states_input(source, target, state_map);
        empty.morphism.state_map[0].target_state.clear();
        assert_eq!(
            check(&empty).expect_err("empty map state").code,
            "invalid_input"
        );
    }

    #[test]
    fn schema_and_action_alphabet_are_strict() {
        let (source, mut target, state_map) = isomorphic_pair();
        let mut wrong_schema = all_states_input(source.clone(), target.clone(), state_map.clone());
        wrong_schema.schema_version = "kerf.morphism.v0".to_owned();
        assert_eq!(
            check(&wrong_schema).expect_err("wrong schema").code,
            "invalid_input"
        );

        target.actions = vec!["other".to_owned()];
        for state in &mut target.states {
            let transition = state
                .transitions
                .remove("a")
                .expect("fixture action exists");
            state.transitions.insert("other".to_owned(), transition);
        }
        let mismatch = all_states_input(source, target, state_map);
        assert_eq!(
            check(&mismatch).expect_err("alphabet mismatch").code,
            "invalid_input"
        );
    }

    #[test]
    fn declaration_order_does_not_change_result_json() {
        let (source, target, state_map) = isomorphic_pair();
        let mut input = all_states_input(source, target, state_map);
        input.claims = vec![
            MorphismClaim::ExactIsomorphism,
            MorphismClaim::CarrierSurjectivity,
        ];
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::TwoSidedInverse, SectionClaim::AsMorphism],
        });
        let first = serde_json::to_value(check(&input).expect("first input must check"))
            .expect("result must serialize");

        input.morphism.source.states.reverse();
        input.morphism.source.actions.reverse();
        input.morphism.target.states.reverse();
        input.morphism.target.actions.reverse();
        input.morphism.state_map.reverse();
        input.claims.reverse();
        let section = input.section.as_mut().expect("fixture has a section");
        section.state_map.reverse();
        section.claims.reverse();
        let second = serde_json::to_value(check(&input).expect("reordered input must check"))
            .expect("result must serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn reachable_context_declaration_order_does_not_change_result_json() {
        let (source, target, state_map) = isomorphic_pair();
        let context = ContextMonitor {
            initial: "c0".to_owned(),
            states: vec![
                ContextState {
                    id: "c0".to_owned(),
                    transitions: BTreeMap::from([("a".to_owned(), "c1".to_owned())]),
                },
                ContextState {
                    id: "c1".to_owned(),
                    transitions: BTreeMap::from([("a".to_owned(), "c0".to_owned())]),
                },
            ],
        };
        let mut input = reachable_input(source, target, state_map, Some(context), 4);
        input.claims = vec![MorphismClaim::CarrierSurjectivity];
        input.section = Some(SectionDefinition {
            state_map: reverse(&[("t0", "s0"), ("t1", "s1")]),
            claims: vec![SectionClaim::AsMorphism, SectionClaim::TwoSidedInverse],
        });
        let first = serde_json::to_value(check(&input).expect("first input must check"))
            .expect("result must serialize");

        input.morphism.source.states.reverse();
        input.morphism.target.states.reverse();
        input.morphism.state_map.reverse();
        if let MorphismScope::ReachableUnderContext {
            context: Some(context),
            ..
        } = &mut input.scope
        {
            context.states.reverse();
        } else {
            panic!("fixture has a reachable context");
        }
        let section = input.section.as_mut().expect("fixture has a section");
        section.state_map.reverse();
        section.claims.reverse();
        let second = serde_json::to_value(check(&input).expect("reordered input must check"))
            .expect("result must serialize");
        assert_eq!(first, second);
    }
}
