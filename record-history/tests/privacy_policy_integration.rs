//! Deny/allow privacy contracts for platform history fixture schemas.
#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod helpers;

use chrono::Utc;
use helpers::{as_user, seed_sources, seed_user, setup_valence, source_a_record_id};
use record_history::E2eRecordHistoryFixture;
use valence::{
    Actor, Model, PrivacyEvaluator, PrivacyOperation, QueryCore, RecordId, SchemaRegistry,
};

const USER_A: &str = "rh-privacy-user-a";
const USER_B: &str = "rh-privacy-user-b";

#[tokio::test]
async fn fixture_history_create_is_system_only_sad() {
    let base = setup_valence().await;
    seed_sources(&base).await;
    seed_user(USER_A, "a@example.com", &base).await;
    seed_user(USER_B, "b@example.com", &base).await;

    let outsider = as_user(&base, USER_B);
    let system = base.with_actor(Actor::System {
        operation: "rh_history_policy_probe".to_string(),
    });

    let seed = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "name".to_string(),
        "old".to_string(),
        "new".to_string(),
        Utc::now(),
        Some(RecordId::new("user", USER_A)),
    )
    .expect("build history row");
    let seeded = E2eRecordHistoryFixture::create(seed, &system)
        .await
        .expect("system may create fixture history");
    let history_id = seeded
        .id()
        .and_then(|r| valence::extract_id_from_record(r).ok())
        .expect("history id");

    let schema = SchemaRegistry::global()
        .get_schema("e2e_record_history_fixture")
        .expect("e2e_record_history_fixture schema registered");
    let raw = QueryCore::get_record_json("e2e_record_history_fixture", &history_id, &system)
        .await
        .expect("raw get")
        .expect("history row");

    assert!(
        PrivacyEvaluator::check_entity_access(schema, PrivacyOperation::Create, &raw, &outsider)
            .await
            .is_err(),
        "authenticated user must not satisfy fixture history create policy"
    );
    assert!(
        PrivacyEvaluator::check_entity_access(schema, PrivacyOperation::Create, &raw, &system)
            .await
            .is_ok(),
        "System must satisfy fixture history create policy"
    );

    let forged = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "forged_field".to_string(),
        "-".to_string(),
        "spoof".to_string(),
        Utc::now(),
        Some(RecordId::new("user", USER_A)),
    )
    .expect("build forged row");
    let forge_attempt = E2eRecordHistoryFixture::create(forged, &outsider).await;
    assert!(
        forge_attempt.is_err(),
        "authenticated E2eRecordHistoryFixture::create must fail under SYSTEM_ONLY create"
    );
}

#[tokio::test]
async fn fixture_history_read_defers_to_parent_authenticated_happy_path() {
    let base = setup_valence().await;
    seed_sources(&base).await;
    seed_user(USER_A, "a@example.com", &base).await;

    let system = base.with_actor(Actor::System {
        operation: "rh_history_read_seed".to_string(),
    });
    let seed = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "status".to_string(),
        "draft".to_string(),
        "active".to_string(),
        Utc::now(),
        Some(RecordId::new("user", USER_A)),
    )
    .expect("build");
    let seeded = E2eRecordHistoryFixture::create(seed, &system)
        .await
        .expect("system create");
    let history_id = seeded
        .id()
        .and_then(|r| valence::extract_id_from_record(r).ok())
        .expect("history id");

    let schema = SchemaRegistry::global()
        .get_schema("e2e_record_history_fixture")
        .expect("schema");
    let raw = QueryCore::get_record_json("e2e_record_history_fixture", &history_id, &system)
        .await
        .expect("raw get")
        .expect("row");

    let reader = as_user(&base, USER_A);
    assert!(
        PrivacyEvaluator::check_entity_access(schema, PrivacyOperation::Read, &raw, &reader)
            .await
            .is_ok(),
        "authenticated reader succeeds when parent Read allows AUTHENTICATED (defer_to_edge)"
    );
}
