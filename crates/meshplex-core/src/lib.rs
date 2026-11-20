//! Protocol-neutral building blocks for coordinating independent overlay networks.
//!
//! This crate deliberately does not implement an overlay protocol, packet forwarding,
//! daemon supervision, or operating-system route changes.

mod adapter;
mod model;
mod selector;

pub use adapter::{AdapterError, OverlayAdapter, OverlaySnapshot};
pub use model::{
    MAX_PACKET_LOSS_PPM, OverlayEndpoint, OverlayId, RouteCandidate, RouteMetrics, RouteState,
};
pub use selector::SelectionPolicy;
