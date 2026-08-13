use crate::report::ToolError;
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TRANSPORT_SCHEMA: &str = "kerf.transport.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportInput {
    pub schema_version: String,
    pub workload_id: String,
    pub rate_unit: String,
    pub weighted_hop_traffic_unit: String,
    pub resources: Vec<String>,
    pub links: Vec<Link>,
    pub commodities: Vec<Commodity>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub id: String,
    pub from: String,
    pub to: String,
    pub capacity: RationalInput,
    pub latency_ns: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commodity {
    pub id: String,
    pub source: String,
    pub sink: String,
    pub demand: RationalInput,
    pub route: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalInput {
    pub num: String,
    pub den: String,
}

#[derive(Debug, Serialize)]
pub struct TransportResult {
    pub algorithm: &'static str,
    pub workload_id: String,
    pub rate_unit_label: String,
    pub link_loads: Vec<LinkLoad>,
    pub overloaded_links: Vec<String>,
    pub maximum_utilization: RationalOutput,
    pub weighted_hop_traffic: MeasuredRational,
    pub demand_weighted_link_latency: MeasuredRational,
    pub maximum_route_latency_ns_without_queueing: String,
    pub maximum_route_hops: usize,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct LinkLoad {
    pub id: String,
    pub load: RationalOutput,
    pub capacity: RationalOutput,
    pub utilization: RationalOutput,
    pub overloaded: bool,
}

#[derive(Debug, Serialize)]
pub struct MeasuredRational {
    pub value: RationalOutput,
    pub unit_label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RationalOutput {
    pub num: String,
    pub den: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rational {
    num: BigUint,
    den: BigUint,
}

impl Rational {
    fn zero() -> Self {
        Self {
            num: BigUint::zero(),
            den: BigUint::from(1_u8),
        }
    }

    fn new(num: BigUint, den: BigUint, field: &str) -> Result<Self, ToolError> {
        if den.is_zero() {
            return Err(ToolError::invalid(format!(
                "{field} denominator must be positive"
            )));
        }
        let divisor = num.gcd(&den);
        Ok(Self {
            num: num / &divisor,
            den: den / divisor,
        })
    }

    fn from_input(input: &RationalInput, field: &str) -> Result<Self, ToolError> {
        let num = parse_biguint(&input.num, &format!("{field}.num"))?;
        let den = parse_biguint(&input.den, &format!("{field}.den"))?;
        Self::new(num, den, field)
    }

    fn add(&self, other: &Self) -> Self {
        let shared = self.den.gcd(&other.den);
        let left_multiplier = &other.den / &shared;
        let right_multiplier = &self.den / &shared;
        let num = &self.num * &left_multiplier + &other.num * &right_multiplier;
        let den = &self.den * left_multiplier;
        Self::new(num, den, "rational sum").expect("positive denominators stay positive")
    }

    fn mul_integer(&self, factor: &BigUint) -> Self {
        Self::new(&self.num * factor, self.den.clone(), "rational product")
            .expect("positive denominator stays positive")
    }

    fn div(&self, other: &Self, field: &str) -> Result<Self, ToolError> {
        if other.num.is_zero() {
            return Err(ToolError::invalid(format!("cannot divide {field} by zero")));
        }
        let left_gcd = self.num.gcd(&other.num);
        let right_gcd = other.den.gcd(&self.den);
        let num = (&self.num / &left_gcd) * (&other.den / &right_gcd);
        let den = (&self.den / &right_gcd) * (&other.num / &left_gcd);
        Self::new(num, den, field)
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.num * &other.den).cmp(&(&other.num * &self.den))
    }

    fn output(&self) -> RationalOutput {
        RationalOutput {
            num: self.num.to_string(),
            den: self.den.to_string(),
        }
    }
}

/// Validate and score exact static unsplittable routes.
///
/// # Errors
///
/// Returns [`ToolError`] for invalid topology, routes, units, rational values,
/// or invalid exact rational values.
#[allow(clippy::too_many_lines)]
pub fn score(input: &TransportInput) -> Result<TransportResult, ToolError> {
    if input.schema_version != TRANSPORT_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {:?}",
            input.schema_version, TRANSPORT_SCHEMA
        )));
    }
    require_nonempty("workload_id", &input.workload_id)?;
    require_nonempty("rate_unit", &input.rate_unit)?;
    require_nonempty(
        "weighted_hop_traffic_unit",
        &input.weighted_hop_traffic_unit,
    )?;
    let resources = unique_ids("resources", &input.resources)?;
    if input.links.is_empty() {
        return Err(ToolError::invalid("links must not be empty"));
    }

    let mut link_indexes = BTreeMap::new();
    let mut capacities = Vec::with_capacity(input.links.len());
    let mut latencies = Vec::with_capacity(input.links.len());
    for (index, link) in input.links.iter().enumerate() {
        require_nonempty("link ID", &link.id)?;
        if link_indexes.insert(link.id.as_str(), index).is_some() {
            return Err(ToolError::invalid(format!(
                "duplicate link ID {:?}",
                link.id
            )));
        }
        if !resources.contains(link.from.as_str()) || !resources.contains(link.to.as_str()) {
            return Err(ToolError::invalid(format!(
                "link {:?} references an unknown resource",
                link.id
            )));
        }
        let capacity = Rational::from_input(&link.capacity, &format!("link {} capacity", link.id))?;
        if capacity.num.is_zero() {
            return Err(ToolError::invalid(format!(
                "link {:?} capacity must be positive",
                link.id
            )));
        }
        capacities.push(capacity);
        latencies.push(parse_biguint(
            &link.latency_ns,
            &format!("link {} latency_ns", link.id),
        )?);
    }

    let mut commodity_ids = BTreeSet::new();
    let mut loads = vec![Rational::zero(); input.links.len()];
    let mut weighted_hop_traffic = Rational::zero();
    let mut demand_weighted_latency = Rational::zero();
    let mut max_latency = BigUint::zero();
    let mut max_hops = 0_usize;
    for commodity in &input.commodities {
        require_nonempty("commodity ID", &commodity.id)?;
        if !commodity_ids.insert(commodity.id.as_str()) {
            return Err(ToolError::invalid(format!(
                "duplicate commodity ID {:?}",
                commodity.id
            )));
        }
        if !resources.contains(commodity.source.as_str())
            || !resources.contains(commodity.sink.as_str())
        {
            return Err(ToolError::invalid(format!(
                "commodity {:?} references an unknown source or sink",
                commodity.id
            )));
        }
        let demand = Rational::from_input(
            &commodity.demand,
            &format!("commodity {} demand", commodity.id),
        )?;
        if commodity.source == commodity.sink {
            if !commodity.route.is_empty() {
                return Err(ToolError::invalid(format!(
                    "commodity {:?} has identical source and sink and therefore requires an empty route",
                    commodity.id
                )));
            }
            continue;
        }
        if commodity.route.is_empty() {
            return Err(ToolError::invalid(format!(
                "commodity {:?} requires a non-empty route",
                commodity.id
            )));
        }
        let mut route_seen = BTreeSet::new();
        let mut resource_seen = BTreeSet::from([commodity.source.as_str()]);
        let mut expected_source = commodity.source.as_str();
        let mut route_latency = BigUint::zero();
        for link_id in &commodity.route {
            let link_index = link_indexes.get(link_id.as_str()).copied().ok_or_else(|| {
                ToolError::invalid(format!(
                    "commodity {:?} route references unknown link {:?}",
                    commodity.id, link_id
                ))
            })?;
            if !route_seen.insert(link_id.as_str()) {
                return Err(ToolError::invalid(format!(
                    "commodity {:?} route repeats link {:?}; cyclic routes are not supported",
                    commodity.id, link_id
                )));
            }
            let link = &input.links[link_index];
            if link.from != expected_source {
                return Err(ToolError::invalid(format!(
                    "commodity {:?} route is not contiguous at link {:?}",
                    commodity.id, link.id
                )));
            }
            expected_source = &link.to;
            if !resource_seen.insert(expected_source) {
                return Err(ToolError::invalid(format!(
                    "commodity {:?} route revisits resource {:?}; cyclic routes are not supported",
                    commodity.id, expected_source
                )));
            }
            loads[link_index] = loads[link_index].add(&demand);
            route_latency += &latencies[link_index];
        }
        if expected_source != commodity.sink {
            return Err(ToolError::invalid(format!(
                "commodity {:?} route ends at {:?}, not declared sink {:?}",
                commodity.id, expected_source, commodity.sink
            )));
        }
        weighted_hop_traffic =
            weighted_hop_traffic.add(&demand.mul_integer(&BigUint::from(commodity.route.len())));
        demand_weighted_latency = demand_weighted_latency.add(&demand.mul_integer(&route_latency));
        max_latency = max_latency.max(route_latency);
        max_hops = max_hops.max(commodity.route.len());
    }

    let mut maximum_utilization = Rational::zero();
    let mut overloaded_links = Vec::new();
    let mut link_loads = Vec::with_capacity(input.links.len());
    for (index, link) in input.links.iter().enumerate() {
        let utilization = loads[index].div(&capacities[index], "link utilization")?;
        let overloaded = loads[index].cmp(&capacities[index]).is_gt();
        if overloaded {
            overloaded_links.push(link.id.clone());
        }
        if utilization.cmp(&maximum_utilization).is_gt() {
            maximum_utilization = utilization.clone();
        }
        link_loads.push(LinkLoad {
            id: link.id.clone(),
            load: loads[index].output(),
            capacity: capacities[index].output(),
            utilization: utilization.output(),
            overloaded,
        });
    }
    overloaded_links.sort();
    link_loads.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(TransportResult {
        algorithm: "exact_static_unsplittable_route_accounting_v1",
        workload_id: input.workload_id.clone(),
        rate_unit_label: input.rate_unit.clone(),
        link_loads,
        overloaded_links,
        maximum_utilization: maximum_utilization.output(),
        weighted_hop_traffic: MeasuredRational {
            value: weighted_hop_traffic.output(),
            unit_label: input.weighted_hop_traffic_unit.clone(),
        },
        demand_weighted_link_latency: MeasuredRational {
            value: demand_weighted_latency.output(),
            unit_label: format!("({})*nanoseconds", input.rate_unit),
        },
        maximum_route_latency_ns_without_queueing: max_latency.to_string(),
        maximum_route_hops: max_hops,
        nonclaims: vec![
            "Static average route load does not prove queue stability, burst tolerance, or latency under contention.",
            "Link latency is summed without queueing, arbitration, overlap, pipelining, or backpressure.",
            "Unit strings are opaque user-declared labels; dimensional consistency is not verified.",
            "The result does not infer cache, NUMA, coherence, physical energy, or runtime behavior.",
        ],
    })
}

fn unique_ids<'a>(kind: &str, values: &'a [String]) -> Result<BTreeSet<&'a str>, ToolError> {
    if values.is_empty() {
        return Err(ToolError::invalid(format!("{kind} must not be empty")));
    }
    let mut result = BTreeSet::new();
    for value in values {
        require_nonempty(kind, value)?;
        if !result.insert(value.as_str()) {
            return Err(ToolError::invalid(format!("duplicate {kind} ID {value:?}")));
        }
    }
    Ok(result)
}

fn require_nonempty(field: &str, value: &str) -> Result<(), ToolError> {
    if value.is_empty() {
        Err(ToolError::invalid(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn parse_biguint(value: &str, field: &str) -> Result<BigUint, ToolError> {
    value.parse::<BigUint>().map_err(|_| {
        ToolError::invalid(format!(
            "{field} value {value:?} must be an unsigned base-10 integer string"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(num: u128, den: u128) -> RationalInput {
        RationalInput {
            num: num.to_string(),
            den: den.to_string(),
        }
    }

    fn input() -> TransportInput {
        TransportInput {
            schema_version: TRANSPORT_SCHEMA.to_owned(),
            workload_id: "tick".to_owned(),
            rate_unit: "bytes_per_second".to_owned(),
            weighted_hop_traffic_unit: "byte_hops_per_second".to_owned(),
            resources: vec!["cpu".to_owned(), "llc".to_owned(), "dram".to_owned()],
            links: vec![
                Link {
                    id: "cpu_llc".to_owned(),
                    from: "cpu".to_owned(),
                    to: "llc".to_owned(),
                    capacity: rational(100, 1),
                    latency_ns: "10".to_owned(),
                },
                Link {
                    id: "llc_dram".to_owned(),
                    from: "llc".to_owned(),
                    to: "dram".to_owned(),
                    capacity: rational(50, 1),
                    latency_ns: "30".to_owned(),
                },
            ],
            commodities: vec![Commodity {
                id: "read".to_owned(),
                source: "cpu".to_owned(),
                sink: "dram".to_owned(),
                demand: rational(60, 1),
                route: vec!["cpu_llc".to_owned(), "llc_dram".to_owned()],
            }],
        }
    }

    #[test]
    fn route_load_is_exact() {
        let result = score(&input()).expect("valid routes");
        assert_eq!(result.overloaded_links, vec!["llc_dram"]);
        assert_eq!(
            result.weighted_hop_traffic.value,
            RationalOutput {
                num: "120".to_owned(),
                den: "1".to_owned()
            }
        );
    }

    #[test]
    fn discontinuous_route_is_rejected() {
        let mut input = input();
        input.commodities[0].route.reverse();
        let error = score(&input).expect_err("route must be contiguous");
        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn reducible_intermediate_products_do_not_overflow() {
        let mut input = input();
        let maximum = u128::MAX.to_string();
        input.commodities[0].demand = RationalInput {
            num: "2".to_owned(),
            den: maximum.clone(),
        };
        input.links[0].latency_ns = maximum;
        input.commodities[0].route = vec!["cpu_llc".to_owned()];
        input.commodities[0].sink = "llc".to_owned();
        let result = score(&input).expect("exact result is representable");
        assert_eq!(
            result.demand_weighted_link_latency.value,
            RationalOutput {
                num: "2".to_owned(),
                den: "1".to_owned(),
            }
        );
    }

    #[test]
    fn large_rational_comparison_does_not_overflow() {
        let mut input = input();
        let numerator = (u128::MAX / 2).to_string();
        input.commodities[0].demand = RationalInput {
            num: numerator.clone(),
            den: "2".to_owned(),
        };
        input.links[0].capacity = RationalInput {
            num: numerator,
            den: "3".to_owned(),
        };
        input.commodities[0].route = vec!["cpu_llc".to_owned()];
        input.commodities[0].sink = "llc".to_owned();
        let result = score(&input).expect("exact comparison must complete");
        assert_eq!(
            result.maximum_utilization,
            RationalOutput {
                num: "3".to_owned(),
                den: "2".to_owned(),
            }
        );
        assert_eq!(result.overloaded_links, ["cpu_llc"]);
    }

    #[test]
    fn values_beyond_u128_remain_exact_and_units_remain_labels() {
        let mut input = input();
        let huge = "1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890";
        input.commodities[0].demand = RationalInput {
            num: huge.to_owned(),
            den: "1".to_owned(),
        };
        input.links[0].capacity = RationalInput {
            num: huge.to_owned(),
            den: "1".to_owned(),
        };
        input.commodities[0].route = vec!["cpu_llc".to_owned()];
        input.commodities[0].sink = "llc".to_owned();
        let result = score(&input).expect("arbitrary-precision case must complete");
        assert_eq!(result.weighted_hop_traffic.value.num, huge);
        assert_eq!(
            result.weighted_hop_traffic.unit_label,
            "byte_hops_per_second"
        );
        assert!(
            result
                .nonclaims
                .iter()
                .any(|claim| claim.contains("opaque user-declared labels"))
        );
    }

    #[test]
    fn distinct_links_cannot_hide_a_resource_cycle() {
        let mut input = input();
        input.links.push(Link {
            id: "dram_llc".to_owned(),
            from: "dram".to_owned(),
            to: "llc".to_owned(),
            capacity: rational(50, 1),
            latency_ns: "30".to_owned(),
        });
        input.commodities[0].sink = "llc".to_owned();
        input.commodities[0].route.push("dram_llc".to_owned());
        let error = score(&input).expect_err("resource cycle must be rejected");
        assert_eq!(error.code, "invalid_input");
    }
}
