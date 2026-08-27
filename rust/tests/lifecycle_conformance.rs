use serde::Deserialize;
use serde_json::Value;
use value_core::hold::{
    HoldState, ValueHold, create_value_hold, release_value_hold, settle_value_hold,
};
use value_core::idempotency::{
    ValueCommand, create_value_command_digest, resolve_value_command_replay,
};
use value_core::reconciliation::{ReconciliationBalance, reconcile_balances};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoldCreateVector {
    hold_id: String,
    account_id: String,
    asset: String,
    amount_minor: String,
    available_balance_minor: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoldVector {
    hold_id: String,
    account_id: String,
    asset: String,
    amount_minor: String,
    state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettlementVector {
    destination_account_id: String,
    amount_minor: String,
    settled_amount_minor: String,
    released_amount_minor: String,
    state: String,
    postings: Vec<PostingVector>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostingVector {
    account_id: String,
    asset: String,
    amount_minor: String,
}

#[derive(Deserialize)]
struct FailureVector {
    kind: String,
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoldFixture {
    create: HoldCreateVector,
    open: HoldVector,
    settlement: SettlementVector,
    release_state: String,
    failures: Vec<FailureVector>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandVector {
    command_id: String,
    contract_version: String,
    payload: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdempotencyFixture {
    existing: CommandVector,
    same_intent: CommandVector,
    changed_intent: CommandVector,
    digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceVector {
    account_id: String,
    asset: String,
    amount_minor: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DifferenceVector {
    account_id: String,
    asset: String,
    expected_minor: String,
    actual_minor: String,
    difference_minor: String,
}

#[derive(Deserialize)]
struct ReconciliationFixture {
    expected: Vec<BalanceVector>,
    actual: Vec<BalanceVector>,
    closed: bool,
    differences: Vec<DifferenceVector>,
}

#[derive(Deserialize)]
struct LifecycleFixture {
    profile: String,
    hold: HoldFixture,
    idempotency: IdempotencyFixture,
    reconciliation: ReconciliationFixture,
}

fn fixture() -> LifecycleFixture {
    serde_json::from_str(include_str!("../fixtures/lifecycle-v1.json")).unwrap()
}

fn hold_state(value: &str) -> HoldState {
    match value {
        "open" => HoldState::Open,
        "released" => HoldState::Released,
        "settled" => HoldState::Settled,
        other => panic!("unknown Hold state {other}"),
    }
}

fn value_hold(vector: &HoldVector) -> ValueHold {
    ValueHold {
        hold_id: vector.hold_id.clone(),
        account_id: vector.account_id.clone(),
        asset: vector.asset.clone(),
        amount_minor: vector.amount_minor.clone(),
        state: hold_state(&vector.state),
    }
}

fn value_command(vector: &CommandVector) -> ValueCommand {
    ValueCommand {
        command_id: vector.command_id.clone(),
        contract_version: vector.contract_version.clone(),
        payload: vector.payload.clone(),
    }
}

fn balance(vector: BalanceVector) -> ReconciliationBalance {
    ReconciliationBalance {
        account_id: vector.account_id,
        asset: vector.asset,
        amount_minor: vector.amount_minor,
    }
}

#[test]
fn hold_lifecycle_matches_the_shared_corpus() {
    let fixture = fixture();
    assert_eq!(fixture.profile, "value-core-lifecycle-v1");
    let create = &fixture.hold.create;
    let hold = create_value_hold(
        &create.hold_id,
        &create.account_id,
        &create.asset,
        &create.amount_minor,
        &create.available_balance_minor,
    )
    .unwrap();
    assert_eq!(hold, value_hold(&fixture.hold.open));

    let expected = &fixture.hold.settlement;
    let settlement = settle_value_hold(
        &hold,
        &expected.destination_account_id,
        Some(&expected.amount_minor),
    )
    .unwrap();
    assert_eq!(settlement.hold.state, hold_state(&expected.state));
    assert_eq!(
        settlement.settled_amount_minor,
        expected.settled_amount_minor
    );
    assert_eq!(
        settlement.released_amount_minor,
        expected.released_amount_minor
    );
    assert_eq!(settlement.postings.len(), expected.postings.len());
    for (actual, expected) in settlement.postings.iter().zip(&expected.postings) {
        assert_eq!(actual.account_id, expected.account_id);
        assert_eq!(actual.asset, expected.asset);
        assert_eq!(actual.amount_minor, expected.amount_minor);
    }
    assert_eq!(
        release_value_hold(&hold).unwrap().state,
        hold_state(&fixture.hold.release_state)
    );

    for failure in fixture.hold.failures {
        let error = match failure.kind.as_str() {
            "over_available" => create_value_hold(
                &create.hold_id,
                &create.account_id,
                &create.asset,
                "101",
                &create.available_balance_minor,
            )
            .unwrap_err(),
            "same_destination" => settle_value_hold(&hold, &hold.account_id, None).unwrap_err(),
            "double_release" => {
                release_value_hold(&release_value_hold(&hold).unwrap()).unwrap_err()
            }
            other => panic!("unknown failure vector {other}"),
        };
        assert_eq!(error.message(), failure.error);
    }
}

#[test]
fn idempotency_matches_the_shared_corpus() {
    let fixture = fixture().idempotency;
    let existing = value_command(&fixture.existing);
    assert_eq!(
        create_value_command_digest(&existing).unwrap(),
        fixture.digest
    );
    let replay =
        resolve_value_command_replay(&existing, &value_command(&fixture.same_intent)).unwrap();
    assert_eq!(replay.status, "replayed");
    assert_eq!(replay.digest, fixture.digest);
    assert!(
        resolve_value_command_replay(&existing, &value_command(&fixture.changed_intent)).is_err()
    );
}

#[test]
fn reconciliation_matches_the_shared_corpus() {
    let fixture = fixture().reconciliation;
    let result = reconcile_balances(
        &fixture
            .expected
            .into_iter()
            .map(balance)
            .collect::<Vec<_>>(),
        &fixture.actual.into_iter().map(balance).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(result.closed, fixture.closed);
    assert_eq!(result.differences.len(), fixture.differences.len());
    for (actual, expected) in result.differences.iter().zip(fixture.differences) {
        assert_eq!(actual.account_id, expected.account_id);
        assert_eq!(actual.asset, expected.asset);
        assert_eq!(actual.expected_minor, expected.expected_minor);
        assert_eq!(actual.actual_minor, expected.actual_minor);
        assert_eq!(actual.difference_minor, expected.difference_minor);
    }
}
