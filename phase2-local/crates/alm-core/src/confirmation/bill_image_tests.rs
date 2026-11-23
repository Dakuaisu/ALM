use std::num::NonZeroU32;

use super::{BillImageParty, DrawOp, FontStyle};
use crate::confirmation::demo_fixture::{customer_id, directory, fixture};
use crate::confirmation::owner_billing::{OwnerPosting, SaleTerms};
use crate::confirmation::{
    BalanceMode, ConfirmationError, EstimateDisplay, EstimateError, EstimateLineDisplay,
    EstimateRequest,
};
use crate::{BillDate, DraftCustomer, DraftId, DraftLine, ProductId, ProvisionalDraft, Role};

/// A fixed-advance stand-in for font metrics: half the font size per character.
#[allow(clippy::cast_precision_loss)]
fn measure(text: &str, size: u32, _: FontStyle) -> f32 {
    text.chars().count() as f32 * size as f32 * 0.5
}

const PARTY: BillImageParty<'static> = BillImageParty {
    business: "ALM Enterprises (synthetic)",
    tagline: "Synthetic cold drinks",
    address: "Synthetic Market Road, Near The Old Clock Tower, Lucknow, Uttar Pradesh",
    mobile: "90000 00000",
    profile: None,
    customer_mobile: "",
    customer_address: "",
};

const LABELS: [EstimateLineDisplay<'static>; 1] = [EstimateLineDisplay {
    product: ProductId::new(1),
    label: "Synthetic Lime",
}];

fn request(role: Role, mode: BalanceMode) -> EstimateRequest<'static> {
    EstimateRequest {
        role,
        draft_id: DraftId::new(1),
        balance_mode: mode,
        display: EstimateDisplay {
            today: BillDate::new(2025, 5, 2).unwrap(),
            lines: &LABELS,
        },
    }
}

fn posted() -> crate::confirmation::ConfirmationFixture {
    let mut f = fixture().unwrap();
    let draft = ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(2).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: "Synthetic Customer".into(),
    });
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            &mut f,
            &directory(),
            Role::Owner,
            Some(&draft),
            SaleTerms {
                date: "2025-04-30",
                paid: "20",
            },
        )
        .unwrap();
    posting.confirm(&mut f, Role::Owner, 1).unwrap();
    f
}

#[test]
fn golden_display_list_for_the_fixture_bill() {
    let f = posted();
    let image = f
        .bill_image(
            request(Role::Owner, BalanceMode::AsOnBillDate),
            &PARTY,
            &measure,
        )
        .unwrap();
    assert_eq!((image.width, image.height, image.scale), (760, 427, 2));
    assert_eq!(image.pixels(), (1520, 854));
    let golden = include_str!("bill_image_golden.txt");
    assert_eq!(format!("{:#?}\n", image.ops), golden);
}

#[test]
fn latest_mode_and_manager_denial() {
    let f = posted();
    let image = f
        .bill_image(request(Role::Owner, BalanceMode::Latest), &PARTY, &measure)
        .unwrap();
    let texts: Vec<_> = image
        .ops
        .iter()
        .filter_map(|op| match op {
            DrawOp::Text { text, color, .. } => Some((text.as_str(), *color)),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&("Paid on this bill", 0x000000)));
    assert!(texts.contains(&("Total Due (as on 02-May-2025)", 0xb91c1c)));
    assert_eq!(
        f.bill_image(
            request(Role::Manager, BalanceMode::Latest),
            &PARTY,
            &measure
        ),
        Err(EstimateError::Preview(ConfirmationError::AccessDenied))
    );
}
