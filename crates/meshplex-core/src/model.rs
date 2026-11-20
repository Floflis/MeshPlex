use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

/// Default maximum age for measurements accepted by the selector.
pub const DEFAULT_MAX_MEASUREMENT_AGE: Duration = Duration::from_secs(30);

/// One million parts per million represents 100% packet loss.
pub const MAX_PACKET_LOSS_PPM: u32 = 1_000_000;

/// Stable identifier for an overlay implementation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OverlayId {
    Cjdns,
    Yggdrasil,
    Other(String),
}

/// Application-level identity shared by a service's endpoints on different overlays.
///
/// The identifier is SHA-256 over a domain-separated, canonical public-key byte string.
/// It is deliberately independent of CJDNS and Yggdrasil node identities.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ServiceId([u8; 32]);

impl ServiceId {
    /// Derive a stable service identifier from its canonical public identity key.
    ///
    /// This hashes public key material only; callers must never pass a private key.
    pub fn from_public_key(public_key: &[u8]) -> Result<Self, ServiceIdentityError> {
        if public_key.is_empty() {
            return Err(ServiceIdentityError::EmptyPublicKey);
        }

        let mut hasher = Sha256::new();
        hasher.update(b"meshplex/service-id/v1\0");
        hasher.update((public_key.len() as u64).to_be_bytes());
        hasher.update(public_key);

        let digest = hasher.finalize();
        let mut identifier = [0_u8; 32];
        identifier.copy_from_slice(&digest);
        Ok(Self(identifier))
    }

    /// Return the raw 32-byte identifier.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Display for ServiceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sha256:")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Error raised while deriving a service identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceIdentityError {
    EmptyPublicKey,
}

impl Display for ServiceIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPublicKey => formatter.write_str("service public key cannot be empty"),
        }
    }
}

impl std::error::Error for ServiceIdentityError {}

/// An address explicitly advertised by a service on one overlay.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OverlayEndpoint {
    pub network: OverlayId,
    pub address: IpAddr,
    pub port: Option<u16>,
}

/// Latest observed reachability state for a route to an overlay endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteState {
    /// An active end-to-end probe succeeded.
    Reachable,
    /// The overlay currently has a route, but no end-to-end probe is implied.
    Routed,
    /// The path is usable, but the adapter detected degradation.
    Degraded,
    /// The path was checked and is not reachable.
    Unreachable,
    /// The adapter has not established whether the path is reachable.
    Unknown,
}

impl RouteState {
    /// Lower values are preferred by the selector.
    pub(crate) const fn preference(self) -> u8 {
        match self {
            Self::Reachable => 0,
            Self::Routed => 1,
            Self::Degraded => 2,
            Self::Unreachable => 3,
            Self::Unknown => 4,
        }
    }

    pub(crate) const fn is_eligible(self) -> bool {
        matches!(self, Self::Reachable | Self::Routed | Self::Degraded)
    }
}

/// Measurements associated with a path.
///
/// observed_at is the collection time for this set of metrics, not a claim that
/// every value was actively measured. Missing values remain unknown, never zero.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RouteMetrics {
    pub round_trip_time: Option<Duration>,
    pub jitter: Option<Duration>,
    pub packet_loss_ppm: Option<u32>,
    pub throughput_bps: Option<u64>,
    /// Overlay-provided route quality score, where larger values are better.
    pub path_quality: Option<u32>,
    /// Wall-clock time when this adapter collected the values.
    pub observed_at: Option<SystemTime>,
}

impl RouteMetrics {
    /// Returns false when a supplied packet-loss value is outside its valid range.
    pub fn is_valid(&self) -> bool {
        self.packet_loss_ppm
            .is_none_or(|loss| loss <= MAX_PACKET_LOSS_PPM)
    }

    /// A missing timestamp, a future timestamp, or an expired sample is not fresh.
    pub fn is_fresh_at(&self, now: SystemTime, max_age: Duration) -> bool {
        self.observed_at
            .and_then(|observed_at| now.duration_since(observed_at).ok())
            .is_some_and(|age| age <= max_age)
    }
}

/// A route observed by one overlay adapter, before service identity is joined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverlayRoute {
    pub endpoint: OverlayEndpoint,
    pub state: RouteState,
    pub metrics: RouteMetrics,
}

/// A service-specific route candidate created by joining an advertisement to an
/// overlay's observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCandidate {
    pub service_id: ServiceId,
    pub endpoint: OverlayEndpoint,
    pub state: RouteState,
    pub metrics: RouteMetrics,
}

#[cfg(test)]
mod tests {
    use super::{ServiceId, ServiceIdentityError};

    #[test]
    fn service_identity_is_stable_for_the_same_public_key() {
        let first = ServiceId::from_public_key(b"service public key").unwrap();
        let second = ServiceId::from_public_key(b"service public key").unwrap();

        assert_eq!(first, second);
        assert_eq!(first.as_bytes().len(), 32);
        assert!(first.to_string().starts_with("sha256:"));
    }

    #[test]
    fn different_public_keys_produce_different_service_identities() {
        let first = ServiceId::from_public_key(b"service A").unwrap();
        let second = ServiceId::from_public_key(b"service B").unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn empty_public_keys_are_rejected() {
        assert_eq!(
            ServiceId::from_public_key(b"").unwrap_err(),
            ServiceIdentityError::EmptyPublicKey
        );
    }
}
