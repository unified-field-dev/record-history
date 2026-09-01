//! Leptos components rendering a [`crate::HistoryTimeline`].

pub mod history_timeline;

pub use history_timeline::HistoryTimeline;
#[cfg(feature = "preview")]
pub use history_timeline::{HISTORYTIMELINE_DOC, HISTORYTIMELINE_PROPS};
