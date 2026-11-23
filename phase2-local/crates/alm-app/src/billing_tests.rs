use super::*;
use crate::billing::{self, Posting};

fn fixture() -> ConfirmationFixture {
    billing::fixture().expect("synthetic fixture")
}

pub(crate) fn draft() -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_customer(DraftCustomer::Typed(billing::CUSTOMER.into()))
}

#[test]
fn sales_show_confirmed_owner_amount_not_provisional_amount() {
    let mut fixture = fixture();
    let posting = Posting::prepare(&mut fixture, Role::Owner, &draft(), "2025-04-30", "2000", 1)
        .expect("review");
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
    let mut fixture = fixture();
    let posting = Posting::prepare(&mut fixture, Role::Owner, &draft(), "2025-04-30", "2000", 1)
        .expect("review");
    assert!(posting.confirm(&mut fixture, Role::Manager, 1).is_err());
    assert_eq!(fixture.posted_count(), 0);
    let html = views::sales_screen(&fixture, Role::Manager, &Default::default()).expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("tbl-wrap"));
    assert!(!html.contains('₹'));
}
