//! Static preview registrations exported for host catalog merge.

use crate::preview::PreviewRegistration;

#[cfg(feature = "preview")]
use crate::components::history_timeline::HISTORYTIMELINE_PREVIEW_REGISTRATION;

orbital_macros::preview_registrations! {
    &HISTORYTIMELINE_PREVIEW_REGISTRATION,
}
