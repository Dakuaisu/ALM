//! Opt-in differential suite: every scenario runs the same commands through the core's
//! `LocalAuthority` and the dev Postgres authority and fails on any difference in outcome or
//! effect. Set `ALM_PG_URL` (as for `conformance_postgres`); without it every test returns early.
#![forbid(unsafe_code)]

mod conformance;
mod differential;
mod pg_authority;

use alm_core::Role;
use alm_sync::{Intent, Outcome};
use differential::Pair;
use differential::script::{
    Bill, CUSTOMER, For, LIME, MARKET, SODA, cash_day, count, crate_sale, crate_terms, mixed_sale,
    on_device, on_hand, product, purchase, receipt, receive, sale,
};

fn created(outcome: Option<Outcome>) -> u64 {
    match outcome {
        Some(Outcome::Created(id)) => id,
        other => panic!("expected a created ID, got {other:?}"),
    }
}

/// Batch 1: one customer, a part-paid bill, a separately dated online receipt, Cash Book
/// closes, cancellation (the receipt survives), restore and cancel again.
#[test]
fn batch1() {
    let Some(mut p) = Pair::new("batch1") else {
        return;
    };
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 2)],
        date: "2025-04-30",
        paid: 5_000,
        loading: 0,
    };
    p.owner("post", &sale(&core, &bill));
    p.owner("receipt", &receipt(CUSTOMER.0, "2025-05-02", 3_050, true));
    p.owner("close 30 Apr", &cash_day(None, "2025-04-30", 5_000, 0, 0));
    p.owner("close 2 May", &cash_day(None, "2025-05-02", 0, 3_100, 0));
    p.owner(
        "cancel",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.owner(
        "restore",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.owner(
        "cancel again",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.owner(
        "restore twice",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.owner(
        "restore again refused",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    let log: Vec<String> = p
        .core()
        .owner
        .unwrap()
        .book
        .activity
        .iter()
        .map(|a| format!("{} {} {}", a.device, a.kind, a.message()))
        .collect();
    assert_eq!(
        log,
        [
            "diff-device bill-save #1 Synthetic Customer ₹241",
            "diff-device payment Synthetic Customer ₹31 (Online)",
            "diff-device bill-cancel #1 Synthetic Customer ₹241",
            "diff-device bill-restore #1 Synthetic Customer",
            "diff-device bill-cancel #1 Synthetic Customer ₹241",
            "diff-device bill-restore #1 Synthetic Customer",
        ]
    );
    p.finish();
}

/// Batch 2: products, quotes, a purchase order received in parts at changed rates, close
/// remaining, crates, godown rows, stock counts and a bill spanning received lots.
#[test]
fn batch2() {
    let Some(mut p) = Pair::new("batch2") else {
        return;
    };
    p.owner("reorder default", &Intent::SetReorderDefault { level: 7 });
    p.owner("restore log", &Intent::RecordRestore);
    p.owner(
        "new product",
        &product(3, true, "Synthetic Orange", 3_000, 4_500, None),
    );
    p.owner(
        "edit lime",
        &product(
            LIME,
            false,
            "Synthetic Lime",
            7_000,
            12_550,
            Some((7_000, 12_050)),
        ),
    );
    p.owner(
        "stale edit",
        &product(
            LIME,
            false,
            "Synthetic Lime",
            7_000,
            12_650,
            Some((7_000, 12_050)),
        ),
    );
    let supplier = created(p.owner(
        "supplier",
        &Intent::AddSupplier {
            name: "Batch Two Traders".into(),
        },
    ));
    p.owner(
        "quotes",
        &Intent::SetSupplierPrices {
            product: 3,
            rates: vec![
                (0, Some(alm_core::Money::from_minor_units(2_950))),
                (supplier, None),
            ],
        },
    );
    let po = created(p.owner(
        "po",
        &purchase(
            "Synthetic Wholesale",
            "2025-05-03",
            &[
                ("Synthetic Orange", Some(3), 12, 2_900),
                ("Synthetic Soda", Some(SODA), 6, 3_700),
            ],
        ),
    ));
    let core = p.core();
    p.owner(
        "receive part",
        &receive(
            &core,
            po,
            Some("2025-05-04"),
            "2025-05-05",
            &[(5, 0), (6, 3_650)],
        ),
    );
    let core = p.core();
    p.owner(
        "receive more",
        &receive(&core, po, None, "2025-05-06", &[(4, 2_800), (0, 0)]),
    );
    let core = p.core();
    p.owner(
        "close remaining",
        &Intent::ClosePurchaseOrder {
            po,
            remaining: differential::script::remaining(&core, po),
        },
    );
    let id = created(p.owner(
        "crate",
        &crate_terms(
            None,
            "Party Crate",
            40_000,
            &[(LIME, 2, false), (3, 1, true)],
        ),
    ));
    p.owner(
        "crate edit",
        &crate_terms(Some(id), "Party Crate", 41_000, &[(LIME, 1, false)]),
    );
    p.owner("crate delete", &Intent::DeleteCrate { id });
    p.owner(
        "godown",
        &Intent::SaveGodown {
            item: "Soda".into(),
            cases: "9".into(),
            note: "back room".into(),
        },
    );
    p.owner(
        "godown again",
        &Intent::SaveGodown {
            item: "soda".into(),
            cases: "12".into(),
            note: String::new(),
        },
    );
    let core = p.core();
    let orange = on_hand(&core, 3);
    p.owner(
        "count down",
        &count(&core, 3, orange - 2, Some(3), "2025-05-07"),
    );
    let core = p.core();
    let lime = on_hand(&core, LIME);
    p.owner(
        "count up",
        &count(&core, LIME, lime + 3, None, "2025-05-07"),
    );
    let core = p.core();
    let typed = Bill {
        draft: 2,
        to: For::Typed("Walk In Buyer"),
        lines: &[(3, 6), (SODA, 2)],
        date: "2025-05-08",
        paid: 39_500,
        loading: 0,
    };
    p.owner("typed customer refused", &sale(&core, &typed));
    let bill = Bill {
        to: For::Customer(MARKET),
        ..typed
    };
    p.owner("sale across lots", &sale(&core, &bill));
    p.owner(
        "renamed customer refused",
        &sale(
            &core,
            &Bill {
                draft: 3,
                to: For::Customer((MARKET.0, "Market Renamed")),
                lines: &[(SODA, 1)],
                date: "2025-05-08",
                paid: 0,
                loading: 0,
            },
        ),
    );
    p.owner(
        "overpaid refused",
        &sale(
            &core,
            &Bill {
                draft: 3,
                to: For::Customer(MARKET),
                lines: &[(SODA, 1)],
                date: "2025-05-08",
                paid: 6_251,
                loading: 0,
            },
        ),
    );
    p.owner("delete sold product", &Intent::DeleteProduct { id: 3 });
    p.finish();
}

/// Batch 4: a posted bill with a loading charge, a receipt, a Cash Book close edited and
/// deleted, and a walk-in bill.
#[test]
fn batch4() {
    let Some(mut p) = Pair::new("batch4") else {
        return;
    };
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(MARKET),
        lines: &[(LIME, 3), (SODA, 1)],
        date: "2025-06-01",
        paid: 5_000,
        loading: 150,
    };
    p.owner("post with loading", &sale(&core, &bill));
    p.owner("receipt", &receipt(MARKET.0, "2025-06-01", 20_000, false));
    p.owner("close", &cash_day(None, "2025-06-01", 20_000, 0, 1_050));
    let id = p.core().cash_days.last().map_or(0, |d| d.id);
    p.owner(
        "close edit",
        &cash_day(Some(id), "2025-06-01", 19_950, 50, 1_050),
    );
    p.owner("close delete", &Intent::DeleteCashDay { id });
    let core = p.core();
    let walkin = Bill {
        draft: 2,
        to: For::Walkin,
        lines: &[(SODA, 4)],
        date: "2025-06-02",
        paid: 25_000,
        loading: 0,
    };
    p.owner("walk-in refused", &sale(&core, &walkin));
    p.owner(
        "unpaid",
        &sale(
            &core,
            &Bill {
                to: For::Customer(CUSTOMER),
                paid: 0,
                ..walkin
            },
        ),
    );
    p.owner(
        "cancel unpaid refused",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(2),
        },
    );
    p.owner(
        "receipt moves no stock",
        &receipt(CUSTOMER.0, "2025-06-02", 100, true),
    );
    p.owner(
        "cancel after movement",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.finish();
}

/// Milestone C: a Manager is denied every command and sees only the catalog; the Owner posts.
#[test]
fn milestone_c() {
    let Some(mut p) = Pair::new("milestone_c") else {
        return;
    };
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 1)],
        date: "2025-04-30",
        paid: 12_050,
        loading: 0,
    };
    p.run(Role::Manager, "manager post", &sale(&core, &bill));
    p.owner("owner post", &sale(&core, &bill));
    p.owner("post again", &sale(&core, &bill));
    p.run(
        Role::Manager,
        "manager cancel",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.finish();
}

/// L14: opening balances (dated, advance, cleared), receipts with half rupees, a part-paid
/// bill edited down, and a cancelled fully paid bill.
#[test]
fn l14_ledger() {
    let Some(mut p) = Pair::new("l14") else {
        return;
    };
    let opening = |customer: &str, amount: Option<i64>, date: Option<&str>| Intent::SaveOpening {
        customer: customer.into(),
        amount: amount.map(alm_core::Money::from_minor_units),
        date: date.map(differential::script::day),
    };
    p.owner(
        "opening",
        &opening(CUSTOMER.0, Some(15_050), Some("2025-04-01")),
    );
    p.owner("advance", &opening(MARKET.0, Some(-2_550), None));
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 1)],
        date: "2025-04-10",
        paid: 2_050,
        loading: 0,
    };
    p.owner("part paid", &sale(&core, &bill));
    p.owner(
        "cash receipt",
        &receipt(CUSTOMER.0, "2025-04-11", 4_550, false),
    );
    p.owner("half rupee", &receipt(MARKET.0, "2025-04-11", 50, true));
    p.owner(
        "edit down",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(1),
            lines: vec![(1, 11_050)],
        },
    );
    let core = p.core();
    let paid = Bill {
        draft: 2,
        to: For::Customer(MARKET),
        lines: &[(SODA, 3)],
        date: "2025-04-12",
        paid: 18_750,
        loading: 0,
    };
    p.owner("fully paid", &sale(&core, &paid));
    p.owner(
        "cancel paid",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(2),
        },
    );
    p.owner("clear opening", &opening(CUSTOMER.0, None, None));
    p.owner(
        "unknown customer",
        &receipt("nobody", "2025-04-12", 100, false),
    );
    p.finish();
}

/// L15: Cash Book day closes with paise, an edit, a delete and an unknown row.
#[test]
fn l15_cashbook() {
    let Some(mut p) = Pair::new("l15") else {
        return;
    };
    p.owner("close", &cash_day(None, "2025-04-30", 12_345, 50, 1_999));
    p.owner("second", &cash_day(None, "2025-04-29", 100, 0, 0));
    let id = p.core().cash_days.first().map_or(0, |d| d.id);
    p.owner("edit", &cash_day(Some(id), "2025-04-28", 1, 2, 3));
    p.owner("delete", &Intent::DeleteCashDay { id });
    p.owner("unknown", &cash_day(Some(999), "2025-04-30", 1, 0, 0));
    p.finish();
}

/// L10: repeated-product FIFO layers at different costs and dates, a bill spanning layers,
/// edits up and down, cancel and restore, a count down, and a crate repeating a product, its
/// bills edited, cancelled and restored.
#[test]
fn l10_crate_fifo() {
    // A Lime piece is half a packet under these checkout terms.
    let Some(mut p) = Pair::new_with("l10", |shop| {
        shop.set_checkout_product(
            alm_core::ProductId::new(LIME),
            alm_core::confirmation::ProductTerms {
                pack: std::num::NonZeroU32::new(2).unwrap(),
                piece_price: alm_core::Money::from_minor_units(6_100),
                case_price: alm_core::Money::from_minor_units(12_050),
            },
        )
        .unwrap();
    }) else {
        return;
    };
    let po = created(p.owner(
        "po",
        &purchase(
            "Synthetic Depot",
            "2025-02-01",
            &[("Synthetic Lime", Some(LIME), 10, 6_500)],
        ),
    ));
    let core = p.core();
    p.owner(
        "receive dated",
        &receive(&core, po, Some("2025-02-02"), "2025-02-03", &[(4, 0)]),
    );
    let core = p.core();
    p.owner(
        "receive backdated",
        &receive(&core, po, Some("2025-02-01"), "2025-02-03", &[(6, 6_800)]),
    );
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 55)],
        date: "2025-02-05",
        paid: 10_000,
        loading: 0,
    };
    p.owner("sale across layers", &sale(&core, &bill));
    p.owner(
        "edit down",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(1),
            lines: vec![(52, 12_050)],
        },
    );
    p.owner(
        "edit up",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(1),
            lines: vec![(58, 12_050)],
        },
    );
    p.owner(
        "cancel",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    p.owner(
        "restore",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    let core = p.core();
    let lime = on_hand(&core, LIME);
    p.owner(
        "count down",
        &count(&core, LIME, lime - 1, Some(1), "2025-02-06"),
    );
    p.owner(
        "repeated crate",
        &crate_terms(
            None,
            "Lime Twins",
            30_000,
            &[(LIME, 1, false), (LIME, 2, true)],
        ),
    );
    // L10 crate FIFO: a crate repeating a product takes from its lots once per component,
    // across a fresh dated layer at another cost.
    let po = created(p.owner(
        "po for crates",
        &purchase(
            "Synthetic Depot",
            "2025-02-06",
            &[("Synthetic Lime", Some(LIME), 20, 6_600)],
        ),
    ));
    let core = p.core();
    p.owner(
        "receive for crates",
        &receive(&core, po, Some("2025-02-06"), "2025-02-06", &[(20, 0)]),
    );
    let twins = [(LIME, 1, false), (LIME, 2, true)];
    let template = p.core().crates.last().map(|c| c.id).unwrap();
    p.owner(
        "crate sale stale",
        &crate_sale(7, template, 3, CUSTOMER, 0, 31_000, &twins),
    );
    p.owner(
        "crate sale paid above total",
        &crate_sale(7, template, 1, CUSTOMER, 40_000, 30_000, &twins),
    );
    p.owner(
        "crate sale typed customer",
        &crate_sale(7, template, 1, ("walk-in", "Walk In"), 0, 30_000, &twins),
    );
    p.owner(
        "crate sale shortage",
        &crate_sale(7, template, 500, CUSTOMER, 0, 30_000, &twins),
    );
    let sold = p.owner(
        "crate sale",
        &crate_sale(7, template, 3, CUSTOMER, 20_000, 30_000, &twins),
    );
    assert!(
        matches!(sold, Some(Outcome::Sale(r)) if r.quantity == 6),
        "{sold:?}"
    );
    p.owner(
        "crate sale again",
        &crate_sale(7, template, 3, CUSTOMER, 20_000, 30_000, &twins),
    );
    p.owner(
        "crate sale twice",
        &crate_sale(8, template, 2, MARKET, 0, 30_000, &twins),
    );
    // Crate bills are edited (crates and rate), cancelled and restored like any bill.
    let edit = |crates: u32, rate: i64| Intent::EditSale {
        draft: alm_core::DraftId::new(7),
        lines: vec![(crates, rate)],
    };
    assert!(p.owner("crate edit up", &edit(4, 31_000)).is_some());
    assert!(p.owner("crate edit down", &edit(2, 29_500)).is_some());
    assert!(p.owner("crate edit below paid", &edit(1, 100)).is_none());
    assert!(p.owner("crate edit shortage", &edit(500, 30_000)).is_none());
    p.owner(
        "crate edit two lines",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(7),
            lines: vec![(1, 30_000), (1, 30_000)],
        },
    );
    assert!(
        p.owner(
            "crate cancel",
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(7)
            }
        )
        .is_some()
    );
    p.owner("crate edit cancelled", &edit(3, 30_000));
    assert!(
        p.owner(
            "crate restore",
            &Intent::RestoreSale {
                draft: alm_core::DraftId::new(7)
            }
        )
        .is_some()
    );
    assert!(
        p.owner(
            "crate cancel unpaid",
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(8)
            }
        )
        .is_none()
    );
    p.owner(
        "receipt after crate",
        &receipt(CUSTOMER.0, "2025-02-08", 5_000, false),
    );
    let solo = created(p.owner(
        "case crate",
        &crate_terms(None, "Soda Case", 9_000, &[(SODA, 1, false)]),
    ));
    let sold = p.owner(
        "case crate sale",
        &crate_sale(9, solo, 2, MARKET, 9_000, 9_000, &[(SODA, 1, false)]),
    );
    assert!(matches!(sold, Some(Outcome::Sale(_))), "{sold:?}");
    let core = p.core();
    assert_eq!(core.sales_unsupported, None);
    assert_eq!(
        core.sales
            .iter()
            .filter(|s| s.items == ["Lime Twins"])
            .count(),
        2
    );
    p.finish();
}

/// L10-style mixed bills (OPEN-013): ordinary lines beside crate rows on one bill, a crate
/// repeating a product that is also an ordinary line (each component its own FIFO demand after
/// the lines, OPEN-010), across a fresh dated layer at another cost; the rejections; edits of
/// lines and crates up and down; cancel, edit while cancelled, restore; a loading charge.
#[test]
fn l10_mixed_bills() {
    // A Lime piece is half a packet under these checkout terms.
    let Some(mut p) = Pair::new_with("l10_mixed", |shop| {
        shop.set_checkout_product(
            alm_core::ProductId::new(LIME),
            alm_core::confirmation::ProductTerms {
                pack: std::num::NonZeroU32::new(2).unwrap(),
                piece_price: alm_core::Money::from_minor_units(6_100),
                case_price: alm_core::Money::from_minor_units(12_050),
            },
        )
        .unwrap();
    }) else {
        return;
    };
    p.owner(
        "twins template",
        &crate_terms(
            None,
            "Lime Twins",
            30_000,
            &[(LIME, 1, false), (LIME, 2, true)],
        ),
    );
    p.owner(
        "soda template",
        &crate_terms(None, "Soda Case", 9_000, &[(SODA, 1, false)]),
    );
    let po = created(p.owner(
        "po for mixed",
        &purchase(
            "Synthetic Depot",
            "2025-02-06",
            &[("Synthetic Lime", Some(LIME), 20, 6_600)],
        ),
    ));
    let core = p.core();
    p.owner(
        "receive for mixed",
        &receive(&core, po, Some("2025-02-06"), "2025-02-06", &[(20, 0)]),
    );
    let twins: &[(u64, u32, bool)] = &[(LIME, 1, false), (LIME, 2, true)];
    let soda: &[(u64, u32, bool)] = &[(SODA, 1, false)];
    let core = p.core();
    let bill = |draft: u64, lines: &'static [(u64, u32)], paid: i64, loading: i64| Bill {
        draft,
        to: For::Customer(CUSTOMER),
        lines,
        date: "2025-02-07",
        paid,
        loading,
    };
    let lime_twins = (Some("Lime Twins"), 30_000, 3, twins);
    p.owner(
        "mixed unpriced crate",
        &mixed_sale(&core, &bill(11, &[(LIME, 2)], 0, 0), &[(None, 0, 1, twins)]),
    );
    p.owner(
        "mixed typed customer",
        &mixed_sale(
            &core,
            &Bill {
                to: For::Typed("Walk In"),
                ..bill(11, &[(LIME, 2)], 0, 0)
            },
            &[lime_twins],
        ),
    );
    p.owner(
        "mixed paid above total",
        &mixed_sale(&core, &bill(11, &[(LIME, 2)], 9_000_000, 0), &[lime_twins]),
    );
    p.owner(
        "mixed shortage",
        &mixed_sale(
            &core,
            &bill(11, &[(LIME, 2)], 0, 0),
            &[(Some("Lime Twins"), 30_000, 500, twins)],
        ),
    );
    p.owner(
        "mixed piece not whole packs",
        &mixed_sale(
            &core,
            &bill(11, &[(LIME, 2)], 0, 0),
            &[(Some("Odd"), 1_000, 1, &[(LIME, 1, true)])],
        ),
    );
    let sold = p.owner(
        "mixed sale",
        &mixed_sale(&core, &bill(11, &[(LIME, 2)], 20_000, 0), &[lime_twins]),
    );
    assert!(
        matches!(sold, Some(Outcome::Sale(r)) if r.quantity == 8),
        "{sold:?}"
    );
    p.owner(
        "mixed sale again",
        &mixed_sale(&core, &bill(11, &[(LIME, 2)], 20_000, 0), &[lime_twins]),
    );
    let sold = p.owner(
        "two lines two crates with loading",
        &mixed_sale(
            &core,
            &bill(12, &[(SODA, 1), (LIME, 1)], 5_000, 150),
            &[
                (Some("Soda Case"), 9_000, 2, soda),
                (Some("Lime Twins"), 30_000, 1, twins),
            ],
        ),
    );
    assert!(matches!(sold, Some(Outcome::Sale(_))), "{sold:?}");
    let price = |core: &alm_core::projection::ProjectionSnapshot, id: u64| {
        core.products
            .iter()
            .find(|x| x.id == id)
            .unwrap()
            .sell
            .minor_units()
    };
    let lime = price(&p.core(), LIME);
    let edit = |line: u32, crates: u32, rate: i64| Intent::EditSale {
        draft: alm_core::DraftId::new(11),
        lines: vec![(line, lime), (crates, rate)],
    };
    assert!(p.owner("mixed edit up", &edit(3, 4, 31_000)).is_some());
    assert!(p.owner("mixed edit down", &edit(1, 2, 29_500)).is_some());
    assert!(p.owner("mixed edit below paid", &edit(1, 1, 100)).is_none());
    assert!(
        p.owner("mixed edit shortage", &edit(1, 500, 30_000))
            .is_none()
    );
    p.owner(
        "mixed edit one entry",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(11),
            lines: vec![(1, lime)],
        },
    );
    assert!(
        p.owner(
            "mixed cancel",
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(11)
            }
        )
        .is_some()
    );
    p.owner("mixed edit cancelled", &edit(2, 2, 30_000));
    assert!(
        p.owner(
            "mixed restore",
            &Intent::RestoreSale {
                draft: alm_core::DraftId::new(11)
            }
        )
        .is_some()
    );
    p.owner(
        "receipt after mixed",
        &receipt(CUSTOMER.0, "2025-02-08", 5_000, false),
    );
    let core = p.core();
    assert_eq!(core.sales_unsupported, None);
    assert!(core.sales.iter().any(|s| s.items
        == [
            "Synthetic Soda",
            "Synthetic Lime",
            "Soda Case",
            "Lime Twins"
        ]));
    p.finish();
}

/// Crates holding Piece components of products without checkout terms (`linePackets`:
/// `packetQty || 1`): a crate sale, a mixed bill with loading, edits up and down, cancel and
/// restore, and a piece demand beside a case demand of the same product.
#[test]
fn piece_crates_without_pack_terms() {
    let Some(mut p) = Pair::new("piece_crates") else {
        return;
    };
    let pieces: &[(u64, u32, bool)] = &[(LIME, 3, true), (SODA, 1, false)];
    let template = created(p.owner(
        "piece template",
        &crate_terms(None, "Piece Mix", 25_000, pieces),
    ));
    p.owner(
        "piece crate sale",
        &crate_sale(21, template, 2, CUSTOMER, 10_000, 25_000, pieces),
    );
    for (step, intent) in [
        (
            "piece crate cancel",
            Intent::CancelSale {
                draft: alm_core::DraftId::new(21),
            },
        ),
        (
            "piece crate restore",
            Intent::RestoreSale {
                draft: alm_core::DraftId::new(21),
            },
        ),
    ] {
        assert!(p.owner(step, &intent).is_some(), "{step}");
    }
    let core = p.core();
    let bill = Bill {
        draft: 22,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 1)],
        date: "2025-02-07",
        paid: 5_000,
        loading: 250,
    };
    let sold = p.owner(
        "mixed piece bill",
        &mixed_sale(&core, &bill, &[(Some("Piece Mix"), 25_000, 2, pieces)]),
    );
    assert!(
        matches!(sold, Some(Outcome::Sale(r)) if r.quantity == 1 + 2 * 4),
        "{sold:?}"
    );
    let lime = core
        .products
        .iter()
        .find(|x| x.id == LIME)
        .unwrap()
        .sell
        .minor_units();
    let edit = |line: u32, crates: u32, rate: i64| Intent::EditSale {
        draft: alm_core::DraftId::new(22),
        lines: vec![(line, lime), (crates, rate)],
    };
    assert!(p.owner("piece edit up", &edit(2, 3, 26_000)).is_some());
    assert!(p.owner("piece edit down", &edit(1, 1, 24_000)).is_some());
    for (step, intent) in [
        (
            "piece cancel",
            Intent::CancelSale {
                draft: alm_core::DraftId::new(22),
            },
        ),
        (
            "piece restore",
            Intent::RestoreSale {
                draft: alm_core::DraftId::new(22),
            },
        ),
    ] {
        assert!(p.owner(step, &intent).is_some(), "{step}");
    }
    p.finish();
}

/// Bills of crate rows only (HTML `saveBill` whose lines are all crates): two and three crate
/// rows with no item line, with and without loading, a shortage, edits up and down, a refused
/// edit below the amount paid, cancel and restore of the bill moved last.
#[test]
fn crate_only_bills() {
    let Some(mut p) = Pair::new("crate_only") else {
        return;
    };
    let case: &[(u64, u32, bool)] = &[(LIME, 2, false), (SODA, 1, false)];
    let soda: &[(u64, u32, bool)] = &[(SODA, 1, false)];
    let pieces: &[(u64, u32, bool)] = &[(LIME, 3, true)];
    for (name, price, components) in [
        ("Case Pair", 30_000, case),
        ("Soda Case", 9_000, soda),
        ("Lime Pieces", 12_000, pieces),
    ] {
        created(p.owner(name, &crate_terms(None, name, price, components)));
    }
    let core = p.core();
    let bill = |draft: u64, paid: i64, loading: i64| Bill {
        draft,
        to: For::Customer(CUSTOMER),
        lines: &[],
        date: "2025-02-07",
        paid,
        loading,
    };
    let sold = p.owner(
        "two crate rows",
        &mixed_sale(
            &core,
            &bill(31, 10_000, 0),
            &[
                (Some("Case Pair"), 30_000, 2, case),
                (Some("Soda Case"), 9_000, 1, soda),
            ],
        ),
    );
    assert!(
        matches!(sold, Some(Outcome::Sale(r)) if r.quantity == 7 && r.total.minor_units() == 69_000),
        "{sold:?}"
    );
    let sold = p.owner(
        "three crate rows with loading",
        &mixed_sale(
            &core,
            &bill(32, 0, 125),
            &[
                (Some("Lime Pieces"), 12_000, 1, pieces),
                (Some("Case Pair"), 30_000, 1, case),
                (Some("Soda Case"), 9_000, 2, soda),
            ],
        ),
    );
    assert!(
        matches!(sold, Some(Outcome::Sale(r)) if r.quantity == 8),
        "{sold:?}"
    );
    assert!(
        p.owner(
            "crate rows shortage",
            &mixed_sale(
                &core,
                &bill(33, 0, 0),
                &[
                    (Some("Case Pair"), 30_000, 1, case),
                    (Some("Soda Case"), 9_000, 9_000, soda),
                ],
            ),
        )
        .is_none()
    );
    let edit = |a: u32, b: u32, rate: i64| Intent::EditSale {
        draft: alm_core::DraftId::new(31),
        lines: vec![(a, rate), (b, 9_000)],
    };
    assert!(p.owner("crate rows edit up", &edit(3, 2, 31_000)).is_some());
    assert!(
        p.owner("crate rows edit down", &edit(1, 1, 29_000))
            .is_some()
    );
    assert!(
        p.owner("crate rows edit below paid", &edit(1, 1, 0))
            .is_none()
    );
    for (step, intent) in [
        (
            "crate rows cancel",
            Intent::CancelSale {
                draft: alm_core::DraftId::new(31),
            },
        ),
        (
            "crate rows restore",
            Intent::RestoreSale {
                draft: alm_core::DraftId::new(31),
            },
        ),
    ] {
        assert!(p.owner(step, &intent).is_some(), "{step}");
    }
    p.owner(
        "receipt after crate rows",
        &receipt(CUSTOMER.0, "2025-02-08", 4_000, false),
    );
    p.finish();
}

/// Partial PO: two lines received in parts with a backdated date and a kept rate, a stale
/// receipt, an over-receipt, close remaining and a receipt after closing.
#[test]
fn partial_po() {
    let Some(mut p) = Pair::new("partial_po") else {
        return;
    };
    let po = created(p.owner(
        "po",
        &purchase(
            "Synthetic Wholesale",
            "2025-03-01",
            &[
                ("Synthetic Lime", Some(LIME), 12, 6_900),
                ("Synthetic Soda", Some(SODA), 9, 3_700),
            ],
        ),
    ));
    let core = p.core();
    p.owner(
        "receive backdated",
        &receive(
            &core,
            po,
            Some("2025-02-27"),
            "2025-03-02",
            &[(8, 0), (5, 3_650)],
        ),
    );
    p.owner(
        "stale",
        &Intent::ReceivePurchaseOrder {
            po,
            date: None,
            today: differential::script::day("2025-03-02"),
            lines: vec![(1, alm_core::Money::from_minor_units(0)); 2],
            remaining: vec![12, 9],
        },
    );
    let core = p.core();
    p.owner(
        "over-receive",
        &receive(&core, po, None, "2025-03-03", &[(10, 0), (0, 0)]),
    );
    let core = p.core();
    p.owner(
        "close",
        &Intent::ClosePurchaseOrder {
            po,
            remaining: differential::script::remaining(&core, po),
        },
    );
    let core = p.core();
    p.owner(
        "after close",
        &receive(&core, po, None, "2025-03-04", &[(1, 0), (1, 0)]),
    );
    p.finish();
}

/// Standalone GST bills: numbers, save, cancel, delete (frees the top number) and import.
#[test]
fn gst_bills() {
    let Some(mut p) = Pair::new("gst") else {
        return;
    };
    let first = p.gst_number("allocate", 50).unwrap();
    p.owner(
        "save",
        &Intent::SaveGstBill {
            bill: conformance::gst_bill(50, first, "GST Buyer"),
        },
    );
    let second = p.gst_number("allocate second", 51).unwrap();
    p.owner(
        "save second",
        &Intent::SaveGstBill {
            bill: conformance::gst_bill(51, second, "GST Buyer Two"),
        },
    );
    p.owner("cancel", &Intent::CancelGstBill { id: 50 });
    p.owner("delete top", &Intent::DeleteGstBill { id: 51 });
    p.gst_number("reuse freed", 52);
    p.owner(
        "import",
        &Intent::ImportGstBill {
            bill: Some(conformance::gst_bill(70, 70, "Moved Buyer")),
            deleted: Some(71),
        },
    );
    p.finish();
}

/// Legacy import: both synthetic files, the same file again under a new request, a file that
/// changed one record (quarantined, no duplicate); the manifests, records and quarantine are
/// identical, and so is the reconciliation report computed from each authority's rows.
#[test]
fn legacy_import() {
    use alm_core::legacy::{LegacyBackup, MigrateClock, live_numbers, reconcile};
    let Some(mut p) = Pair::new("legacy") else {
        return;
    };
    let clock = MigrateClock {
        now_ms: 1_767_900_000_000,
        today: "2026-01-08",
    };
    let a = LegacyBackup::parse(conformance::LEGACY_A, clock).unwrap();
    let b = LegacyBackup::parse(conformance::LEGACY_B, clock).unwrap();
    let edited_text = String::from_utf8(conformance::LEGACY_A.to_vec())
        .unwrap()
        .replace("\"amount\": 91,", "\"amount\": 92,");
    let edited = LegacyBackup::parse(edited_text.as_bytes(), clock).unwrap();
    let source = |backup: &LegacyBackup| {
        [
            conformance::LEGACY_A,
            conformance::LEGACY_B,
            edited_text.as_bytes(),
        ]
        .into_iter()
        .find(|bytes| alm_core::legacy::sha256_hex(bytes) == backup.digest)
        .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
        .unwrap()
    };
    let import = |backup: &LegacyBackup| {
        let mut plan = alm_core::legacy::import::plan(backup);
        plan.source = source(backup);
        Intent::ImportLegacy(Box::new(plan))
    };
    assert_eq!(created(p.owner("import a", &import(&a))), 1);
    assert_eq!(created(p.owner("import a again", &import(&a))), 1);
    assert_eq!(created(p.owner("import edited a", &import(&edited))), 2);
    assert_eq!(created(p.owner("import b", &import(&b))), 3);
    let core = p.core().owner.unwrap().book;
    let sql = p.sql_book();
    for (backup, id) in [(&a, 1), (&edited, 2), (&b, 3)] {
        let from_core = reconcile(backup, &core.legacy, id, live_numbers(&core)).unwrap();
        let from_sql = reconcile(backup, &sql.legacy, id, live_numbers(&sql)).unwrap();
        assert_eq!(from_core, from_sql, "manifest {id}");
    }
    // Quarantine decisions: refusals first, then one kept excluded and one corrected import.
    {
        use alm_core::legacy::resolve::{Decision, QuarantineDecision, map_corrected};
        let book = p.core().owner.unwrap().book.legacy;
        let manifest = book.manifests.iter().find(|m| m.id == 1).unwrap().clone();
        let decide = |legacy: &str, decision, body, reason: &str| {
            let entry = manifest
                .entries
                .iter()
                .position(|e| e.legacy_id == legacy)
                .unwrap();
            Intent::ResolveLegacyQuarantine(Box::new(QuarantineDecision {
                manifest: 1,
                entry: entry as u64,
                kind: manifest.entries[entry].kind.clone(),
                legacy_id: legacy.into(),
                decision,
                body,
                reason: reason.into(),
            }))
        };
        let raw: serde_json::Value = serde_json::from_slice(conformance::LEGACY_A).unwrap();
        let mut pay_4 = raw["payments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "pay_4")
            .unwrap()
            .clone();
        pay_4["amount"] = serde_json::json!(10.01);
        let corrected = map_corrected("payment", &pay_4).unwrap();
        let b_7 = raw["sales"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "b_7")
            .unwrap();
        let sale = map_corrected("sale", b_7).unwrap();
        p.owner(
            "resolve no reason",
            &decide("pay_9", Decision::KeepExcluded, None, " "),
        );
        p.owner(
            "resolve not quarantined",
            &decide("pay_1", Decision::KeepExcluded, None, "x"),
        );
        p.owner(
            "resolve body with exclude",
            &decide(
                "pay_9",
                Decision::KeepExcluded,
                Some(corrected.clone()),
                "x",
            ),
        );
        p.owner(
            "resolve unresolved reference",
            &decide("b_7", Decision::ImportCorrected, Some(sale), "x"),
        );
        p.owner(
            "resolve exclude",
            &decide("pay_9", Decision::KeepExcluded, None, "deleted in the shop"),
        );
        p.owner(
            "resolve twice",
            &decide("pay_9", Decision::KeepExcluded, None, "again"),
        );
        p.owner(
            "resolve corrected",
            &decide(
                "pay_4",
                Decision::ImportCorrected,
                Some(corrected),
                "rounded to paise",
            ),
        );
        assert_eq!(p.core().owner.unwrap().book.legacy.resolutions.len(), 2);
    }
    p.finish();
}

/// Two devices of one account each post their draft 1 beside the earlier bare draft 1: three
/// bills with three invoices; cancelling, editing and restoring one device's bill touches only
/// that bill; a second post of a device's draft 1 is refused as already posted.
#[test]
fn two_devices_first_drafts() {
    let Some(mut p) = Pair::new("two_devices_first_drafts") else {
        return;
    };
    let core = p.core();
    let bill = Bill {
        draft: 1,
        to: For::Customer(CUSTOMER),
        lines: &[(LIME, 2)],
        date: "2025-04-30",
        paid: 5_000,
        loading: 0,
    };
    let (a, b) = (
        alm_sync::request_prefix("diff-shop", "dev-00000000000000a1"),
        alm_sync::request_prefix("diff-shop", "dev-00000000000000b2"),
    );
    let id = |origin: &str| alm_core::DraftId::scoped(origin, 1).unwrap();
    p.owner("bare draft 1", &sale(&core, &bill));
    p.owner("device A draft 1", &on_device(sale(&core, &bill), &a));
    p.owner(
        "device B draft 1",
        &on_device(
            sale(
                &core,
                &Bill {
                    draft: 1,
                    to: For::Customer(CUSTOMER),
                    lines: &[(SODA, 1)],
                    date: "2025-04-30",
                    paid: 5_000,
                    loading: 0,
                },
            ),
            &b,
        ),
    );
    p.owner("device B draft 1 again", &on_device(sale(&core, &bill), &b));
    p.owner("cancel device A", &Intent::CancelSale { draft: id(&a) });
    p.owner(
        "edit device B",
        &Intent::EditSale {
            draft: id(&b),
            lines: vec![(3, 4_000)],
        },
    );
    p.owner("restore device A", &Intent::RestoreSale { draft: id(&a) });
    p.owner(
        "cancel bare",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        },
    );
    let sales = p.core().sales;
    assert_eq!(sales.len(), 3);
    let invoices: std::collections::BTreeSet<_> = sales.iter().map(|s| s.invoice).collect();
    assert_eq!(invoices.len(), 3, "{sales:?}");
    p.finish();
}

/// DEC-034: a Manager posts an item bill and a mixed bill, records a payment, edits and cancels
/// a bill and saves a crate template, godown row and stock count; restoring a bill and stale or
/// Owner-only commands are refused alike. Both authorities log the Manager's account.
#[test]
fn manager_posts_edits_and_cancels() {
    let Some(mut p) = Pair::new("manager") else {
        return;
    };
    let manager = |p: &mut Pair, step: &str, intent: &Intent| p.run(Role::Manager, step, intent);
    manager(
        &mut p,
        "crate template",
        &crate_terms(None, "Soda Case", 9_000, &[(SODA, 1, false)]),
    );
    let core = p.core();
    let bill = |draft: u64, lines: &'static [(u64, u32)], paid: i64| Bill {
        draft,
        to: For::Customer(CUSTOMER),
        lines,
        date: "2025-04-30",
        paid,
        loading: 150,
    };
    manager(&mut p, "post", &sale(&core, &bill(31, &[(LIME, 2)], 5_000)));
    let soda: &[(u64, u32, bool)] = &[(SODA, 1, false)];
    manager(
        &mut p,
        "post mixed",
        &mixed_sale(
            &core,
            &bill(32, &[(LIME, 1)], 2_000),
            &[(Some("Soda Case"), 9_000, 2, soda)],
        ),
    );
    let mut stale = sale(&core, &bill(33, &[(LIME, 1)], 0));
    if let Intent::PostSale(sale) = &mut stale {
        sale.prices[0] = alm_core::Money::from_minor_units(1);
    }
    manager(&mut p, "stale price", &stale);
    manager(
        &mut p,
        "payment",
        &receipt(CUSTOMER.0, "2025-05-02", 3_050, false),
    );
    let contact = |id: Option<&str>, name: &str, mobile: &str| {
        Intent::SaveCustomer(alm_core::confirmation::owner_billing::CustomerContact {
            id: id.map(str::to_owned),
            name: name.into(),
            mobile: mobile.into(),
            addr1: " 3 Synthetic Road ".into(),
            addr2: String::new(),
        })
    };
    manager(
        &mut p,
        "add customer",
        &contact(None, " Synthetic Corner ", "90000 00009"),
    );
    manager(
        &mut p,
        "same name refused",
        &contact(None, "synthetic CORNER", ""),
    );
    manager(&mut p, "blank name refused", &contact(None, "  ", ""));
    manager(
        &mut p,
        "edit contact",
        &contact(Some("cust-1"), "Synthetic Corner", "90000 00010"),
    );
    manager(
        &mut p,
        "unknown customer",
        &contact(Some("cust-9"), "x", ""),
    );
    manager(
        &mut p,
        "pay the new customer",
        &receipt("cust-1", "2025-05-02", 500, true),
    );
    let corner = p
        .core()
        .customers
        .into_iter()
        .find(|c| c.id == "cust-1")
        .expect("the added customer");
    assert_eq!(
        (
            corner.name.as_str(),
            corner.mobile.as_str(),
            corner.addr1.as_str()
        ),
        ("Synthetic Corner", "90000 00010", "3 Synthetic Road")
    );
    assert_eq!(corner.balance, alm_core::Money::from_minor_units(-500));
    manager(
        &mut p,
        "edit",
        &Intent::EditSale {
            draft: alm_core::DraftId::new(31),
            lines: vec![(3, 12_050)],
        },
    );
    manager(
        &mut p,
        "cancel",
        &Intent::CancelSale {
            draft: alm_core::DraftId::new(31),
        },
    );
    manager(
        &mut p,
        "restore refused",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(31),
        },
    );
    p.owner(
        "owner restores",
        &Intent::RestoreSale {
            draft: alm_core::DraftId::new(31),
        },
    );
    manager(
        &mut p,
        "godown",
        &Intent::SaveGodown {
            item: "Synthetic Soda".into(),
            cases: "4".into(),
            note: String::new(),
        },
    );
    let po = created(p.owner(
        "po",
        &purchase(
            "Synthetic Depot",
            "2025-04-30",
            &[("Synthetic Soda", Some(SODA), 10, 3_610)],
        ),
    ));
    let receive_goods = |quantity: u32, remaining: u32| Intent::ReceiveGoods {
        po,
        date: Some(alm_core::BillDate::new(2025, 5, 1).unwrap()),
        today: alm_core::BillDate::new(2025, 5, 1).unwrap(),
        lines: vec![quantity],
        remaining: vec![remaining],
    };
    manager(&mut p, "receive quantities", &receive_goods(4, 10));
    manager(&mut p, "receive stale", &receive_goods(1, 10));
    manager(
        &mut p,
        "receive at a rate refused",
        &Intent::ReceivePurchaseOrder {
            po,
            date: None,
            today: alm_core::BillDate::new(2025, 5, 1).unwrap(),
            lines: vec![(1, alm_core::Money::from_minor_units(1))],
            remaining: vec![6],
        },
    );
    manager(
        &mut p,
        "close refused",
        &Intent::ClosePurchaseOrder {
            po,
            remaining: vec![6],
        },
    );
    let core = p.core();
    manager(
        &mut p,
        "count",
        &count(&core, SODA, on_hand(&core, SODA) - 1, None, "2025-05-02"),
    );
    manager(
        &mut p,
        "opening refused",
        &Intent::SaveOpening {
            customer: CUSTOMER.0.into(),
            amount: Some(alm_core::Money::from_minor_units(100)),
            date: None,
        },
    );
    let accounts: Vec<(String, String)> = p
        .core()
        .owner
        .unwrap()
        .book
        .activity
        .iter()
        .map(|a| (a.kind.clone(), a.account.clone()))
        .collect();
    for kind in [
        "bill-save",
        "payment",
        "bill-edit",
        "bill-cancel",
        "crate-save",
        "adjust",
    ] {
        assert!(
            accounts
                .iter()
                .any(|(k, account)| k == kind && account == "diff-manager"),
            "{kind}: {accounts:?}"
        );
    }
    assert!(
        accounts
            .iter()
            .any(|(k, account)| k == "bill-restore" && account == "diff-owner")
    );
    p.finish();
}
