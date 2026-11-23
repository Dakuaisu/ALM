use super::*;
use alm_core::confirmation::trial_billing::{
    CUSTOMER, DAY, OwnerPosting, SaleTerms, customer_id, fixture,
};

fn review(fixture: &mut ConfirmationFixture) -> OwnerPosting {
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            fixture,
            Role::Owner,
            Some(&draft()),
            SaleTerms {
                date: DAY,
                paid: "20.00",
            },
        )
        .expect("review");
    posting
}

pub(crate) fn draft() -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: CUSTOMER.into(),
    })
}

#[test]
fn sales_show_confirmed_owner_amount_not_provisional_amount() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    assert_eq!(fixture.posted_count(), 0);
    let receipt = posting.confirm(&mut fixture, Role::Owner, 1).expect("post");
    assert_eq!(receipt.total.minor_units(), 12_050);
    assert_eq!(posting.confirm(&mut fixture, Role::Owner, 1), Ok(receipt));
    assert_eq!(fixture.posted_count(), 1);
    let html = views::sales_screen(&fixture, Role::Owner, &Default::default()).expect("sales");
    assert!(html.contains("<td class=\"num\">₹121</td>"));
    assert!(html.contains("₹20 bills"));
    assert!(html.contains("Synthetic Customer"));
}

#[test]
fn manager_cannot_confirm_an_owner_review_or_read_sales() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    assert!(posting.confirm(&mut fixture, Role::Manager, 1).is_err());
    assert_eq!(fixture.posted_count(), 0);
    let html = views::sales_screen(&fixture, Role::Manager, &Default::default()).expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("tbl-wrap"));
    assert!(!html.contains('₹'));
}

#[test]
fn standalone_receipt_changes_sales_collected_and_survives_cancellation() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    posting.confirm(&mut fixture, Role::Owner, 1).expect("post");
    crate::receipts::save(&mut fixture, Role::Owner, receipt_terms()).expect("receipt");
    let html = views::receipt_screen(&fixture, Role::Owner).expect("receipt screen");
    assert!(html.contains("₹70"));
    fixture
        .cancel_sale(Role::Owner, DraftId::new(1))
        .expect("cancel");
    let sales = views::sales_screen(&fixture, Role::Owner, &Default::default()).expect("sales");
    assert!(sales.contains("(₹0 bills · ₹31 received)"));
    assert!(sales.contains("CANCELLED"));
}

#[test]
fn manager_cannot_save_a_standalone_receipt_or_read_its_form() {
    let mut fixture = fixture().expect("synthetic fixture");
    assert!(crate::receipts::save(&mut fixture, Role::Manager, receipt_terms()).is_err());
    let html = views::receipt_screen(&fixture, Role::Manager).expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("<form"));
    assert_eq!(fixture.customer_due(customer_id().value()).minor_units(), 0);
}

fn receipt_terms() -> crate::receipts::Terms {
    crate::receipts::Terms {
        date: "2025-05-01".into(),
        amount: "30.50".into(),
        mode: crate::receipts::Mode::Online,
    }
}

#[test]
fn new_bill_renders_connected_choices_in_source_layout() {
    let root = std::env::temp_dir().join(format!("alm-bill-markup-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut trial = Trial::open(&root).unwrap();
    let html = trial.render("bill").unwrap();
    drop(trial);
    std::fs::remove_dir_all(root).unwrap();
    for markup in [
        "data-title=\"New Bill\"",
        "class=\"f3\"",
        "value=\"batch1-customer\"",
        "value=\"batch1-market\"",
        "value=\"1\">Synthetic Lime",
        "value=\"2\">Synthetic Soda",
        "Bill Amount",
    ] {
        assert!(html.contains(markup), "missing {markup}");
    }
}
