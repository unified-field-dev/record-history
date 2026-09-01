use valence::prelude::*;
use valence::privacy_policies::common::{AUTHENTICATED, SYSTEM_ONLY};

valence_schema! {
    E2eHistorySourceB {
        table: "e2e_history_source_b",
        version: "0.1.0",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "E2E HistorySource implementor B (platform tests only)",

        traits: [HistorySource],

        policies: {
            read: {
                allow: [AUTHENTICATED, SYSTEM_ONLY],
            },
            create: {
                allow: [SYSTEM_ONLY],
            },
            update: {
                allow: [SYSTEM_ONLY],
            },
            delete: {
                allow: [SYSTEM_ONLY],
            },
        },

        fields: [
            id: {
                r#type: FieldType::String,
                primary_key: true,
                required: true,
            },
            label: {
                r#type: FieldType::String,
                required: true,
            },
        ],
    }
}
