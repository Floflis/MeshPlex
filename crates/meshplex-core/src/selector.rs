use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

use crate::{DEFAULT_MAX_MEASUREMENT_AGE, RouteCandidate, ServiceId};

/// Explicit policy used to rank eligible service endpoints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionPolicy {
    /// Prefer the lowest measured round-trip time.
    LowestLatency,
    /// Prefer the highest measured throughput.
    HighestThroughput,
    /// Prefer the lowest measured packet loss.
    LowestPacketLoss,
    /// Prefer one overlay, then use other metrics as tie-breakers.
    PreferNetwork(crate::OverlayId),
}

impl SelectionPolicy {
    /// Select the best fresh route for one service using the default 30-second age limit.
    pub fn select<'a>(
        &self,
        service_id: &ServiceId,
        candidates: &'a [RouteCandidate],
    ) -> Option<&'a RouteCandidate> {
        self.select_at(
            service_id,
            candidates,
            SystemTime::now(),
            DEFAULT_MAX_MEASUREMENT_AGE,
        )
    }

    /// Select at an explicit time and freshness limit, which also makes the policy
    /// deterministic and easy to test.
    pub fn select_at<'a>(
        &self,
        service_id: &ServiceId,
        candidates: &'a [RouteCandidate],
        now: SystemTime,
        max_measurement_age: Duration,
    ) -> Option<&'a RouteCandidate> {
        candidates
            .iter()
            .filter(|candidate| candidate.service_id == *service_id)
            .filter(|candidate| candidate.state.is_eligible())
            .filter(|candidate| candidate.metrics.is_valid())
            .filter(|candidate| candidate.metrics.is_fresh_at(now, max_measurement_age))
            .min_by(|left, right| self.compare(left, right))
    }

    fn compare(&self, left: &RouteCandidate, right: &RouteCandidate) -> Ordering {
        left.state
            .preference()
            .cmp(&right.state.preference())
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
                compare_optional_descending(
                    left.metrics.path_quality.as_ref(),
                    right.metrics.path_quality.as_ref(),
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
    use std::time::{Duration, SystemTime};

    use crate::{
        OverlayEndpoint, OverlayId, RouteCandidate, RouteMetrics, RouteState, SelectionPolicy,
        ServiceId,
    };

    fn service_id(key: &[u8]) -> ServiceId {
        ServiceId::from_public_key(key).unwrap()
    }

    fn candidate(
        service: ServiceId,
        network: OverlayId,
        last_segment: u16,
        state: RouteState,
        latency_ms: Option<u64>,
        packet_loss_ppm: Option<u32>,
        throughput_bps: Option<u64>,
        observed_at: Option<SystemTime>,
    ) -> RouteCandidate {
        RouteCandidate {
            service_id: service,
            endpoint: OverlayEndpoint {
                network,
                address: IpAddr::V6(Ipv6Addr::new(0x200, 0, 0, 0, 0, 0, 0, last_segment)),
                port: Some(443),
            },
            state,
            metrics: RouteMetrics {
                round_trip_time: latency_ms.map(Duration::from_millis),
                jitter: None,
                packet_loss_ppm,
                throughput_bps,
                path_quality: None,
                observed_at,
            },
        }
    }

    #[test]
    fn lowest_latency_selects_the_fastest_fresh_route_for_the_service() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"service A");
        let candidates = [
            candidate(
                service,
                OverlayId::Yggdrasil,
                2,
                RouteState::Reachable,
                Some(40),
                Some(1_000),
                Some(10_000),
                Some(now),
            ),
            candidate(
                service,
                OverlayId::Cjdns,
                1,
                RouteState::Reachable,
                Some(20),
                Some(2_000),
                Some(8_000),
                Some(now),
            ),
        ];

        let selected = SelectionPolicy::LowestLatency
            .select_at(&service, &candidates, now, Duration::from_secs(30))
            .unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Cjdns);
    }

    #[test]
    fn candidate_for_another_service_is_never_selected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let requested = service_id(b"requested");
        let other = service_id(b"other");
        let candidate = candidate(
            other,
            OverlayId::Cjdns,
            1,
            RouteState::Reachable,
            Some(1),
            Some(0),
            Some(1_000_000),
            Some(now),
        );

        assert!(
            SelectionPolicy::LowestLatency
                .select_at(&requested, &[candidate], now, Duration::from_secs(30))
                .is_none()
        );
    }

    #[test]
    fn stale_measurements_are_not_eligible() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"stale service");
        let candidate = candidate(
            service,
            OverlayId::Cjdns,
            1,
            RouteState::Reachable,
            Some(1),
            Some(0),
            Some(1_000_000),
            Some(now - Duration::from_secs(31)),
        );

        assert!(
            SelectionPolicy::LowestLatency
                .select_at(&service, &[candidate], now, Duration::from_secs(30))
                .is_none()
        );
    }

    #[test]
    fn future_timestamps_are_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"future service");
        let candidate = candidate(
            service,
            OverlayId::Cjdns,
            1,
            RouteState::Reachable,
            Some(1),
            Some(0),
            Some(1_000_000),
            Some(now + Duration::from_secs(1)),
        );

        assert!(
            SelectionPolicy::LowestLatency
                .select_at(&service, &[candidate], now, Duration::from_secs(30))
                .is_none()
        );
    }

    #[test]
    fn missing_measurement_timestamp_is_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"missing timestamp");
        let candidate = candidate(
            service,
            OverlayId::Cjdns,
            1,
            RouteState::Reachable,
            Some(1),
            Some(0),
            Some(1_000_000),
            None,
        );

        assert!(
            SelectionPolicy::LowestLatency
                .select_at(&service, &[candidate], now, Duration::from_secs(30))
                .is_none()
        );
    }

    #[test]
    fn reachable_path_ranks_before_routed_even_when_routed_is_faster() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"route states");
        let candidates = [
            candidate(
                service,
                OverlayId::Cjdns,
                1,
                RouteState::Routed,
                Some(1),
                Some(0),
                Some(1_000_000),
                Some(now),
            ),
            candidate(
                service,
                OverlayId::Yggdrasil,
                2,
                RouteState::Reachable,
                Some(100),
                Some(10_000),
                Some(100_000),
                Some(now),
            ),
        ];

        let selected = SelectionPolicy::LowestLatency
            .select_at(&service, &candidates, now, Duration::from_secs(30))
            .unwrap();
        assert_eq!(selected.endpoint.network, OverlayId::Yggdrasil);
    }

    #[test]
    fn invalid_packet_loss_measurements_are_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let service = service_id(b"invalid");
        let candidate = candidate(
            service,
            OverlayId::Cjdns,
            1,
            RouteState::Reachable,
            Some(1),
            Some(1_000_001),
            Some(1_000_000),
            Some(now),
        );

        assert!(
            SelectionPolicy::LowestLatency
                .select_at(&service, &[candidate], now, Duration::from_secs(30))
                .is_none()
        );
    }
}
