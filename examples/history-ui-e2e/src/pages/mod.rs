//! Demo pages for the History e2e host.

mod home;
mod not_found;
mod timeline;
mod timeline_kind;
mod timeline_renderers;

pub use home::HomePage;
pub use not_found::NotFoundDemoPage;
pub use timeline::TimelinePage;
pub use timeline_kind::{TimelineKindAbsentPage, TimelineKindFilterPage};
pub use timeline_renderers::TimelineRenderersPage;
