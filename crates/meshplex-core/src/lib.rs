//! Protocol-neutral building blocks for coordinating independent overlay networks.
//!
//! This crate owns service identity and route-selection policy. Overlay adapters
//! publish observations; they do not own application identities or alter routes.

mod adapter;
mod model;
mod selector;
mod service;

pub use adapter::{AdapterError, OverlayAdapter, OverlaySnapshot};
pub use model::{
    DEFAULT_MAX_MEASUREMENT_AGE, MAX_PACKET_LOSS_PPM, OverlayEndpoint, OverlayId, OverlayRoute,
    RouteCandidate, RouteMetrics, RouteState, ServiceId, ServiceIdentityError,
};
pub use selector::SelectionPolicy;
pub use service::{ServiceAdvertisement, ServiceAdvertisementError};
