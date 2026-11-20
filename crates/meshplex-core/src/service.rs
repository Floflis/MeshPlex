use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use crate::{OverlayEndpoint, OverlaySnapshot, RouteCandidate, ServiceId, ServiceIdentityError};

/// A service identity and the endpoints it explicitly advertises on independent overlays.
///
/// This is an identity-to-endpoint mapping, not a signed advertisement format. The
/// authenticity and distribution of advertisements are deliberately left for a later
/// protocol design; a matching identifier alone does not prove endpoint ownership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceAdvertisement {
    pub service_id: ServiceId,
    pub endpoints: Vec<OverlayEndpoint>,
}

impl ServiceAdvertisement {
    /// Build an advertisement by deriving its identity from the service public key.
    pub fn new(
        public_key: &[u8],
        endpoints: Vec<OverlayEndpoint>,
    ) -> Result<Self, ServiceAdvertisementError> {
        if endpoints.is_empty() {
            return Err(ServiceAdvertisementError::NoEndpoints);
        }

        let mut unique = BTreeSet::new();
        for endpoint in &endpoints {
            if !unique.insert(endpoint.clone()) {
                return Err(ServiceAdvertisementError::DuplicateEndpoint);
            }
        }

        let service_id = ServiceId::from_public_key(public_key)?;
        Ok(Self {
            service_id,
            endpoints,
        })
    }

    /// Join advertised endpoints to current overlay observations.
    ///
    /// Only matching (overlay, IP address) pairs produce candidates. A route on
    /// another overlay is never treated as a substitute for the advertised endpoint.
    pub fn route_candidates(&self, snapshots: &[OverlaySnapshot]) -> Vec<RouteCandidate> {
        let mut candidates = Vec::new();

        for endpoint in &self.endpoints {
            let observed_route = snapshots
                .iter()
                .filter(|snapshot| snapshot.network == endpoint.network)
                .flat_map(|snapshot| snapshot.routes.iter())
                .find(|route| {
                    route.endpoint.network == endpoint.network
                        && route.endpoint.address == endpoint.address
                });

            if let Some(route) = observed_route {
                candidates.push(RouteCandidate {
                    service_id: self.service_id,
                    endpoint: endpoint.clone(),
                    state: route.state,
                    metrics: route.metrics.clone(),
                });
            }
        }

        candidates
    }
}

/// Error while constructing a service advertisement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceAdvertisementError {
    Identity(ServiceIdentityError),
    NoEndpoints,
    DuplicateEndpoint,
}

impl From<ServiceIdentityError> for ServiceAdvertisementError {
    fn from(error: ServiceIdentityError) -> Self {
        Self::Identity(error)
    }
}

impl Display for ServiceAdvertisementError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(error) => Display::fmt(error, formatter),
            Self::NoEndpoints => {
                formatter.write_str("a service must advertise at least one endpoint")
            }
            Self::DuplicateEndpoint => {
                formatter.write_str("service advertisement contains a duplicate endpoint")
            }
        }
    }
}

impl std::error::Error for ServiceAdvertisementError {}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};
    use std::time::SystemTime;

    use crate::{
        OverlayEndpoint, OverlayId, OverlayRoute, OverlaySnapshot, RouteMetrics, RouteState,
        ServiceAdvertisement,
    };

    fn endpoint(network: OverlayId, last_segment: u16) -> OverlayEndpoint {
        OverlayEndpoint {
            network,
            address: IpAddr::V6(Ipv6Addr::new(0x200, 0, 0, 0, 0, 0, 0, last_segment)),
            port: Some(443),
        }
    }

    #[test]
    fn one_service_identity_can_advertise_endpoints_on_two_overlays() {
        let advertisement = ServiceAdvertisement::new(
            b"canonical service public key",
            vec![
                endpoint(OverlayId::Cjdns, 1),
                endpoint(OverlayId::Yggdrasil, 2),
            ],
        )
        .unwrap();

        let now = SystemTime::now();
        let snapshots = [
            OverlaySnapshot {
                network: OverlayId::Cjdns,
                connected_peers: None,
                known_routes: 1,
                routes: vec![OverlayRoute {
                    endpoint: endpoint(OverlayId::Cjdns, 1),
                    state: RouteState::Routed,
                    metrics: RouteMetrics {
                        path_quality: Some(u32::MAX),
                        observed_at: Some(now),
                        ..RouteMetrics::default()
                    },
                }],
            },
            OverlaySnapshot {
                network: OverlayId::Yggdrasil,
                connected_peers: Some(1),
                known_routes: 1,
                routes: vec![OverlayRoute {
                    endpoint: endpoint(OverlayId::Yggdrasil, 2),
                    state: RouteState::Reachable,
                    metrics: RouteMetrics {
                        round_trip_time: None,
                        observed_at: Some(now),
                        ..RouteMetrics::default()
                    },
                }],
            },
        ];

        let candidates = advertisement.route_candidates(&snapshots);
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.service_id == advertisement.service_id)
        );
    }

    #[test]
    fn advertisement_does_not_join_an_endpoint_from_the_wrong_overlay() {
        let advertisement =
            ServiceAdvertisement::new(b"service key", vec![endpoint(OverlayId::Cjdns, 7)]).unwrap();

        let snapshots = [OverlaySnapshot {
            network: OverlayId::Yggdrasil,
            connected_peers: Some(1),
            known_routes: 1,
            routes: vec![OverlayRoute {
                endpoint: endpoint(OverlayId::Yggdrasil, 7),
                state: RouteState::Reachable,
                metrics: RouteMetrics {
                    observed_at: Some(SystemTime::now()),
                    ..RouteMetrics::default()
                },
            }],
        }];

        assert!(advertisement.route_candidates(&snapshots).is_empty());
    }

    #[test]
    fn duplicate_endpoints_are_rejected() {
        let endpoint = endpoint(OverlayId::Cjdns, 1);
        let result = ServiceAdvertisement::new(b"service key", vec![endpoint.clone(), endpoint]);

        assert_eq!(
            result.unwrap_err(),
            crate::ServiceAdvertisementError::DuplicateEndpoint
        );
    }
}
