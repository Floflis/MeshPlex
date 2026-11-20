use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{OverlayId, OverlayRoute};

/// A read-only view of one overlay adapter's latest observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverlaySnapshot {
    pub network: OverlayId,
    /// None means this adapter does not expose a trustworthy connected-peer count.
    pub connected_peers: Option<usize>,
    pub known_routes: usize,
    pub routes: Vec<OverlayRoute>,
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
/// Implementations collect observations from a documented local/admin interface.
/// The contract intentionally has no methods for changing peers, daemon settings,
/// host routes, or packet forwarding state.
pub trait OverlayAdapter: Send + Sync {
    fn snapshot(&self) -> Result<OverlaySnapshot, AdapterError>;
}
