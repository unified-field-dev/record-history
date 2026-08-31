use valence::prelude::*;
use valence::privacy_policies::common::{BLOCK_ALL, SYSTEM_ONLY};

valence_schema! {
    E2eRecordHistoryFixtureAlt {
        table: "e2e_record_history_fixture_alt",
        version: "0.1.3",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "Second E2E fixture implementor for cross-table RecordHistoryQueryAll tests",

        traits: [RecordHistory],

        policies: {
            read: { defer_to_edge: "source" },
            create: {
                allow: [SYSTEM_ONLY],
            },
            update: {
                always_block: [BLOCK_ALL],
            },
            // Direct user deletes stay denied; source CascadeDelete runs as System.
            delete: {
                allow: [SYSTEM_ONLY],
            },
        },

        // Keep fields empty (trait-only), matching `E2eRecordHistoryFixture`.
        // An extra local field previously broke SQLite readback of trait HasOne `source`.
        fields: [],
    }
}
