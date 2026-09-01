//! Wire-friendly row view type shared between SSR mapping and client rendering.

mod map_entry;
mod row_view;

pub use map_entry::{history_row_view_to_entry, map_history_page};
pub use row_view::HistoryRowView;
