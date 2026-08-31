use valence::prelude::*;
use valence::privacy_policies::common::{BLOCK_ALL, SYSTEM_ONLY};

valence_schema! {
    E2eRecordHistoryFixture {
        table: "e2e_record_history_fixture",
        version: "0.1.3",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "E2E fixture table implementing RecordHistory (platform tests only)",

        traits: [RecordHistory],

        policies: {
            // Parent Read via `source` (e2e_history_source_a is AUTHENTICATED).
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

        fields: [],
    }
}
