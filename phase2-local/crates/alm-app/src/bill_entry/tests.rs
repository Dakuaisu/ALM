use super::*;
use alm_core::confirmation::owner_billing::{OwnerPosting, SaleTerms};
use alm_core::{DraftCustomer, Money, Role};

struct Root(std::path::PathBuf);

impl Root {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("alm-new-bill-{}-{serial}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn header(customer: &str, intent: Intent) -> Header {
    Header {
        customer: customer.into(),
        date: "2025-05-01".into(),
        loading: None,
        intent,
    }
}

fn effects(trial: &Trial) -> (usize, Option<u32>, Option<u32>, Money, Money, Money) {
    let authority = &trial.authority;
    (
        authority.posted_count(),
        authority.product_stock(ProductId::new(1)),
        authority.product_stock(ProductId::new(2)),
        authority.ledger(),
        authority.customer_due("batch1-customer"),
        authority.customer_due("batch1-market"),
    )
}

#[test]
fn manager_draft_reopens_with_nondefault_identity_and_exact_provisional_amounts() {
    let root = Root::new();
    let mut trial = Trial::open(&root.0, Some(crate::billing_tests::day())).unwrap();
    trial.set_owner(false).unwrap();
    trial.render("bill").unwrap();
    let before = effects(&trial);
    trial
        .bill_header(header("batch1-customer", Intent::Select))
        .unwrap();
    trial
        .bill_add(ProductId::new(1), quantity("1").unwrap())
        .unwrap();
    trial
        .bill_header(header("batch1-market", Intent::Select))
        .unwrap();
    trial
        .bill_add(ProductId::new(2), quantity("2").unwrap())
        .unwrap();
    assert_eq!(effects(&trial), before);
    assert!(trial.saved_ids().unwrap().is_empty());
    trial
        .bill_header(header("batch1-market", Intent::Save))
        .unwrap();
    assert_eq!(effects(&trial), before);
    drop(trial);

    let mut trial = Trial::open(&root.0, Some(crate::billing_tests::day())).unwrap();
    trial.set_owner(false).unwrap();
    trial.bill_open(trial.local_draft(1)).unwrap();
    let html = trial.render("bill").unwrap();
    for markup in [
        r#"name="customer_q" data-search="bill_customer_search" placeholder="Search customer…" autocomplete="off" value="Synthetic Market"><input type="hidden" name="customer" value="batch1-market">"#,
        r#"value="2025-05-01""#,
        r#"data-product="2" data-live="bill_live" data-target="bill-body"><input type="hidden" name="line" value="2"><b>Synthetic Soda"#,
        r#"<span class="num">₹59.00</span>"#,
        r#"<b>₹118.00</b>"#,
        "Bill Amount</span><b>₹236.00</b>",
    ] {
        assert!(html.contains(markup), "missing {markup}");
    }
    assert!(!html.contains("₹62.50"));
    assert!(!html.contains("₹38.00"));
    let html = trial.render("review").unwrap();
    assert!(html.contains("Access denied"));
    assert!(!html.contains("sale_prepare"));
}

#[test]
fn invalid_or_disconnected_choices_do_not_mutate_the_open_or_saved_draft() {
    let root = Root::new();
    let mut trial = Trial::open(&root.0, Some(crate::billing_tests::day())).unwrap();
    trial.render("bill").unwrap();
    trial.bill_add(ProductId::new(1), NonZeroU32::MIN).unwrap();
    trial
        .bill_header(header("batch1-customer", Intent::Save))
        .unwrap();
    let before = trial.current.clone();
    let ordinary = effects(&trial);
    for customer in ["", "Synthetic Market", "forged"] {
        assert!(trial.bill_header(header(customer, Intent::Save)).is_err());
    }
    assert!(
        trial
            .bill_add(ProductId::new(999), NonZeroU32::MIN)
            .is_err()
    );
    assert!(quantity("0").is_err());
    let connected = std::mem::replace(
        &mut *trial.authority,
        alm_core::confirmation::ConfirmationFixture::new(
            ProductId::new(1),
            0,
            Money::from_minor_units(0),
        )
        .unwrap(),
    );
    trial.refresh().unwrap();
    let disconnected = effects(&trial);
    assert!(
        trial
            .bill_header(header("batch1-market", Intent::Select))
            .is_err()
    );
    assert!(trial.bill_add(ProductId::new(2), NonZeroU32::MIN).is_err());
    assert_eq!(effects(&trial), disconnected);
    *trial.authority = connected;
    trial.refresh().unwrap();
    assert_eq!(trial.current, before);
    assert_eq!(effects(&trial), ordinary);
    trial.bill_open(trial.local_draft(1)).unwrap();
    assert_eq!(trial.current, before);
}

#[test]
fn only_explicit_owner_confirmation_posts_the_selected_customer_and_product() {
    let root = Root::new();
    let mut trial = Trial::open(&root.0, Some(crate::billing_tests::day())).unwrap();
    trial.render("bill").unwrap();
    trial
        .bill_add(ProductId::new(2), quantity("2").unwrap())
        .unwrap();
    trial
        .bill_header(header("batch1-market", Intent::Save))
        .unwrap();
    let before = effects(&trial);
    let (draft, _) = trial.current.as_ref().unwrap();
    let mut posting = OwnerPosting::default();
    let terms = SaleTerms {
        date: "2025-05-01",
        paid: "20.00",
    };
    posting
        .prepare(
            &mut trial.authority,
            &trial.directory,
            Role::Owner,
            Some(draft),
            terms,
        )
        .unwrap();
    assert_eq!(effects(&trial), before);
    let review = posting.pending(Role::Owner).unwrap();
    assert!(
        matches!(review.review().draft().customer(), DraftCustomer::Selected { id, .. } if id.value() == "batch1-market")
    );
    let serial = review.serial();
    assert!(
        posting
            .confirm(&mut trial.authority, Role::Manager, serial)
            .is_err()
    );
    assert_eq!(effects(&trial), before);
    let receipt = posting
        .confirm(&mut trial.authority, Role::Owner, serial)
        .unwrap();
    assert_eq!(receipt.total.minor_units(), 12_500);
    assert_eq!(effects(&trial).1, Some(50));
    assert_eq!(effects(&trial).2, Some(38));
    assert_eq!(effects(&trial).4.minor_units(), 0);
    assert_eq!(effects(&trial).5.minor_units(), 10_500);
}

#[test]
fn live_quantity_returns_core_totals_fragment_and_marks_the_draft_unsaved() {
    let root = Root::new();
    let mut trial = Trial::open(&root.0, Some(crate::billing_tests::day())).unwrap();
    trial.render("bill").unwrap();
    trial.bill_add(ProductId::new(1), NonZeroU32::MIN).unwrap();
    let html = trial.bill_live("1", "3").unwrap();
    assert!(html.starts_with(r#"<div id="bill-body">"#));
    assert!(html.contains(r#"id="bill-qty-1" name="qty" inputmode="numeric" value="3""#));
    assert!(html.contains("Bill Amount</span><b>₹354.00</b>"), "{html}");
    assert!(!trial.current.as_ref().unwrap().1);
    assert!(trial.bill_live("1", "0").is_err());
    assert!(trial.bill_live("9", "1").is_err());
    assert_eq!(trial.authority.posted_count(), 0);
}
