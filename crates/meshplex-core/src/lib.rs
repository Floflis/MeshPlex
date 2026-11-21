//! Protocol-neutral building blocks for coordinating independent overlay networks.
mod adapter;
mod model;
mod selector;
mod service;
mod signed_service;
pub use adapter::{AdapterError, OverlayAdapter, OverlaySnapshot};
pub use model::{
    DEFAULT_MAX_MEASUREMENT_AGE, MAX_PACKET_LOSS_PPM, OverlayEndpoint, OverlayId, OverlayRoute,
    RouteCandidate, RouteMetrics, RouteState, ServiceId, ServiceIdentityError,
};
pub use selector::SelectionPolicy;
pub use service::{ServiceAdvertisement, ServiceAdvertisementError};
pub use signed_service::{SignedAdvertisementError, SignedServiceAdvertisement};
