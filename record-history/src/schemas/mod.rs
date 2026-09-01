//! Compile-time trait DSL only.
//!
//! Concrete `valence_schema!` tables are registered by codegen
//! (`generated_models.rs` via `build.rs`). Including those files here as well
//! submitted a second `SchemaMetadataInit` with trait fields unmerged (empty
//! `fields`), and inventory last-write-wins left the empty schema in
//! [`valence::SchemaRegistry`] — breaking boot sync / typed SQLite layouts.

mod history_source_trait {
    include!("../../schemas/history_source_valence_trait.rs");
}

mod record_history_trait {
    include!("../../schemas/record_history_valence_trait.rs");
}
