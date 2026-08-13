use crate::factor::{
    BoundaryRepresentation, FactorMapEntry, FactorSource, admit_partition, source_for_node,
};
use crate::lts::{
    ContextMonitor, Machine, build_product, index_context, index_machine, require_schema,
};
use crate::report::ToolError;
use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;

pub const FACTOR_SEARCH_SCHEMA: &str = "kerf.factor-search.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorSearchInput {
    pub schema_version: String,
    pub machine: Machine,
    #[serde(default)]
    pub context: Option<ContextMonitor>,
    pub limits: FactorSearchLimits,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FactorSearchLimits {
    pub max_boundary_states: usize,
    pub max_product_states: usize,
    pub max_candidates: usize,
}

#[derive(Debug, Serialize)]
pub struct FactorSearchResult {
    pub algorithm: &'static str,
    pub verdict: &'static str,
    pub context_retained: bool,
    pub reachable_product_states: usize,
    pub requested_max_boundary_states: usize,
    pub effective_max_boundary_states: usize,
    pub coverage: FactorSearchCoverage,
    pub minimum: Option<MinimumFactor>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorSearchCoverage {
    pub canonical_source_order: Vec<FactorSource>,
    pub evaluated_candidates: usize,
    pub completed_cardinalities: Vec<CardinalityCoverage>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CardinalityCoverage {
    pub boundary_state_count: usize,
    pub candidates_evaluated: usize,
    pub candidates_rejected: usize,
}

#[derive(Debug, Serialize)]
pub struct MinimumFactor {
    pub boundary_state_count: usize,
    pub candidate_ordinal_within_cardinality: usize,
    pub global_candidate_ordinal: usize,
    pub factor_map: Vec<FactorMapEntry>,
    pub representation: BoundaryRepresentation,
}

#[derive(Debug, Serialize)]
struct CandidateLimitDetails {
    schema_version: &'static str,
    algorithm: &'static str,
    reason: &'static str,
    context_retained: bool,
    reachable_product_states: usize,
    requested_max_boundary_states: usize,
    effective_max_boundary_states: usize,
    coverage: FactorSearchCoverage,
    current_cardinality: PartialCardinalityCoverage,
    minimum: Option<MinimumFactor>,
}

#[derive(Debug, Serialize)]
struct PartialCardinalityCoverage {
    boundary_state_count: usize,
    candidates_evaluated: usize,
    candidates_rejected: usize,
    complete: bool,
}

enum SearchStop {
    CandidateLimit,
    Minimum {
        representation: BoundaryRepresentation,
        factor_map: Vec<FactorMapEntry>,
        ordinal_within_cardinality: usize,
    },
}

#[derive(Clone, Copy)]
struct SearchMetadata {
    limits: FactorSearchLimits,
    reachable_product_states: usize,
    effective_max_boundary_states: usize,
}

struct SearchProgress {
    canonical_source_order: Vec<FactorSource>,
    evaluated_candidates: usize,
    completed_cardinalities: Vec<CardinalityCoverage>,
}

/// Exhaustively search bounded partitions of the reachable product for a lawful factor map.
///
/// # Errors
///
/// Returns [`ToolError`] when the schema, model, or limits are invalid. Product or
/// candidate-bound exhaustion returns a `resource_limit` error and therefore an
/// unknown result at the CLI boundary.
pub fn search(input: &FactorSearchInput) -> Result<FactorSearchResult, ToolError> {
    require_schema(&input.schema_version, FACTOR_SEARCH_SCHEMA)?;
    validate_limits(input.limits)?;

    let machine = index_machine(&input.machine)?;
    let context = index_context(input.context.as_ref(), &machine.actions)?;
    let graph = build_product(&machine, &context, input.limits.max_product_states)?;
    let reachable_product_states = graph.nodes.len();
    let effective_max_boundary_states = input
        .limits
        .max_boundary_states
        .min(reachable_product_states);
    let metadata = SearchMetadata {
        limits: input.limits,
        reachable_product_states,
        effective_max_boundary_states,
    };
    let canonical_source_order: Vec<FactorSource> = graph
        .nodes
        .iter()
        .map(|node| source_for_node(input.context.is_some(), node, &machine, &context))
        .collect();

    let mut evaluated_candidates = 0_usize;
    let mut completed_cardinalities = Vec::new();

    for boundary_state_count in 1..=effective_max_boundary_states {
        let mut evaluated_at_cardinality = 0_usize;
        let boundary_id_width = (boundary_state_count - 1).to_string().len();
        let outcome = enumerate_exact_partitions(
            reachable_product_states,
            boundary_state_count,
            &mut |labels| {
                if evaluated_candidates == input.limits.max_candidates {
                    return Ok(ControlFlow::Break(SearchStop::CandidateLimit));
                }

                evaluated_candidates += 1;
                evaluated_at_cardinality += 1;
                let boundary_by_node: Vec<String> = labels
                    .iter()
                    .map(|label| format!("z{label:0boundary_id_width$}"))
                    .collect();
                let representation =
                    admit_partition(&machine, input.context.as_ref(), &graph, &boundary_by_node)?;
                if let Some(representation) = representation {
                    Ok(ControlFlow::Break(SearchStop::Minimum {
                        representation,
                        factor_map: factor_map(&canonical_source_order, &boundary_by_node),
                        ordinal_within_cardinality: evaluated_at_cardinality,
                    }))
                } else {
                    Ok(ControlFlow::Continue(()))
                }
            },
        )?;

        match outcome {
            ControlFlow::Continue(()) => {
                completed_cardinalities.push(CardinalityCoverage {
                    boundary_state_count,
                    candidates_evaluated: evaluated_at_cardinality,
                    candidates_rejected: evaluated_at_cardinality,
                });
            }
            ControlFlow::Break(SearchStop::CandidateLimit) => {
                return Err(candidate_limit_error(
                    metadata,
                    SearchProgress {
                        canonical_source_order,
                        evaluated_candidates,
                        completed_cardinalities,
                    },
                    boundary_state_count,
                    evaluated_at_cardinality,
                )?);
            }
            ControlFlow::Break(SearchStop::Minimum {
                representation,
                factor_map,
                ordinal_within_cardinality,
            }) => {
                return Ok(minimum_result(
                    metadata,
                    SearchProgress {
                        canonical_source_order,
                        evaluated_candidates,
                        completed_cardinalities,
                    },
                    boundary_state_count,
                    ordinal_within_cardinality,
                    factor_map,
                    representation,
                ));
            }
        }
    }

    Ok(bounded_negative_result(
        metadata,
        SearchProgress {
            canonical_source_order,
            evaluated_candidates,
            completed_cardinalities,
        },
    ))
}

fn candidate_limit_error(
    metadata: SearchMetadata,
    progress: SearchProgress,
    boundary_state_count: usize,
    evaluated_at_cardinality: usize,
) -> Result<ToolError, ToolError> {
    let details = CandidateLimitDetails {
        schema_version: "kerf.factor-search-coverage.v1",
        algorithm: "restricted_growth_factor_search_v1",
        reason: "candidate_limit_before_cardinality_complete",
        context_retained: true,
        reachable_product_states: metadata.reachable_product_states,
        requested_max_boundary_states: metadata.limits.max_boundary_states,
        effective_max_boundary_states: metadata.effective_max_boundary_states,
        coverage: FactorSearchCoverage {
            canonical_source_order: progress.canonical_source_order,
            evaluated_candidates: progress.evaluated_candidates,
            completed_cardinalities: progress.completed_cardinalities,
        },
        current_cardinality: PartialCardinalityCoverage {
            boundary_state_count,
            candidates_evaluated: evaluated_at_cardinality,
            candidates_rejected: evaluated_at_cardinality,
            complete: false,
        },
        minimum: None,
    };
    let details = serde_json::to_value(details).map_err(|error| {
        ToolError::internal(format!(
            "could not serialize factor-search coverage: {error}"
        ))
    })?;
    Ok(ToolError::resource_limit_with_details(
        format!(
            "factor search reached max_candidates={} before completing boundary_state_count={boundary_state_count}",
            metadata.limits.max_candidates
        ),
        details,
    ))
}

fn minimum_result(
    metadata: SearchMetadata,
    progress: SearchProgress,
    boundary_state_count: usize,
    ordinal_within_cardinality: usize,
    factor_map: Vec<FactorMapEntry>,
    representation: BoundaryRepresentation,
) -> FactorSearchResult {
    FactorSearchResult {
        algorithm: "restricted_growth_factor_search_v1",
        verdict: "minimum_found",
        context_retained: true,
        reachable_product_states: metadata.reachable_product_states,
        requested_max_boundary_states: metadata.limits.max_boundary_states,
        effective_max_boundary_states: metadata.effective_max_boundary_states,
        coverage: FactorSearchCoverage {
            canonical_source_order: progress.canonical_source_order,
            evaluated_candidates: progress.evaluated_candidates,
            completed_cardinalities: progress.completed_cardinalities,
        },
        minimum: Some(MinimumFactor {
            boundary_state_count,
            candidate_ordinal_within_cardinality: ordinal_within_cardinality,
            global_candidate_ordinal: progress.evaluated_candidates,
            factor_map,
            representation,
        }),
        nonclaims: search_nonclaims(),
    }
}

fn bounded_negative_result(
    metadata: SearchMetadata,
    progress: SearchProgress,
) -> FactorSearchResult {
    FactorSearchResult {
        algorithm: "restricted_growth_factor_search_v1",
        verdict: "no_lawful_partition_within_boundary_limit",
        context_retained: true,
        reachable_product_states: metadata.reachable_product_states,
        requested_max_boundary_states: metadata.limits.max_boundary_states,
        effective_max_boundary_states: metadata.effective_max_boundary_states,
        coverage: FactorSearchCoverage {
            canonical_source_order: progress.canonical_source_order,
            evaluated_candidates: progress.evaluated_candidates,
            completed_cardinalities: progress.completed_cardinalities,
        },
        minimum: None,
        nonclaims: search_nonclaims(),
    }
}

fn validate_limits(limits: FactorSearchLimits) -> Result<(), ToolError> {
    if limits.max_boundary_states == 0
        || limits.max_product_states == 0
        || limits.max_candidates == 0
    {
        return Err(ToolError::invalid(
            "limits.max_boundary_states, limits.max_product_states, and limits.max_candidates must all be positive",
        ));
    }
    Ok(())
}

fn factor_map(sources: &[FactorSource], boundary_by_node: &[String]) -> Vec<FactorMapEntry> {
    sources
        .iter()
        .zip(boundary_by_node)
        .map(|(source, boundary_state)| FactorMapEntry {
            machine_state: source.machine_state.clone(),
            context_state: source.context_state.clone(),
            boundary_state: boundary_state.clone(),
        })
        .collect()
}

fn enumerate_exact_partitions<F>(
    element_count: usize,
    block_count: usize,
    visit: &mut F,
) -> Result<ControlFlow<SearchStop>, ToolError>
where
    F: FnMut(&[usize]) -> Result<ControlFlow<SearchStop>, ToolError>,
{
    if element_count == 0 || block_count == 0 || block_count > element_count {
        return Err(ToolError::internal(
            "restricted-growth enumeration received an invalid domain",
        ));
    }
    let mut labels = vec![0_usize; element_count];
    fill_minimum_completion(&mut labels, 1, 0, block_count);
    let mut prefix_maxima = vec![0_usize; element_count];
    loop {
        if let ControlFlow::Break(stop) = visit(&labels)? {
            return Ok(ControlFlow::Break(stop));
        }
        if !advance_exact_partition(&mut labels, block_count, &mut prefix_maxima) {
            return Ok(ControlFlow::Continue(()));
        }
    }
}

fn advance_exact_partition(
    labels: &mut [usize],
    block_count: usize,
    prefix_maxima: &mut [usize],
) -> bool {
    prefix_maxima[0] = labels[0];
    for position in 1..labels.len() {
        prefix_maxima[position] = prefix_maxima[position - 1].max(labels[position]);
    }

    for position in (1..labels.len()).rev() {
        let prefix_maximum = prefix_maxima[position - 1];
        let maximum_next_label = (prefix_maximum + 1).min(block_count - 1);
        for label in (labels[position] + 1)..=maximum_next_label {
            let next_maximum = prefix_maximum.max(label);
            let remaining_positions = labels.len() - position - 1;
            if next_maximum + 1 + remaining_positions >= block_count {
                labels[position] = label;
                fill_minimum_completion(labels, position + 1, next_maximum, block_count);
                return true;
            }
        }
    }
    false
}

fn fill_minimum_completion(
    labels: &mut [usize],
    start: usize,
    mut maximum_label: usize,
    block_count: usize,
) {
    for position in start..labels.len() {
        let remaining_positions = labels.len() - position - 1;
        if maximum_label + 1 + remaining_positions < block_count {
            maximum_label += 1;
            labels[position] = maximum_label;
        } else {
            labels[position] = 0;
        }
    }
}

fn search_nonclaims() -> Vec<&'static str> {
    vec![
        "The search is exact only for all set partitions of the supplied reachable finite machine-by-context product under the existing factor laws and declared bounds.",
        "A minimum is not a minimum interface for source code or a globally optimal real architecture.",
        "A bounded negative excludes only boundary cardinalities at or below effective_max_boundary_states.",
        "The candidate bound counts retained partitions evaluated; per-candidate generation and factor checking still scale with reachable product size.",
        "Transport, placement, scheduling, Pareto cost, migration, probabilistic behavior, and natural-system analogies are outside this search.",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::{FACTOR_SCHEMA, FactorInput, FactorLimits, check};
    use serde_json::{Value, json};
    use std::collections::BTreeSet;

    fn parse_input(value: Value) -> FactorSearchInput {
        serde_json::from_value(value).expect("test factor-search input must deserialize")
    }

    fn distinct_observations(state_count: usize, limits: &Value) -> FactorSearchInput {
        let states: Vec<Value> = (0..state_count)
            .map(|index| {
                let next = (index + 1).min(state_count - 1);
                json!({
                    "id": format!("s{index}"),
                    "observation": format!("o{index}"),
                    "transitions": {
                        "next": {"to": format!("s{next}"), "emission": "step"}
                    }
                })
            })
            .collect();
        parse_input(json!({
            "schema_version": FACTOR_SEARCH_SCHEMA,
            "machine": {
                "initial": "s0",
                "actions": ["next"],
                "states": states
            },
            "limits": limits
        }))
    }

    fn collected_partitions(element_count: usize, block_count: usize) -> Vec<Vec<usize>> {
        let mut partitions = Vec::new();
        let outcome = enumerate_exact_partitions(element_count, block_count, &mut |labels| {
            partitions.push(labels.to_vec());
            Ok(ControlFlow::Continue(()))
        })
        .expect("enumeration succeeds");
        assert!(matches!(outcome, ControlFlow::Continue(())));
        partitions
    }

    fn stirling_second_kind(element_count: usize, block_count: usize) -> usize {
        let mut counts = vec![vec![0_usize; block_count + 1]; element_count + 1];
        counts[0][0] = 1;
        for elements in 1..=element_count {
            for blocks in 1..=block_count.min(elements) {
                counts[elements][blocks] =
                    counts[elements - 1][blocks - 1] + blocks * counts[elements - 1][blocks];
            }
        }
        counts[element_count][block_count]
    }

    #[test]
    fn restricted_growth_enumeration_is_unique_lexical_and_has_stirling_counts() {
        assert_eq!(collected_partitions(3, 1), [vec![0, 0, 0]]);
        assert_eq!(
            collected_partitions(3, 2),
            [vec![0, 0, 1], vec![0, 1, 0], vec![0, 1, 1]]
        );
        assert_eq!(collected_partitions(3, 3), [vec![0, 1, 2]]);

        for elements in 1..=8 {
            let all: Vec<Vec<usize>> = (1..=elements)
                .flat_map(|blocks| {
                    let partitions = collected_partitions(elements, blocks);
                    assert_eq!(partitions.len(), stirling_second_kind(elements, blocks));
                    assert!(partitions.windows(2).all(|pair| pair[0] < pair[1]));
                    partitions
                })
                .collect();
            let unique: BTreeSet<_> = all.iter().collect();
            assert_eq!(all.len(), unique.len());
        }
        assert_eq!(
            (1..=4)
                .map(|blocks| collected_partitions(4, blocks).len())
                .collect::<Vec<_>>(),
            [1, 7, 6, 1]
        );
    }

    #[test]
    fn large_first_candidate_uses_bounded_iterative_generator_state() {
        let mut visits = 0;
        let outcome = enumerate_exact_partitions(100_000, 1, &mut |labels| {
            visits += 1;
            assert_eq!(labels.len(), 100_000);
            assert!(labels.iter().all(|label| *label == 0));
            Ok(ControlFlow::Break(SearchStop::CandidateLimit))
        })
        .expect("large first candidate does not recurse");
        assert!(matches!(
            outcome,
            ControlFlow::Break(SearchStop::CandidateLimit)
        ));
        assert_eq!(visits, 1);
    }

    #[test]
    fn exact_minimum_is_found_only_after_smaller_cardinalities_complete() {
        let input = distinct_observations(
            2,
            &json!({
                "max_boundary_states": 2,
                "max_product_states": 2,
                "max_candidates": 2
            }),
        );
        let result = search(&input).expect("search completes");
        assert_eq!(result.verdict, "minimum_found");
        assert_eq!(result.coverage.evaluated_candidates, 2);
        assert_eq!(
            result.coverage.completed_cardinalities,
            [CardinalityCoverage {
                boundary_state_count: 1,
                candidates_evaluated: 1,
                candidates_rejected: 1,
            }]
        );
        let minimum = result.minimum.expect("minimum exists");
        assert_eq!(minimum.boundary_state_count, 2);
        assert_eq!(minimum.candidate_ordinal_within_cardinality, 1);
        assert_eq!(minimum.global_candidate_ordinal, 2);
    }

    #[test]
    fn completed_cutoff_reports_exact_bounded_negative() {
        let input = distinct_observations(
            3,
            &json!({
                "max_boundary_states": 2,
                "max_product_states": 3,
                "max_candidates": 4
            }),
        );
        let result = search(&input).expect("search completes");
        assert_eq!(result.verdict, "no_lawful_partition_within_boundary_limit");
        assert!(result.minimum.is_none());
        assert_eq!(result.coverage.evaluated_candidates, 4);
        assert_eq!(
            result
                .coverage
                .completed_cardinalities
                .iter()
                .map(|entry| entry.candidates_evaluated)
                .collect::<Vec<_>>(),
            [1, 3]
        );
    }

    #[test]
    fn candidate_limit_unknown_reports_partial_coverage_without_a_minimum() {
        let input = distinct_observations(
            3,
            &json!({
                "max_boundary_states": 3,
                "max_product_states": 3,
                "max_candidates": 4
            }),
        );
        let error = search(&input).expect_err("next cardinality exceeds budget");
        assert_eq!(error.code, "resource_limit");
        let details = error.details.expect("partial coverage is retained");
        assert_eq!(details["minimum"], Value::Null);
        assert_eq!(details["coverage"]["evaluated_candidates"], 4);
        assert_eq!(details["current_cardinality"]["boundary_state_count"], 3);
        assert_eq!(details["current_cardinality"]["candidates_evaluated"], 0);
        assert_eq!(
            details["coverage"]["completed_cardinalities"][1]["candidates_evaluated"],
            3
        );
    }

    #[test]
    fn exact_budget_permits_the_successful_candidate() {
        let input = distinct_observations(
            3,
            &json!({
                "max_boundary_states": 3,
                "max_product_states": 3,
                "max_candidates": 5
            }),
        );
        let result = search(&input).expect("fifth candidate may be evaluated");
        let minimum = result.minimum.expect("identity partition is lawful");
        assert_eq!(minimum.boundary_state_count, 3);
        assert_eq!(minimum.global_candidate_ordinal, 5);
    }

    #[test]
    fn emitted_map_is_admitted_by_the_public_factor_checker() {
        let input = distinct_observations(
            2,
            &json!({
                "max_boundary_states": 2,
                "max_product_states": 2,
                "max_candidates": 2
            }),
        );
        let result = search(&input).expect("search completes");
        let minimum = result.minimum.expect("minimum exists");
        let checked = check(&FactorInput {
            schema_version: FACTOR_SCHEMA.to_owned(),
            machine: input.machine,
            context: input.context,
            factor_map: minimum.factor_map,
            limits: FactorLimits {
                max_product_states: input.limits.max_product_states,
            },
        })
        .expect("emitted map is valid input");
        assert!(checked.lawful);
        assert_eq!(checked.boundary_state_count, minimum.boundary_state_count);
    }

    #[test]
    fn product_limit_is_unknown_and_zero_limits_are_invalid() {
        let mut product_limited = distinct_observations(
            3,
            &json!({
                "max_boundary_states": 3,
                "max_product_states": 2,
                "max_candidates": 5
            }),
        );
        let error = search(&product_limited).expect_err("product is not silently rejected");
        assert_eq!(error.code, "resource_limit");

        for field in [
            "max_boundary_states",
            "max_product_states",
            "max_candidates",
        ] {
            product_limited.limits = FactorSearchLimits {
                max_boundary_states: 3,
                max_product_states: 3,
                max_candidates: 5,
            };
            match field {
                "max_boundary_states" => product_limited.limits.max_boundary_states = 0,
                "max_product_states" => product_limited.limits.max_product_states = 0,
                "max_candidates" => product_limited.limits.max_candidates = 0,
                _ => unreachable!(),
            }
            let error = search(&product_limited).expect_err("zero limit is invalid");
            assert_eq!(error.code, "invalid_input");
        }
    }

    #[test]
    fn declaration_order_does_not_change_search_output() {
        let original: FactorSearchInput =
            serde_json::from_str(include_str!("../self-host/process.factor-search.json"))
                .expect("self-host search input must deserialize");
        let mut reordered = original.clone();
        reordered.machine.actions.reverse();
        reordered.machine.states.reverse();
        reordered
            .context
            .as_mut()
            .expect("context is present")
            .states
            .reverse();
        assert_eq!(
            serde_json::to_value(search(&original).expect("original search"))
                .expect("result serializes"),
            serde_json::to_value(search(&reordered).expect("reordered search"))
                .expect("result serializes")
        );
    }

    #[test]
    fn self_host_process_map_has_exact_minimum_two_not_supplied_three() {
        let search_value: Value =
            serde_json::from_str(include_str!("../self-host/process.factor-search.json"))
                .expect("self-host search input must be JSON");
        let supplied_value: Value =
            serde_json::from_str(include_str!("../self-host/process.factor.json"))
                .expect("self-host supplied factor must be JSON");
        assert_eq!(search_value["machine"], supplied_value["machine"]);
        assert_eq!(search_value["context"], supplied_value["context"]);

        let search_input: FactorSearchInput =
            serde_json::from_value(search_value).expect("self-host search input must deserialize");
        let result = search(&search_input).expect("self-host search completes");
        let minimum = result.minimum.expect("minimum exists");
        assert_eq!(minimum.boundary_state_count, 2);
        assert_eq!(minimum.global_candidate_ordinal, 2);
        assert_eq!(
            minimum
                .factor_map
                .iter()
                .map(|entry| entry.boundary_state.as_str())
                .collect::<Vec<_>>(),
            ["z0", "z0", "z0", "z1"]
        );

        let supplied: FactorInput = serde_json::from_value(supplied_value)
            .expect("self-host supplied factor must deserialize");
        let supplied_result = check(&supplied).expect("supplied factor remains lawful");
        assert!(supplied_result.lawful);
        assert_eq!(supplied_result.boundary_state_count, 3);
    }
}
