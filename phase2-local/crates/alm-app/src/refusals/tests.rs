use std::num::NonZeroU32;

use alm_core::confirmation::ConfirmationError;
use alm_core::confirmation::owner_billing::{NewBillPricingError, OwnerBillingError};
use alm_core::provisional_preview::ProvisionalPreviewError;
use alm_core::{DraftId, ProductId, ProvisionalDraft};
use alm_sync::AuthorityError;

use super::{NO_ITEM, pricing, user_message};
use crate::Trial;
use crate::bill_entry::{Header, HeaderIntent, Line};
use crate::billing_tests::day;
use crate::screen_tests::Root;

/// Every variant name of the core, sync and store errors that can reach a screen.
const VARIANTS: &[&str] = &[
    "AccessDenied",
    "InvalidAmount",
    "InvalidDraft",
    "UnsupportedSalesRecord",
    "DraftNotFound",
    "MissingBillableItems",
    "MissingCustomerName",
    "MissingBillDate",
    "StaleReview",
    "RequestConflict",
    "DraftAlreadyPosted",
    "InsufficientStock",
    "Overflow",
    "UnknownProduct",
    "UnsupportedCrateRow",
    "UnsupportedRounding",
    "Confirmation",
    "NoSavedDraft",
    "NoReview",
    "ReviewLimit",
    "LoadingCharge",
    "NotJson",
    "NoProducts",
    "PickCustomer",
    "NotQuarantined",
    "AlreadyResolved",
    "NoReason",
    "AlreadyImported",
    "Unresolved",
    "DigestMismatch",
    "ActiveDraftMismatch",
    "DurableStoreRequired",
    "InvalidSnapshot",
    "InvalidMetadata",
    "ItemNameRequired",
    "InvalidCases",
    "MissingItems",
    "MissingCustomer",
    "MissingDate",
    "InvalidGstin",
    "InvalidEway",
    "RowNotFound",
    "InvalidLineDisplay",
    "DateOutOfRange",
    "UnsupportedRecord",
    "InvalidPrice",
    "MissingPieceMrp",
    "OrderNotFound",
    "OrderNotPending",
    "StaleOrderNote",
    "NoUsableRows",
    "MissingSupplier",
    "InvalidReceipt",
    "ReceiptOverflow",
    "UnknownFrozenCost",
    "ProductNotFound",
    "ProductIdInUse",
    "ProductInUse",
    "DuplicateName",
    "MissingSaleDate",
    "InvalidCount",
    "InvalidFridgeCount",
    "EmptyCart",
    "QuantityOutOfRange",
    "AmountOverflow",
    "SupplierNotFound",
    "SaleNotFound",
    "MissingTax",
    "InvalidArchive",
    "VersionOverflow",
    "AlreadyPosted",
    "Rejected",
    "Denied",
    "Unavailable",
    "SignedOut",
    "Refused",
    "InvalidPath",
    "InvalidAccountScope",
    "AccountStoreMismatch",
    "TooManyLines",
    "InvalidOutput",
    "ProductEntry",
    "Calculation",
    "Corrupt",
    "Storage",
    "Sqlite",
];

/// No type name, no debug braces or paths, not empty.
fn assert_clean(message: &str, from: &str) {
    assert!(!message.trim().is_empty(), "{from}: empty");
    assert!(
        !message.contains(['{', '}']) && !message.contains("::"),
        "{from}: debug output in {message:?}"
    );
    for word in message.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        assert!(
            !VARIANTS.contains(&word),
            "{from}: type name {word} in {message:?}"
        );
        let camel = word
            .as_bytes()
            .windows(2)
            .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase());
        assert!(!camel, "{from}: type-like word {word} in {message:?}");
    }
}

#[test]
fn every_refusal_reads_as_words_not_debug_output() {
    let mut samples: Vec<String> = VARIANTS
        .iter()
        .flat_map(|v| {
            [
                (*v).to_owned(),
                format!("Confirmation({v})"),
                format!("Rejected(\"{v}\")"),
                format!("{v} {{ existing: \"Lime\" }}"),
            ]
        })
        .collect();
    samples.extend([
        format!(
            "{:?}",
            OwnerBillingError::Confirmation(ConfirmationError::StaleReview)
        ),
        format!(
            "{:?}",
            NewBillPricingError::Items(ProvisionalPreviewError::UnknownProduct(ProductId::new(9)))
        ),
        format!(
            "{:?}",
            AuthorityError::Rejected("invalid_terms: whole packs".into())
        ),
        format!(
            "{:?}",
            AuthorityError::Rejected("weird \"quoted\" {thing}".into())
        ),
        format!("{:?}", AuthorityError::Unavailable),
        "Io(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })".into(),
        "SomethingNew(Inner { x: 1 })".into(),
    ]);
    for sample in &samples {
        assert_clean(&user_message(sample), sample);
    }
    for reason in [
        "InvalidDraft",
        "invalid_terms: whole packs",
        "Insufficient stock {x}",
    ] {
        let error = AuthorityError::Rejected(reason.into());
        assert_clean(&crate::device::refused(&error), reason);
    }
    assert_eq!(user_message("MissingBillableItems"), NO_ITEM);
    assert_eq!(
        pricing(&NewBillPricingError::Crates(
            ConfirmationError::InvalidAmount
        )),
        "The crate's pieces don't make whole packets."
    );
}

#[test]
fn add_item_on_an_empty_row_says_what_the_html_says() {
    for payload in [
        r#"{"product":"","quantity":"1"}"#,
        r#"{"product":" ","quantity":""}"#,
        r#"{"quantity":"1"}"#,
        r#"{"product":"abc","quantity":"1"}"#,
    ] {
        let line = tauri::ipc::InvokeResponseBody::Json(payload.into())
            .deserialize::<Line>()
            .expect("the form always decodes");
        assert_eq!(line.picked().unwrap_err(), NO_ITEM, "{payload}");
    }
    let line = tauri::ipc::InvokeResponseBody::Json(r#"{"product":"1","quantity":"2"}"#.into())
        .deserialize::<Line>()
        .unwrap();
    assert_eq!(
        line.picked().unwrap(),
        (ProductId::new(1), NonZeroU32::new(2).unwrap())
    );
}

const TABS: &[&str] = &[
    "bill",
    "review",
    "sales",
    "customers",
    "expenses",
    "dashboard",
    "reorder",
    "stock",
    "godown",
    "gstsales",
    "products",
    "suppliers",
    "order",
    "purchases",
    "crates",
    "ratelist",
    "reports",
    "settings",
    "counter",
    "receipt",
    "rateprint",
    "gst",
    "estimate",
    "poprint",
];

/// Refused commands and every screen, as the Owner and as the Manager, show words only.
#[test]
fn refused_commands_and_rendered_screens_show_no_type_names() {
    let root = Root::new("refusal-words");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let mut errors = Vec::new();
    for owner in [true, false] {
        trial.set_owner(owner).unwrap();
        trial.current = None;
        trial.render("bill").unwrap();
        errors.push(
            trial
                .bill_add(ProductId::new(999), NonZeroU32::MIN)
                .unwrap_err(),
        );
        errors.push(
            trial
                .bill_header(Header::new(
                    "batch1-customer",
                    "2025-04-30",
                    None,
                    HeaderIntent::Save,
                ))
                .unwrap_err(),
        );
        errors.push(
            trial
                .bill_header(Header::new(
                    "nobody",
                    "2025-04-30",
                    None,
                    HeaderIntent::Select,
                ))
                .unwrap_err(),
        );
        errors.push(
            trial
                .bill_header(Header::new(
                    "batch1-customer",
                    "x",
                    None,
                    HeaderIntent::Select,
                ))
                .unwrap_err(),
        );
        errors.push(trial.bill_crate_post("0").unwrap_err());
        errors.push(trial.bill_crate_add(999, NonZeroU32::MIN).unwrap_err());
        errors.push(
            trial
                .sale_prepare(&crate::billing::Terms {
                    date: "2025-04-30".into(),
                    paid: "abc".into(),
                })
                .unwrap_err(),
        );
        errors.push(trial.sale_confirm(42).unwrap_err());
        errors.push(trial.estimate_html(DraftId::new(999)).unwrap_err());
        errors.push(trial.bill_image(DraftId::new(999)).map(|_| ()).unwrap_err());
        trial
            .edit_bill(|draft| {
                *draft = ProvisionalDraft::new(
                    draft.id(),
                    vec![alm_core::DraftLine::new(
                        ProductId::new(999),
                        NonZeroU32::MIN,
                    )],
                );
                Ok(true)
            })
            .unwrap();
        for tab in TABS {
            let html = trial.render(tab).unwrap_or_else(|error| error);
            for word in VARIANTS {
                let hit = html.match_indices(word).any(|(at, _)| {
                    let before = html[..at].chars().next_back().unwrap_or(' ');
                    let after = html[at + word.len()..].chars().next().unwrap_or(' ');
                    !before.is_ascii_alphanumeric() && !after.is_ascii_alphanumeric()
                });
                assert!(!hit, "{tab} (owner {owner}) shows {word}");
            }
        }
        errors.push(trial.message.clone());
    }
    for error in errors.iter().filter(|e| !e.is_empty()) {
        assert_clean(error, error);
    }
}
