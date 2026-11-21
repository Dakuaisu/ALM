use crate::confirmation::ConfirmationError;
use crate::{DraftCustomerId, Role};

mod projection;
pub(super) mod seed;

use projection::{Period, Source, money_r, round_rupees, sales_history};
use seed::{GROVE_ID, OWNER, date, history_fixture};

type Case = (
    &'static str,
    Period,
    &'static str,
    &'static [u64],
    &'static [u64],
    &'static [usize],
    [i64; 5],
);

#[test]
fn l04_sales_history_replay_filters_bills_and_receipts_independently() {
    // Given: a fixed today, backdated postings, a cancellation, and separate receipts.
    use Period::{All, Custom, ThisMonth, Today};
    let fixture = history_fixture();
    let before = fixture.clone();
    let today = date("2025-03-15");
    let first = date("2025-03-01");
    let receipt_day = date("2025-03-10");
    let bounded = Custom((Some(first), Some(today)));
    let receipt_only = Custom((Some(receipt_day), Some(receipt_day)));
    #[rustfmt::skip]
    let cases: &[Case] = &[
        (
            "Today", Today, "", &[58, 24, 36], &[58, 36], &[4], [2548, 149, 151, 300, 998],
        ),
        (
            "This Month", ThisMonth, "", &[58, 24, 36, 73, 41], &[58, 36, 73, 41],
            &[1, 2, 4, 5, 6, 7], [6145, 447, 499, 946, 2445],
        ),
        (
            "All", All, "", &[58, 24, 36, 73, 12, 41], &[58, 36, 73, 12, 41],
            &[1, 2, 3, 4, 5, 6, 7], [7194, 522, 550, 1072, 2894],
        ),
        (
            "Custom", bounded, "", &[58, 24, 36, 73], &[58, 36, 73], &[1, 2, 4, 6, 7],
            [4047, 298, 450, 748, 1547],
        ),
        (
            "Start only", Custom((Some(receipt_day), None)), "", &[58, 24, 36, 41],
            &[58, 36, 41], &[1, 2, 4, 5, 6], [4646, 298, 449, 747, 1896],
        ),
        (
            "End only", Custom((None, Some(today))), "", &[58, 24, 36, 73, 12],
            &[58, 36, 73, 12], &[1, 2, 3, 4, 6, 7], [5096, 373, 501, 874, 1996],
        ),
        (
            "Reversed", Custom((Some(today), Some(first))), "", &[], &[], &[], [0; 5],
        ),
        (
            "Party", bounded, "gRoVe", &[36, 73], &[36, 73], &[1, 6, 7],
            [2548, 298, 250, 548, 998],
        ),
        (
            "Item only", bounded, "LiMe", &[58, 24, 73], &[58, 73], &[], [2998, 149, 0, 149, 1098],
        ),
        (
            "Receipt-only date", receipt_only, "", &[], &[], &[1, 2, 6], [0, 0, 249, 249, 0],
        ),
        (
            "Receipt-only party", receipt_only, "gRoVe", &[], &[], &[1, 6], [0, 0, 200, 200, 0],
        ),
    ];

    for &(label, period, search, rows, active, receipt_ids, raw) in cases {
        // When: the same history is read under each Sales filter.
        let history = sales_history(&fixture, period.bounds(today), search);
        let profit = history.profit(OWNER).unwrap();
        let collected = history.collected();

        // Then: membership, order, drill inputs and exact money match independent literals.
        let row_ids = history.row_ids();
        assert_eq!(row_ids, rows, "{label}: row order");
        assert_eq!(profit.bill_ids, active, "{label}: profit inputs");
        assert_eq!(collected.bill_ids, active, "{label}: collected bill inputs");
        assert_eq!(
            collected.receipt_ids, receipt_ids,
            "{label}: standalone inputs"
        );
        let [sales, paid, received, total] = history.tile_paise();
        let observed = [sales, paid, received, total, profit.profit_paise()];
        assert_eq!(observed, raw, "{label}: raw paise");
        assert_eq!(
            collected.cash_paise + collected.online_paise,
            total,
            "{label}: detail total"
        );
        println!(
            "L04 {label}: rows={row_ids:?} profit_bills={:?} collected_bills={:?} receipts={:?} raw_paise(sales,bill_paid,receipt_paid,collected,profit)={observed:?}",
            profit.bill_ids, collected.bill_ids, collected.receipt_ids
        );
    }

    let history = sales_history(&fixture, Today.bounds(today), "");
    let active_packets = history.active_packet_count();
    assert_eq!(active_packets, 2);
    let balances: Vec<_> = history
        .rows
        .iter()
        .map(|row| {
            (
                row.bill.draft_id.value(),
                row.cancelled,
                row.remainder_paise(),
                row.balance_paise,
            )
        })
        .collect();
    assert_eq!(
        balances,
        [
            (58, false, 1499, 1499),
            (24, true, 1248, 0),
            (36, false, 900, 3849),
        ]
    );
    let ledger = fixture
        .customer_ledger(OWNER, &DraftCustomerId::new(GROVE_ID.into()))
        .unwrap();
    assert_eq!(ledger.closing.minor_units(), 3849);
    println!(
        "L04 Today active packets={active_packets}; balances (bill,cancelled,remainder_paise,shown_paise)={balances:?}; Grove ledger closing={}p",
        ledger.closing.minor_units()
    );
    let denied = history.profit(Role::Manager);
    assert_eq!(denied, Err(ConfirmationError::AccessDenied));
    println!("L04 Manager profit: {denied:?} (no payload)");

    let custom = sales_history(&fixture, bounded.bounds(today), "");
    let profit = custom.profit(OWNER).unwrap();
    let collected = custom.collected();
    use Source::{Bill, Receipt};
    assert_eq!(
        collected.details,
        [
            (Receipt(4), 151),
            (Bill(36), 149),
            (Receipt(6), 51),
            (Receipt(2), 49),
            (Receipt(1), 149),
            (Receipt(7), 50),
            (Bill(73), 149),
        ]
    );
    let (cash, online) = (collected.cash_paise, collected.online_paise);
    let detail_totals = [cash, online, cash + online];
    assert_eq!(detail_totals, [397, 351, 748]);
    println!(
        "L04 Custom Collected rows (source,paise)={:?}; cash/online/total paise={detail_totals:?}",
        collected.details
    );
    let [_, paid, received, total] = custom.tile_paise();
    let tile_rounding = [paid, received, total].map(round_rupees);
    let detail_rounding = detail_totals.map(round_rupees);
    assert_eq!(tile_rounding, [3, 5, 7]);
    assert_eq!(detail_rounding, [4, 4, 7]);
    let tile_display = [paid, received, total].map(money_r);
    let detail_display = detail_totals.map(money_r);
    let row_display: Vec<_> = collected
        .details
        .iter()
        .map(|&(_, paise)| money_r(paise))
        .collect();
    assert_eq!(tile_display, ["₹3", "₹5", "₹7"]);
    assert_eq!(detail_display, ["₹4", "₹4", "₹7"]);
    assert_eq!(row_display, ["₹2", "₹1", "₹1", "₹0", "₹1", "₹1", "₹1"]);
    println!(
        "L04 Custom Collected numeric rupees tile(bills,receipts,total)={tile_rounding:?} detail(cash,online,total)={detail_rounding:?}; display tile={tile_display:?} detail={detail_display:?} rows={row_display:?}"
    );

    let profit_paise = [
        profit.revenue_paise,
        profit.cost_paise,
        profit.profit_paise(),
    ];
    let profit_rounding = profit_paise.map(round_rupees);
    let profit_display = profit_paise.map(money_r);
    assert_eq!(profit_paise, [4047, 2500, 1547]);
    assert_eq!(profit_rounding, [40, 25, 15]);
    assert_eq!(profit_display, ["₹40", "₹25", "₹15"]);
    println!(
        "L04 Custom Profit (revenue,cost,profit): paise={profit_paise:?} numeric_rupees={profit_rounding:?} display={profit_display:?}"
    );
    assert_eq!(fixture, before);
}
