use serde_json::json;
use value_core::conversion::{
    DurableConversionReceipt, UnknownConversionResolution, resolve_unknown_value_conversion,
    resolve_value_conversion_quote_replay, settle_value_conversion_execution,
};
use value_core::fact::{OrderedValueFact, order_value_facts};
use value_core::rate::{
    ValueRateSnapshotInput, create_value_rate_snapshot, evaluate_value_rate_freshness,
};
use value_core::statement::{normalize_statement_timestamp, try_canonical_statement_timestamp};
use value_core::time::{format_rfc3339_millis, parse_rfc3339_millis, try_format_rfc3339_millis};
use value_core::transaction::{
    CanonicalPosting, PostingManifestDigestProfile, create_posting_manifest_digest_with_profile,
    create_transaction_reversal, validate_posting_manifest_with_profile,
    validate_single_asset_transaction,
};

fn transfer() -> Vec<CanonicalPosting> {
    vec![
        CanonicalPosting {
            account_id: "source".into(),
            asset: "credits".into(),
            amount_minor: "-10".into(),
        },
        CanonicalPosting {
            account_id: "destination".into(),
            asset: "credits".into(),
            amount_minor: "10".into(),
        },
    ]
}

#[test]
fn conversion_replay_unknown_receipts_and_partial_execution_are_explicit() {
    let quote = json!({"quoteId": "quote-1", "amountMinor": "10"});
    assert_eq!(
        resolve_value_conversion_quote_replay(&quote, &quote)
            .unwrap()
            .status,
        "replayed"
    );
    assert!(
        resolve_value_conversion_quote_replay(
            &quote,
            &json!({"quoteId": "quote-1", "amountMinor": "11"})
        )
        .is_err()
    );

    assert!(matches!(
        resolve_unknown_value_conversion("command-1", None).unwrap(),
        UnknownConversionResolution::Unknown {
            resubmit_allowed: false
        }
    ));
    assert!(matches!(
        resolve_unknown_value_conversion(
            "command-1",
            Some(&DurableConversionReceipt {
                status: "succeeded".into(),
                transaction_ids: vec!["transaction-1".into()],
            }),
        )
        .unwrap(),
        UnknownConversionResolution::Succeeded { .. }
    ));

    let execution =
        settle_value_conversion_execution("10", "7", "14", "return_unexecuted_source").unwrap();
    assert_eq!(execution.returned_source_minor, "3");
    assert!(settle_value_conversion_execution("10", "7", "14", "forbidden").is_err());
}

#[test]
fn single_asset_and_custom_manifest_profiles_are_enforced() {
    let postings = transfer();
    assert_eq!(
        validate_single_asset_transaction("credits", &postings, false)
            .unwrap()
            .asset,
        "credits"
    );
    assert!(validate_single_asset_transaction("points", &postings, false).is_err());
    assert!(validate_single_asset_transaction("credits", &postings, true).is_err());

    let profile = PostingManifestDigestProfile {
        domain: "consumer/posting-manifest",
        contract_version: "v2",
    };
    let digest = create_posting_manifest_digest_with_profile(&postings, profile).unwrap();
    assert!(validate_posting_manifest_with_profile(2, &postings, true, &digest, profile).is_ok());
}

#[test]
fn literal_reversal_rejects_the_unrepresentable_signed_minimum() {
    assert!(
        create_transaction_reversal(&[
            CanonicalPosting {
                account_id: "minimum".into(),
                asset: "credits".into(),
                amount_minor: "-9223372036854775808".into(),
            },
            CanonicalPosting {
                account_id: "maximum".into(),
                asset: "credits".into(),
                amount_minor: "9223372036854775807".into(),
            },
            CanonicalPosting {
                account_id: "remainder".into(),
                asset: "credits".into(),
                amount_minor: "1".into(),
            },
        ])
        .is_err()
    );
}

#[test]
fn checked_rfc3339_formatting_matches_the_typescript_calendar_range() {
    let millis = parse_rfc3339_millis("2026-01-01T09:00:00+09:00", "Value timestamp").unwrap();
    assert_eq!(
        try_format_rfc3339_millis(millis, "Value timestamp").unwrap(),
        "2026-01-01T00:00:00.000Z"
    );
    assert_eq!(
        normalize_statement_timestamp("2026-01-01T09:00:00+09:00").unwrap(),
        "2026-01-01T00:00:00.000Z"
    );
    assert_eq!(
        try_canonical_statement_timestamp(253_402_300_799_999).unwrap(),
        "9999-12-31T23:59:59.999Z"
    );
    assert!(try_format_rfc3339_millis(253_402_300_800_000, "Value timestamp").is_err());
    assert_eq!(
        format_rfc3339_millis(253_402_300_800_000),
        "10000-01-01T00:00:00.000Z"
    );
}

#[test]
fn rate_and_fact_bounds_preserve_cross_language_inputs() {
    assert!(
        create_value_rate_snapshot(ValueRateSnapshotInput {
            snapshot_id: "rate-upper-bound".into(),
            base_asset: "credits".into(),
            quote_asset: "points".into(),
            numerator: "1".into(),
            denominator: "1".into(),
            observed_at: "9999-12-31T23:59:59.000Z".into(),
            recorded_at: "9999-12-31T23:59:59.000Z".into(),
            effective_at: "9999-12-31T23:59:59.000Z".into(),
            max_staleness_seconds: 1,
        })
        .is_err()
    );
    assert!(
        evaluate_value_rate_freshness(
            "rate-1",
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.000Z",
            9_007_199_254_740_992,
        )
        .is_err()
    );
    assert!(
        order_value_facts(&[OrderedValueFact {
            id: "fact-1".into(),
            sequence: 9_007_199_254_740_992,
            occurred_at: "2026-01-01T00:00:00.000Z".into(),
        }])
        .is_err()
    );
}
