//! Typed failures for history reads and parent authorization.

use std::fmt;

/// Why [`HistoryError::AccessDenied`] fired. Inspectable in tests; omitted from [`fmt::Display`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAccessDeniedReason {
    /// No row for the source `RecordId`.
    MissingSource,
    /// Table is not a registered `HistorySource` implementor, or has no schema.
    UnsupportedSource,
    /// Request actor failed the parent record's Read policy.
    ParentReadDenied,
}

/// Library-facing failures from [`crate::history_for_source`] and
/// [`crate::authorize_history_source_read`].
///
/// [`Self::AccessDenied`] uses one Display string for missing, unsupported, and
/// parent-read denial so clients cannot distinguish those cases.
#[derive(Debug)]
pub enum HistoryError {
    /// Parent authorization failed closed.
    AccessDenied {
        /// Classification for tests and logs (not shown in Display).
        reason: HistoryAccessDeniedReason,
    },
    /// Valence query, get, or serialization failure after authorization.
    Query {
        /// Source error.
        source: valence::Error,
    },
}

/// Client-safe message for [`HistoryError::AccessDenied`] and the timeline server fn.
pub const HISTORY_ACCESS_DENIED: &str = "Not authorized to view this history";

impl HistoryError {
    #[must_use]
    pub(crate) const fn access_denied(reason: HistoryAccessDeniedReason) -> Self {
        Self::AccessDenied { reason }
    }

    #[must_use]
    pub(crate) fn query(source: valence::Error) -> Self {
        Self::Query { source }
    }

    /// True when the failure is fail-closed authorization (not a query I/O error).
    #[must_use]
    pub const fn is_access_denied(&self) -> bool {
        matches!(self, Self::AccessDenied { .. })
    }
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied { .. } => f.write_str(HISTORY_ACCESS_DENIED),
            Self::Query { .. } => f.write_str("history query failed"),
        }
    }
}

impl std::error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Query { source } => Some(source),
            Self::AccessDenied { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryAccessDeniedReason, HistoryError, HISTORY_ACCESS_DENIED};

    #[test]
    fn access_denied_display_has_no_record_id_or_email_sad() {
        let err = HistoryError::access_denied(HistoryAccessDeniedReason::ParentReadDenied);
        let msg = err.to_string();
        assert_eq!(msg, HISTORY_ACCESS_DENIED);
        assert!(!msg.contains('@'));
        assert!(!msg.contains("user:"));
        assert!(!msg.contains("e2e_history"));
    }

    #[test]
    fn missing_and_unsupported_share_display_happy_path() {
        let missing = HistoryError::access_denied(HistoryAccessDeniedReason::MissingSource);
        let unsupported = HistoryError::access_denied(HistoryAccessDeniedReason::UnsupportedSource);
        assert_eq!(missing.to_string(), unsupported.to_string());
        assert!(missing.is_access_denied());
    }
}
