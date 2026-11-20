use std::cmp::Ordering;

use crate::{OverlayId, RouteCandidate};

/// Explicit policy used to rank eligible service endpoints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionPolicy {
    /// Prefer the lowest measured round-trip time.
    LowestLatency,
    /// Prefer the highest measured throughput.
    HighestThroughput,
    /// Prefer the lowest measured packet loss.
    LowestPacketLoss,
    /// Prefer one overlay, then use latency and other metrics as tie-breakers.
    PreferNetwork(OverlayId),
}

impl SelectionPolicy {
    /// Selects the best eligible candidate, or None when no candidate is usable.
    ///
    /// Measurements that are absent are ranked behind known measurements.
    /// Ties are resolved deterministically by endpoint identity.
    pub fn select<'a>(&self, candidates: &'a [RouteCandidate]) -> Option<&'a RouteCandidate> {
        candidates
            .iter()
            .filter(|candidate| candidate.is_eligible())
            .min_by(|left, right| self.compare(left, right))
    }

    fn compare(&self, left: &RouteCandidate, right: &RouteCandidate) -> Ordering {
        left.state
            .cmp(&right.state)
            .then_with(|| self.compare_primary(left, right))
            .then_with(|| {
                compare_optional_ascending(
                    left.metrics.round_trip_time.as_ref(),
                    right.metrics.round_trip_time.as_ref(),
                )
            })
            .then_with(|| {
                compare_optional_ascending(
                    left.metrics.packet_loss_ppm.as_ref(),
                    right.metrics.packet_loss_ppm.as_ref(),
                )
            })
            .then_with(|| {
                compare_optional_descending(
                    left.metrics.throughput_bps.as_ref(),
                    right.metrics.throughput_bps.as_ref(),
                )
            })
            .then_with(|| {
                compare_optional_ascending(
                    left.metrics.jitter.as_ref(),
                    right.metrics.jitter.as_ref(),
                )
            })
            .then_with(|| left.endpoint.cmp(&right.endpoint))
    }

    fn compare_primary(&self, left: &RouteCandidate, right: &RouteCandidate) -> Ordering {
        match self {
            Self::LowestLatency => compare_optional_ascending(
                left.metrics.round_trip_time.as_ref(),
                right.metrics.round_trip_time.as_ref(),
            ),
            Self::HighestThroughput => compare_optional_descending(
                left.metrics.throughput_bps.as_ref(),
                right.metrics.throughput_bps.as_ref(),
            ),
            Self::LowestPacketLoss => compare_optional_ascending(
                left.metrics.packet_loss_ppm.as_ref(),
                right.metrics.packet_loss_ppm.as_ref(),
            ),
            Self::PreferNetwork(preferred) => {
                let left_matches = &left.endpoint.network == preferred;
                let right_matches = &right.endpoint.network == preferred;
                right_matches.cmp(&left_matches)
            }
        }
    }
}

fn compare_optional_ascending<T: Ord>(left: Option<&T>, right: Option<&T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_descending<T: Ord>(left: Option<&T>, right: Option<&T>) -> Ordering {
    compare_optional_ascending(right, left)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};
    use std::time::Duration;

    use crate::{
        OverlayEndpoint, OverlayId, RouteCandidate, RouteMetrics, RouteState, SelectionPolicy,
    };

    fn candidate(
        network: OverlayId,
        last_segment: u16,
        state: RouteState,
        latency_ms: Option<u64>,
        packet_loss_ppm: Option<u32>,
        throughput_bps: Option<u64>,
    ) -> RouteCandidate {
        RouteCandidate {
            endpoint: OverlayEndpoint {
                network,
                address: IpAddr::V6(Ipv6Addr::new(
                    0x200, 0, 0, 0, 0, 0, 0, last_segment,
                )),
                port: Some(443),
            },
            state,
            metrics: RouteMetrics {
                round_trip_time: latency_ms.map(Duration::from_millis),
                jitter: None,
                packet_loss_ppm,
                throughput_bps,
            },
        }
    }

    #[test]
    fn lowest_latency_selects_the_fastest_eligible_path() {
        let candidates = [
            candidate(OverlayId::Yggdrasil, 2, RouteState::Reachable, Some(40), Some(1_000), Some(10_000)),
            candidate(OverlayId::Cjdns, 1, RouteState::Reachable, Some(20), Some(2_000), Some(8_000)),
        ];

        let selected = SelectionPolicy::LowestLatency.select(&candidates).unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Cjdns);
        assert_eq!(selected.metrics.round_trip_time, Some(Duration::from_millis(20)));
    }

    #[test]
    fn unknown_and_unreachable_paths_are_excluded() {
        let candidates = [
            candidate(OverlayId::Cjdns, 1, RouteState::Unreachable, Some(1), Some(0), Some(1_000_000)),
            candidate(OverlayId::Yggdrasil, 2, RouteState::Unknown, Some(2), Some(0), Some(1_000_000)),
        ];

        assert!(SelectionPolicy::LowestLatency.select(&candidates).is_none());
    }

    #[test]
    fn reachable_path_ranks_before_degraded_even_when_slower() {
        let candidates = [
            candidate(OverlayId::Cjdns, 1, RouteState::Degraded, Some(1), Some(0), Some(1_000_000)),
            candidate(OverlayId::Yggdrasil, 2, RouteState::Reachable, Some(100), Some(10_000), Some(100_000)),
        ];

        let selected = SelectionPolicy::LowestLatency.select(&candidates).unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Yggdrasil);
    }

    #[test]
    fn highest_throughput_prefers_the_largest_known_measurement() {
        let candidates = [
            candidate(OverlayId::Cjdns, 1, RouteState::Reachable, Some(10), Some(0), Some(100)),
            candidate(OverlayId::Yggdrasil, 2, RouteState::Reachable, Some(50), Some(0), Some(900)),
        ];

        let selected = SelectionPolicy::HighestThroughput.select(&candidates).unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Yggdrasil);
    }

    #[test]
    fn preferred_network_wins_when_both_paths_are_healthy() {
        let candidates = [
            candidate(OverlayId::Cjdns, 1, RouteState::Reachable, Some(10), Some(0), Some(100)),
            candidate(OverlayId::Yggdrasil, 2, RouteState::Reachable, Some(100), Some(0), Some(900)),
        ];

        let selected = SelectionPolicy::PreferNetwork(OverlayId::Yggdrasil)
            .select(&candidates)
            .unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Yggdrasil);
    }

    #[test]
    fn invalid_packet_loss_measurements_are_rejected() {
        let candidate = candidate(
            OverlayId::Cjdns, 1, RouteState::Reachable, Some(1), Some(1_000_001), Some(1_000_000),
        );

        assert!(SelectionPolicy::LowestLatency.select(&[candidate]).is_none());
    }

    #[test]
    fn missing_primary_measurements_rank_after_known_measurements() {
        let candidates = [
            candidate(OverlayId::Cjdns, 1, RouteState::Reachable, None, Some(0), Some(1_000_000)),
            candidate(OverlayId::Yggdrasil, 2, RouteState::Reachable, Some(500), None, None),
        ];

        let selected = SelectionPolicy::LowestLatency.select(&candidates).unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Yggdrasil);
    }
}
