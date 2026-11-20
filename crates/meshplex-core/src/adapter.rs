use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{OverlayId, RouteCandidate};

/// A read-only view of one overlay adapter's latest observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverlaySnapshot {
    pub network: OverlayId,
    pub connected_peers: usize,
    pub known_peers: usize,
    pub routes: Vec<RouteCandidate>,
}

/// Error returned while collecting an overlay snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterError {
    message: String,
}

impl AdapterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AdapterError {}

/// Read-only boundary between MeshPlex and an independently running overlay.
///
/// Implementations may collect data from a supported admin API, a local daemon
/// interface, or another documented source. The initial contract intentionally
/// exposes observation only: it does not permit an adapter to rewrite routes,
/// alter peers, or change daemon configuration.
pub trait OverlayAdapter: Send + Sync {
    fn snapshot(&self) -> Result<OverlaySnapshot, AdapterError>;
}
