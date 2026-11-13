//! One page of a source's history, newest first, continued from a [`HistoryKey`].
//!
//! A source's entries can live in several `RecordHistory` tables, so a page is a
//! merge of per-table reads. Each table is read in `changed_at desc, id desc` order
//! with bounded [`QueryCore`] windows (`limit` plus `resume_at` inside one call), and
//! the merged timeline is ordered by [`HistoryKey`]: `changed_at`, then table, then
//! row id, all descending.
//!
//! - **Keyset path** (`after = Some(key)`): each table reads only the entries after
//!   `key`, so every page costs about one page of rows per table no matter how far
//!   back it is.
//! - **Offset path** (`after = None`): each table reads its newest `skip + limit + 1`
//!   entries and the merge drops the first `skip`. Callers bound `skip`.

use chrono::{DateTime, TimeZone, Utc};
use valence::{
    DataUsePurpose, DateTimePredicate, QueryCore, RecordId, RecordPredicate, SortDirection,
    TraitRegistry, Valence, MAX_QUERY_LIMIT,
};

use super::authorize::authorize_history_source_read;
use super::query::{trait_model_from, HistoryRowWire};
use crate::error::HistoryError;
use crate::generated::RecordHistoryModel;

/// Most entries one read loads for a single `changed_at` instant, and most entries
/// [`super::history_for_source`] returns.
pub const MAX_HISTORY_SCAN: u32 = 10_000;

/// Extra backend windows a read may spend on rows the reader can't see.
///
/// Each window skips up to `valence::MAX_QUERY_HIDDEN_SCAN` hidden rows.
const HIDDEN_WINDOW_BUDGET: usize = 4;

/// Where one history entry sits in the merged timeline.
///
/// Ordering is ascending `(changed_at, table, id)`; timelines list entries in
/// descending order, so "older" means a smaller key. [`Self::encode`] and
/// [`Self::parse`] give a string form for continuation tokens.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HistoryKey {
    /// When the change happened, at the millisecond precision Valence stores.
    pub changed_at: DateTime<Utc>,
    /// Concrete `RecordHistory` table holding the entry.
    pub table: String,
    /// Bare row id within `table`.
    pub id: String,
}

impl HistoryKey {
    /// Key of a row returned by [`history_for_source_page`] or [`super::history_for_source`].
    ///
    /// Returns `None` for a row without an id.
    #[must_use]
    pub fn from_row(row: &RecordHistoryModel) -> Option<Self> {
        use crate::generated::RecordHistoryFields;
        let id = row.id.as_ref()?;
        Some(Self {
            changed_at: *row.changed_at(),
            table: id.table().to_string(),
            id: id.id().to_string(),
        })
    }

    /// `"<changed_at unix millis>|<table>|<id>"`.
    #[must_use]
    pub fn encode(&self) -> String {
        format!(
            "{}|{}|{}",
            self.changed_at.timestamp_millis(),
            self.table,
            self.id
        )
    }

    /// Parse the [`Self::encode`] form. Returns `None` for anything malformed,
    /// including an empty table or id.
    #[must_use]
    pub fn parse(encoded: &str) -> Option<Self> {
        let mut parts = encoded.splitn(3, '|');
        let millis: i64 = parts.next()?.parse().ok()?;
        let table = parts.next()?;
        let id = parts.next()?;
        if table.is_empty() || id.is_empty() {
            return None;
        }
        Some(Self {
            changed_at: Utc.timestamp_millis_opt(millis).single()?,
            table: table.to_string(),
            id: id.to_string(),
        })
    }
}

/// One page from [`history_for_source_page`].
#[derive(Debug, Clone)]
pub struct HistoryPage {
    /// Entries newest first, at most `limit` of them.
    pub rows: Vec<RecordHistoryModel>,
    /// `true` when entries exist after the last row.
    pub has_more: bool,
    /// Key of the last row when `has_more`; pass it as `after` for the next page.
    pub next_key: Option<HistoryKey>,
}

/// One page of history for `source`, newest first, across every registered
/// `RecordHistory` table (or only the tables named in `kinds`).
///
/// Authorizes the request actor against the parent record first, exactly like
/// [`super::history_for_source`]. Entry rows then go through Valence read privacy
/// for the same actor.
///
/// With `after`, the page starts at the first entry older than that key and
/// `skip` should be `0`. Without it, the page skips the newest `skip` entries.
///
/// ## Errors
///
/// [`HistoryError::AccessDenied`] when the parent cannot be authorized;
/// [`HistoryError::Query`] when a read fails or a row cannot be mapped.
pub async fn history_for_source_page(
    source: &RecordId,
    valence: &Valence,
    after: Option<&HistoryKey>,
    skip: u32,
    limit: u32,
    kinds: Option<&[String]>,
) -> Result<HistoryPage, HistoryError> {
    tracing::debug!(
        operation = "history_for_source_page",
        table = source.table(),
        record_id = source.id(),
        keyset = after.is_some(),
        skip,
        limit,
        "paging history for source"
    );
    authorize_history_source_read(source, valence).await?;

    let skip = skip as usize;
    let limit = limit as usize;
    let want = skip.saturating_add(limit).saturating_add(1);
    let predicate = RecordPredicate::Equals(source.clone());
    let tables = TraitRegistry::global().tables_for_trait("RecordHistory");

    let mut merged = Vec::new();
    let mut frontier: Option<HistoryKey> = None;
    let mut any_unread = false;
    for table in tables {
        if kinds.is_some_and(|kinds| !kinds.iter().any(|k| k == table)) {
            continue;
        }
        let read = read_table_after(valence, table, &predicate, after, want).await?;
        if !read.exhausted {
            any_unread = true;
            match read.rows.last() {
                Some((last, _)) => {
                    if frontier.as_ref().is_none_or(|f| last > f) {
                        frontier = Some(last.clone());
                    }
                }
                None => tracing::warn!(
                    operation = "history_for_source_page",
                    table,
                    "history read found no visible rows within its window budget"
                ),
            }
        }
        merged.extend(read.rows);
    }

    merged.sort_by(|a, b| b.0.cmp(&a.0));
    // A table that stopped early only vouches for entries down to its last row.
    if let Some(frontier) = frontier {
        merged.retain(|(key, _)| *key >= frontier);
    }

    let has_more = any_unread || merged.len() > skip.saturating_add(limit);
    let page: Vec<_> = merged.into_iter().skip(skip).take(limit).collect();
    let next_key = if has_more {
        page.last().map(|(key, _)| key.clone())
    } else {
        None
    };
    Ok(HistoryPage {
        rows: page.into_iter().map(|(_, row)| row).collect(),
        has_more,
        next_key,
    })
}

/// Rows of one table, newest first.
struct TableRead {
    rows: Vec<(HistoryKey, RecordHistoryModel)>,
    /// `true` when the table has no entries past the last row.
    exhausted: bool,
}

/// Up to `want` entries of `table` after `after` (or its newest), newest first.
///
/// Entries that share the boundary instant are read as a whole group and ordered
/// in Rust, so the result never depends on how the backend collates ids.
async fn read_table_after(
    valence: &Valence,
    table: &str,
    predicate: &RecordPredicate,
    after: Option<&HistoryKey>,
    want: usize,
) -> Result<TableRead, HistoryError> {
    let mut rows = Vec::new();
    if let Some(key) = after {
        // Entries at the key's instant come after it only from tables that sort
        // below the key's table (or the same table with a smaller id).
        if table <= key.table.as_str() {
            let group = read_rows(
                valence,
                table,
                predicate,
                Some(DateTimePredicate::Equals(key.changed_at)),
                MAX_HISTORY_SCAN as usize,
            )
            .await?;
            warn_if_group_truncated(table, &group);
            rows.extend(group.rows.into_iter().filter(|(k, _)| k < key));
            rows.sort_by(|a, b| b.0.cmp(&a.0));
        }
    }

    let still = want.saturating_sub(rows.len());
    if still == 0 {
        return Ok(TableRead {
            rows,
            exhausted: false,
        });
    }

    let older = read_rows(
        valence,
        table,
        predicate,
        after.map(|key| DateTimePredicate::Before(key.changed_at)),
        still,
    )
    .await?;
    let exhausted = older.exhausted;
    let mut older_rows = older.rows;
    // The read may have stopped inside a group of entries sharing one instant.
    // Replace that last group with the whole group.
    if !exhausted {
        if let Some(last_at) = older_rows.last().map(|(k, _)| k.changed_at) {
            older_rows.retain(|(k, _)| k.changed_at != last_at);
            let group = read_rows(
                valence,
                table,
                predicate,
                Some(DateTimePredicate::Equals(last_at)),
                MAX_HISTORY_SCAN as usize,
            )
            .await?;
            warn_if_group_truncated(table, &group);
            older_rows.extend(group.rows);
        }
    }
    older_rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.extend(older_rows);
    Ok(TableRead { rows, exhausted })
}

fn warn_if_group_truncated(table: &str, group: &TableRead) {
    if !group.exhausted {
        tracing::warn!(
            operation = "history_for_source_page",
            table,
            max = MAX_HISTORY_SCAN,
            "more history entries share one instant than a page reads"
        );
    }
}

/// Up to `want` readable rows of `table` for the source, `changed_at desc, id desc`,
/// resuming across backend windows. Stops early (not exhausted) when the hidden-row
/// window budget runs out.
async fn read_rows(
    valence: &Valence,
    table: &str,
    predicate: &RecordPredicate,
    changed_at: Option<DateTimePredicate>,
    want: usize,
) -> Result<TableRead, HistoryError> {
    // Copy of the read's inline purpose for error wraps (see `history_for_source`).
    let purpose: DataUsePurpose = valence::use_!(
        r"When you **view a record's history**, we **search every table that stores change history** for entries about that record, loading what changed, the old and new values, when, and who made the change. These entries become the record's **timeline**, and this step only runs after we have confirmed you are allowed to read that record."
    );
    let max_windows = want.div_ceil(MAX_QUERY_LIMIT as usize) + HIDDEN_WINDOW_BUDGET;
    let mut rows = Vec::new();
    let mut position = None;
    for _ in 0..max_windows {
        let need = u32::try_from(want - rows.len()).unwrap_or(u32::MAX);
        let mut query =
            QueryCore::new(table.to_string()).where_record("source".to_string(), predicate.clone());
        if let Some(changed_at) = &changed_at {
            query = query.where_datetime("changed_at".to_string(), changed_at.clone());
        }
        query = query
            .order_by("changed_at".to_string(), SortDirection::Desc)
            .order_by("id".to_string(), SortDirection::Desc)
            .limit(need);
        if let Some(position) = position {
            query = query.resume_at(position);
        }
        let (batch, stats) = query
            .execute_with_stats::<HistoryRowWire>(valence, valence::use_!(r"When you **view a record's history**, we **search every table that stores change history** for entries about that record, loading what changed, the old and new values, when, and who made the change. These entries become the record's **timeline**, and this step only runs after we have confirmed you are allowed to read that record."))
            .await
            .map_err(|e| HistoryError::query(e, purpose))?;
        for row in batch {
            let id = row.id.clone().map(|rid| {
                if rid.table() == table {
                    rid
                } else {
                    RecordId::new(table, rid.id())
                }
            });
            let key = HistoryKey {
                changed_at: row.changed_at,
                table: table.to_string(),
                id: id
                    .as_ref()
                    .map(|rid| rid.id().to_string())
                    .unwrap_or_default(),
            };
            rows.push((key, trait_model_from(&row, id, purpose)?));
        }
        let Some(next) = stats.resume else {
            return Ok(TableRead {
                rows,
                exhausted: true,
            });
        };
        position = Some(next);
        if rows.len() >= want {
            break;
        }
    }
    Ok(TableRead {
        rows,
        exhausted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::HistoryKey;
    use chrono::{TimeZone, Utc};

    fn key(millis: i64, table: &str, id: &str) -> HistoryKey {
        HistoryKey {
            changed_at: Utc.timestamp_millis_opt(millis).single().expect("ts"),
            table: table.into(),
            id: id.into(),
        }
    }

    #[test]
    fn history_key_round_trips_happy_path() {
        let k = key(1_763_000_000_123, "tag_history", "row|with|pipes");
        assert_eq!(HistoryKey::parse(&k.encode()), Some(k));
    }

    #[test]
    fn history_key_rejects_malformed_sad() {
        for bad in [
            "",
            "abc|t|i",
            "123",
            "123|t",
            "123||i",
            "123|t|",
            "99999999999999999999|t|i",
        ] {
            assert_eq!(HistoryKey::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn history_key_orders_time_then_table_then_id_happy_path() {
        assert!(key(2, "a", "a") > key(1, "z", "z"));
        assert!(key(1, "b", "a") > key(1, "a", "z"));
        assert!(key(1, "a", "b") > key(1, "a", "a"));
    }
}
