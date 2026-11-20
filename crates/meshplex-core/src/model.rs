use std::net::IpAddr;
use std::time::Duration;

/// One million parts per million represents 100% packet loss.
pub const MAX_PACKET_LOSS_PPM: u32 = 1_000_000;

/// Stable identifier for an overlay implementation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OverlayId {
    Cjdns,
    Yggdrasil,
    Other(String),
}

/// An address that a service has explicitly advertised on an overlay.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OverlayEndpoint {
    pub network: OverlayId,
    pub address: IpAddr,
    pub port: Option<u16>,
}

/// Latest observed reachability state for a candidate path.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RouteState {
    /// The path has passed the adapter's reachability checks.
    Reachable,
    /// The path is usable, but the adapter has detected degradation.
    Degraded,
    /// The path was checked and is not reachable.
    Unreachable,
    /// The adapter has not established whether the path is reachable.
    Unknown,
}

/// Measurements associated with a candidate path.
///
/// Missing values remain unknown; they are not treated as zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RouteMetrics {
    /// Round-trip time measured by the adapter.
    pub round_trip_time: Option<Duration>,
    /// Variation in round-trip time.
    pub jitter: Option<Duration>,
    /// Packet loss in parts per million, from 0 through 1,000,000.
    pub packet_loss_ppm: Option<u32>,
    /// Observed throughput in bits per second.
    pub throughput_bps: Option<u64>,
}

impl RouteMetrics {
    /// Returns false when a supplied packet-loss value is outside its valid range.
    pub fn is_valid(&self) -> bool {
        self.packet_loss_ppm
            .is_none_or(|loss| loss <= MAX_PACKET_LOSS_PPM)
    }
}

/// A service endpoint and the observations currently available for its path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCandidate {
    pub endpoint: OverlayEndpoint,
    pub state: RouteState,
    pub metrics: RouteMetrics,
}

impl RouteCandidate {
    /// Whether this candidate may be considered by the initial selector.
    ///
    /// Unknown and unreachable paths are excluded. Degraded paths remain eligible
    /// as a fallback when no healthy path is available.
    pub fn is_eligible(&self) -> bool {
        matches!(self.state, RouteState::Reachable | RouteState::Degraded)
            && self.metrics.is_valid()
    }
}
