use serde_json::json;
use value_core::account::{AccountHistoryPosting, fold_account_history};
use value_core::amount::{
    ArithmeticOperation, evaluate_value_arithmetic, multiply_rational_floor, parse_amount_minor,
};
use value_core::canonical::{canonical_json, domain_separated_digest};
use value_core::conversion::{
    OriginalConversion, build_value_conversion_plan, plan_value_conversion_correction,
    validate_value_conversion_quote,
};
use value_core::hold::{HoldState, create_value_hold, release_value_hold, settle_value_hold};
use value_core::idempotency::{ValueCommand, resolve_value_command_replay};
use value_core::reconciliation::{ReconciliationBalance, reconcile_balances};
use value_core::statement::{StatementPosting, build_value_statement_page};
use value_core::transaction::{
    AccountBalance, CanonicalPosting, apply_balanced_transaction, create_posting_manifest_digest,
    create_transaction_reversal, validate_balanced_transaction, validate_posting_manifest,
};

fn transfer() -> Vec<CanonicalPosting> {
    vec![
        CanonicalPosting {
            account_id: "source".into(),
            asset: "credits".into(),
            amount_minor: "-25".into(),
        },
        CanonicalPosting {
            account_id: "destination".into(),
            asset: "credits".into(),
            amount_minor: "25".into(),
        },
    ]
}

#[test]
fn exact_arithmetic_enforces_signed_range_and_floor_rounding() {
    assert_eq!(parse_amount_minor("9223372036854775807").unwrap(), i64::MAX);
    assert_eq!(
        parse_amount_minor("-9223372036854775808").unwrap(),
        i64::MIN
    );
    for invalid in ["-0", "+1", "01", "9223372036854775808"] {
        assert!(parse_amount_minor(invalid).is_err());
    }
    assert!(
        evaluate_value_arithmetic(ArithmeticOperation::Add {
            left: i64::MAX.to_string(),
            right: "1".into(),
        })
        .is_err()
    );
    let rounded = multiply_rational_floor("-3", "1", "2").unwrap();
    assert_eq!(rounded.amount_minor, "-2");
    assert_eq!(rounded.remainder_numerator, "1");
}

#[test]
fn transfers_apply_atomically_and_reject_insufficient_value() {
    let balances = vec![
        AccountBalance {
            account_id: "source".into(),
            asset: "credits".into(),
            balance_minor: "100".into(),
            allow_negative: false,
        },
        AccountBalance {
            account_id: "destination".into(),
            asset: "credits".into(),
            balance_minor: "5".into(),
            allow_negative: false,
        },
    ];
    let applied = apply_balanced_transaction(&balances, &transfer()).unwrap();
    assert_eq!(applied[0].balance_minor, "75");
    assert_eq!(applied[1].balance_minor, "30");
    assert_eq!(balances[0].balance_minor, "100");

    let mut insufficient = balances;
    insufficient[0].balance_minor = "20".into();
    assert!(
        apply_balanced_transaction(&insufficient, &transfer())
            .unwrap_err()
            .message()
            .contains("insufficient value")
    );

    let invalid_opening = vec![
        AccountBalance {
            account_id: "source".into(),
            asset: "credits".into(),
            balance_minor: "-5".into(),
            allow_negative: false,
        },
        AccountBalance {
            account_id: "destination".into(),
            asset: "credits".into(),
            balance_minor: "10".into(),
            allow_negative: false,
        },
    ];
    let healing_postings = vec![
        CanonicalPosting {
            account_id: "source".into(),
            asset: "credits".into(),
            amount_minor: "5".into(),
        },
        CanonicalPosting {
            account_id: "destination".into(),
            asset: "credits".into(),
            amount_minor: "-5".into(),
        },
    ];
    assert!(
        apply_balanced_transaction(&invalid_opening, &healing_postings)
            .unwrap_err()
            .message()
            .contains("cannot start with negative value")
    );
}

#[test]
fn conservation_reversal_and_manifest_are_exact() {
    let postings = transfer();
    assert_eq!(
        validate_balanced_transaction(&postings)
            .unwrap()
            .total_minor,
        "0"
    );
    assert_eq!(
        create_transaction_reversal(&postings).unwrap()[0].amount_minor,
        "25"
    );
    let digest = create_posting_manifest_digest(&postings).unwrap();
    assert!(validate_posting_manifest(2, &postings, true, &digest).is_ok());
    assert!(
        validate_posting_manifest(2, &postings, false, &digest)
            .unwrap_err()
            .message()
            .contains("closed before commit")
    );
    assert!(
        validate_posting_manifest(3, &postings, true, &digest)
            .unwrap_err()
            .message()
            .contains("count does not match")
    );
    assert!(
        validate_posting_manifest(2, &postings, true, &"0".repeat(64))
            .unwrap_err()
            .message()
            .contains("digest mismatch")
    );

    let mut unbalanced = postings;
    unbalanced[1].amount_minor = "24".into();
    assert!(validate_balanced_transaction(&unbalanced).is_err());
}

#[test]
fn holds_enforce_available_value_and_lifecycle() {
    let hold = create_value_hold("hold-1", "buyer", "credits", "40", "100").unwrap();
    let settlement = settle_value_hold(&hold, "seller", Some("30")).unwrap();
    assert_eq!(settlement.hold.state, HoldState::Settled);
    assert_eq!(settlement.released_amount_minor, "10");
    assert_eq!(settlement.postings.len(), 2);
    assert!(create_value_hold("hold-2", "buyer", "credits", "101", "100").is_err());
    let released = release_value_hold(&hold).unwrap();
    assert!(release_value_hold(&released).is_err());
    let forged = value_core::hold::ValueHold {
        amount_minor: "0".into(),
        ..hold
    };
    assert!(release_value_hold(&forged).is_err());
}

#[test]
fn duplicate_commands_require_identical_semantic_intent() {
    let existing = ValueCommand {
        command_id: "command-1".into(),
        contract_version: "v1".into(),
        payload: json!({"asset": "credits", "amountMinor": "7"}),
    };
    assert_eq!(
        resolve_value_command_replay(&existing, &existing)
            .unwrap()
            .status,
        "replayed"
    );
    let incoming = ValueCommand {
        payload: json!({"asset": "credits", "amountMinor": "8"}),
        ..existing.clone()
    };
    assert!(resolve_value_command_replay(&existing, &incoming).is_err());
}

#[test]
fn account_history_and_reconciliation_expose_exact_differences() {
    assert_eq!(
        fold_account_history(
            "10",
            &[AccountHistoryPosting {
                sequence: 1,
                balance_before_minor: "10".into(),
                amount_minor: "5".into(),
                balance_after_minor: "15".into(),
            }],
            false,
        )
        .unwrap()
        .final_balance_minor,
        "15"
    );
    assert!(fold_account_history("-1", &[], false).is_err());
    let result = reconcile_balances(
        &[ReconciliationBalance {
            account_id: "a".into(),
            asset: "credits".into(),
            amount_minor: "10".into(),
        }],
        &[ReconciliationBalance {
            account_id: "a".into(),
            asset: "credits".into(),
            amount_minor: "8".into(),
        }],
    )
    .unwrap();
    assert!(!result.closed);
    assert_eq!(result.differences[0].difference_minor, "-2");
}

#[test]
fn conversion_and_statement_facts_remain_balanced_and_continuous() {
    let plan = build_value_conversion_plan(
        "quote-1", "credits", "points", "10", "25", "5", "2", "floor",
    )
    .unwrap();
    assert_eq!(plan.transactions.len(), 2);
    assert_eq!(plan.transactions[0].postings[0].amount_minor, "-10");
    assert_eq!(plan.transactions[1].postings[1].amount_minor, "25");
    assert!(
        build_value_conversion_plan(
            "quote-small",
            "credits",
            "points",
            "1",
            "0",
            "1",
            "2",
            "floor",
        )
        .unwrap_err()
        .message()
        .contains("destinationAmountMinor must be positive")
    );
    assert!(
        validate_value_conversion_quote(
            "quote-date",
            "actor-1",
            "actor-1",
            "rate-1",
            "rate-1",
            "2026-02-01T00:00:00.000Z",
            "2026-02-31T00:00:00.000Z",
        )
        .unwrap_err()
        .message()
        .contains("RFC 3339 instant")
    );
    assert!(
        plan_value_conversion_correction(
            "literal_reversal",
            OriginalConversion {
                source_asset: "credits".into(),
                source_amount_minor: "1".into(),
                destination_asset: "points".into(),
                destination_amount_minor: "0".into(),
                rate_snapshot_id: "rate-1".into(),
            },
        )
        .is_err()
    );

    let statement = build_value_statement_page(
        "account-1",
        "credits",
        "0",
        "10",
        10,
        &[StatementPosting {
            posting_id: "posting-1".into(),
            transaction_id: "transaction-1".into(),
            account_id: "account-1".into(),
            account_sequence: "1".into(),
            posting_sequence: 1,
            asset: "credits".into(),
            amount_minor: "5".into(),
            balance_before_minor: "10".into(),
            balance_after_minor: "15".into(),
            occurred_at: "2026-01-01T00:00:00.000Z".into(),
            recorded_at: "2026-01-01T00:00:00.000Z".into(),
            source_namespace: "marketplace".into(),
            source_type: "sale".into(),
            source_id: "sale-1".into(),
        }],
    )
    .unwrap();
    assert_eq!(statement.closing_balance_minor, "15");
}

#[test]
fn canonical_evidence_matches_the_cross_language_golden_vector() {
    let value = json!({"z": [3, {"b": true, "a": "x"}], "": 2, "😀": 1});
    assert_eq!(
        canonical_json(&value).unwrap(),
        "{\"z\":[3,{\"a\":\"x\",\"b\":true}],\"😀\":1,\"\":2}"
    );
    assert_eq!(
        domain_separated_digest("value-core/test", "v1", &value).unwrap(),
        "d422be436e96980b0c5d83b09c9e6049d4a1834c16e68fe874bf57b9b7b5de62"
    );
}
