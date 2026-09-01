use valence::prelude::*;
use valence::privacy_policies::common::SYSTEM_ONLY;
use valence::privacy_policies::owner::OWNER_BY_USER_FIELD;

valence_schema! {
    E2eHistorySourceOwned {
        table: "e2e_history_source_owned",
        version: "0.1.0",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "E2E HistorySource with owner-scoped read (parent ACL / IDOR tests)",

        traits: [HistorySource],

        policies: {
            read: {
                allow: [OWNER_BY_USER_FIELD, SYSTEM_ONLY],
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
            user: {
                r#type: FieldType::String,
                required: true,
            },
        ],
    }
}
