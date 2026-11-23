use super::*;
use alm_store::DraftLine;
use std::num::NonZeroU32;

#[test]
fn sqlite_lost_response_retries_fifo_without_reallocation_or_cost_disclosure() {
    let scratch = Scratch::new();
    let database = scratch.0.join(DATABASE);
    let draft = dated_named_draft(
        DraftId::new(43),
        [2, 3]
            .into_iter()
            .map(|quantity| DraftLine::new(ProductId::new(1), NonZeroU32::new(quantity).unwrap()))
            .collect(),
    );
    let mut sales = open_sales(&database);
    sales
        .session(Role::Manager)
        .save_draft(draft.clone())
        .unwrap();
    let mut authority = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[
            (3, Money::from_minor_units(80)),
            (4, Money::from_minor_units(20)),
        ],
        Money::from_minor_units(125),
    )
    .unwrap();
    let approved = OwnerRequest {
        id: RequestId::new(987),
        review: authority
            .reconnect_review(Role::Owner, draft.clone())
            .unwrap(),
    };
    let mut first = Client::start("sqlite_submit", &scratch.0);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    assert_eq!(
        receipt,
        Receipt {
            draft_id: DraftId::new(43),
            invoice: 1,
            quantity: 5,
            total: Money::from_minor_units(625),
            received: None,
            due: None,
            customer_balance: None,
        }
    );
    assert_eq!(
        sales
            .session(Role::Manager)
            .reopen_draft(DraftId::new(43))
            .unwrap(),
        Some(draft)
    );
    let sql = "SELECT m.name || ':' || (SELECT group_concat(name, ',') FROM (SELECT name FROM pragma_table_info(m.name) ORDER BY cid)) FROM sqlite_schema m WHERE m.type='table' ORDER BY m.name; SELECT 'draft|' || id FROM provisional_drafts; SELECT 'line|' || draft_id || '|' || position || '|' || product_id || '|' || quantity FROM provisional_draft_lines ORDER BY position; SELECT 'pending|' || request_id || '|' || snapshot FROM pending_owner_confirmations; SELECT 'customers|' || count(*) FROM provisional_draft_customers; SELECT 'crates|' || count(*) FROM provisional_draft_crates;";
    let output =
        sqlite_list::sqlite_list(&sqlite_list::open_read_only(&database).unwrap(), sql).unwrap();
    assert_eq!(
        output,
        concat!(
            "pending_owner_confirmations:request_id,snapshot\n",
            "provisional_draft_crates:draft_id,definition\n",
            "provisional_draft_customers:draft_id,kind,customer_id,name\n",
            "provisional_draft_dates:draft_id,bill_date\n",
            "provisional_draft_lines:draft_id,position,product_id,quantity\n",
            "provisional_drafts:id\n",
            "draft|43\nline|43|0|1|2\nline|43|1|1|3\n",
            "pending|987|6|2024-02-29|2|43|1|0|125,125|1,,53796E746865746963|1,2;1,3\n",
            "customers|1\ncrates|0\n",
        )
    );
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    let committed = authority.catalog().allocation_snapshot();
    assert_eq!(
        committed,
        vec![
            (43, 0, 0, 2, Some(160)),
            (43, 1, 0, 1, Some(80)),
            (43, 1, 1, 2, Some(40))
        ]
    );
    assert_eq!(
        authority.catalog().lot_snapshot(ProductId::new(1)),
        vec![0, 2]
    );
    assert_eq!(
        (
            authority.catalog().allocated_cost(DraftId::new(43)),
            authority.ledger().minor_units()
        ),
        (Some(Some(280)), 625)
    );
    let mut retry = Client::start("sqlite_retry", &scratch.0);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    let replay = authority
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, receipt);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&receipt))]
    );
    assert_eq!(authority.catalog().allocation_snapshot(), committed);
    assert_eq!(
        authority.catalog().lot_snapshot(ProductId::new(1)),
        vec![0, 2]
    );
    assert_eq!(
        (
            authority.catalog().allocated_cost(DraftId::new(43)),
            effects(&authority)
        ),
        (Some(Some(280)), (2, 625, 2, 1, 1))
    );
}
