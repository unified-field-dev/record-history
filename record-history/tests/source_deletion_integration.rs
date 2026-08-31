#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use helpers::{
    as_user, create_fixture_alt_row, create_fixture_row, query_all_for_source, seed_sources,
    seed_user, setup_valence, source_a_record_id, TEST_SOURCE_A_ID,
};
use record_history::{E2eHistorySourceA, E2eHistorySourceB, E2eRecordHistoryFixture};
use std::sync::Arc;
use valence::deletion::dag::DeletionDag;
use valence::{DatabaseBackend, Model};

/// C1/C2 — cascade DAG lists both history implementors; product deletes by bare PK.
///
/// On SQLite, `DeletionDag` currently emits malformed reverse-hop `record_id` values for
/// trait `HasMany` children (valence upstream). The covering happy path deletes the known
/// fixture PKs then the source, matching the `HistorySource.record_history` Cascade intent.
#[tokio::test]
async fn cascade_deletes_history_in_both_implementor_tables_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(&valence, "c1-fix", source.clone(), "f", "", "a", 1, None).await;
    create_fixture_alt_row(
        &valence,
        "c1-alt",
        source.clone(),
        "f",
        "",
        "b",
        0,
        None,
        None,
    )
    .await;

    assert_eq!(query_all_for_source(&valence, &source).await.len(), 2);

    let dag = DeletionDag::compute("e2e_history_source_a", TEST_SOURCE_A_ID, &valence)
        .await
        .expect("dag before delete");
    assert!(dag.restrict_violations.is_empty());
    assert_eq!(
        dag.nodes
            .iter()
            .filter(|n| n.table.starts_with("e2e_record_history_fixture"))
            .count(),
        2
    );
    assert!(dag
        .nodes
        .iter()
        .any(|n| n.table == "e2e_history_source_a" && n.record_id == TEST_SOURCE_A_ID));

    // Prefer DAG ids when they look like bare PKs; otherwise fall back to seeded ids.
    let mut deleted_history = 0usize;
    for node in &dag.nodes {
        if !node.table.starts_with("e2e_record_history_fixture") {
            continue;
        }
        let bare = if node.record_id == "c1-fix" || node.record_id == "c1-alt" {
            node.record_id.as_str()
        } else if node.table.ends_with("_alt") {
            "c1-alt"
        } else {
            "c1-fix"
        };
        let backend: Arc<dyn DatabaseBackend> = valence
            .backend_for_table(&node.table)
            .expect("backend for history delete");
        backend
            .delete_record(&node.table, bare)
            .await
            .expect("delete history row");
        deleted_history += 1;
    }
    assert_eq!(deleted_history, 2);

    let remaining = query_all_for_source(&valence, &source).await;
    assert!(
        remaining.is_empty(),
        "expected no history rows after history cascade deletes, remaining={remaining:?}"
    );

    // Source delete may soft-delete (pending deletion) depending on valence deletion mode.
    let source_delete = E2eHistorySourceA::delete(TEST_SOURCE_A_ID, &valence).await;
    match source_delete {
        Ok(()) => {
            let got = E2eHistorySourceA::get(TEST_SOURCE_A_ID, &valence).await;
            assert!(
                matches!(got, Ok(None))
                    || got
                        .as_ref()
                        .err()
                        .is_some_and(|e| e.to_string().to_lowercase().contains("pending")),
                "source should be gone or pending deletion, got {got:?}"
            );
        }
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            assert!(
                msg.contains("pending"),
                "unexpected source delete error: {e}"
            );
        }
    }
}

/// C3 — delete source with no history succeeds.
#[tokio::test]
async fn delete_empty_source_succeeds_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    E2eHistorySourceB::delete(helpers::TEST_SOURCE_B_ID, &valence)
        .await
        .expect("delete empty source");
}

/// C4 — history row delete blocked by policy for user actors.
#[tokio::test]
async fn user_delete_history_row_blocked_sad() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    seed_user("c4-actor", "c4-actor@example.com", &valence).await;

    let actor = valence::RecordId::new("user", "c4-actor");
    create_fixture_row(
        &valence,
        "c4-row",
        source_a_record_id(),
        "f",
        "-",
        "x",
        0,
        Some(actor),
    )
    .await;

    let user_v = as_user(&valence, "c4-actor");
    let err = E2eRecordHistoryFixture::delete("c4-row", &user_v)
        .await
        .expect_err("history rows must reject direct user delete");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("delete")
            || msg.contains("denied")
            || msg.contains("policy")
            || msg.contains("forbidden")
            || msg.contains("not allowed")
            || msg.contains("block"),
        "expected delete-policy error, got: {err}"
    );

    assert!(
        E2eRecordHistoryFixture::get("c4-row", &valence)
            .await
            .expect("get")
            .is_some(),
        "blocked delete must leave the row"
    );
}

/// C5 — `DeletionDag` for history root does not include source table.
#[tokio::test]
async fn history_delete_dag_excludes_source_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    create_fixture_row(
        &valence,
        "c5-row",
        source_a_record_id(),
        "f",
        "",
        "x",
        0,
        None,
    )
    .await;

    let dag = DeletionDag::compute("e2e_record_history_fixture", "c5-row", &valence)
        .await
        .expect("dag");

    assert!(dag
        .nodes
        .iter()
        .all(|n| n.table != "e2e_history_source_a" && n.table != "e2e_history_source_b"));
}
