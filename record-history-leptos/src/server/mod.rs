//! SSR-only helpers for resolving actor presentation and mapping DB rows to view types.

#[cfg(feature = "ssr")]
mod actor;
#[cfg(feature = "ssr")]
mod map_row;

#[cfg(feature = "ssr")]
pub use actor::{actor_profile_href, resolve_actor_presentation};
#[cfg(feature = "ssr")]
pub use map_row::into_history_row_view;
