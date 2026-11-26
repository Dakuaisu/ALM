//! On every screen that shows a bill, the displayed total is the displayed rows plus the
//! displayed charges, in exact paise and in displayed rupees: New Bill, Owner review, Sales,
//! the estimate and the bill image, for item, crate and mixed bills, with and without loading,
//! for the Owner and the Manager.
use std::num::NonZeroU32;

use alm_core::confirmation::bill_image::DrawOp;
use alm_core::{DraftId, ProductId, ProvisionalDraft};

use crate::Trial;
use crate::bill_entry::{Header, HeaderIntent};
use crate::billing_tests::day;
use crate::screen_tests::Root;

const CUSTOMER: &str = "batch1-customer";
const DATE: &str = "2025-04-30";
const LIME: u64 = 1;
const SODA: u64 = 2;

/// Paise of a displayed amount (`₹1,036.00`, `₹1,036`, `₹1,036.5`, `1,036.50`).
pub(crate) fn paise(text: &str) -> i64 {
    let text = text.trim().trim_start_matches('₹').replace(',', "");
    let (whole, cents) = text.split_once('.').unwrap_or((&text, ""));
    let cents = match cents.len() {
        0 => 0,
        1 => cents.parse::<i64>().unwrap() * 10,
        _ => cents[..2].parse::<i64>().unwrap(),
    };
    whole.parse::<i64>().unwrap() * 100 + cents
}

/// Every text between `open` and the next `close`, in order.
fn between<'a>(html: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(open) {
        rest = &rest[start + open.len()..];
        let Some(end) = rest.find(close) else { break };
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

fn totals_row(html: &str, label: &str) -> Option<i64> {
    let key = format!(r#"<span class="muted">{label}</span><b>"#);
    let start = html.find(&key)? + key.len();
    let end = html[start..].find("</b>")?;
    Some(paise(&html[start..start + end]))
}

/// New Bill: rows' amounts, the loading charge and the bill amount; the items total and the
/// bill amount must add up. Returns `(rows, loading, total)` in paise.
pub(crate) fn new_bill_amounts(html: &str) -> (i64, i64, i64) {
    let items = &html[html.find(r#"class="bill-items""#).expect("items")..];
    let items = &items[..items.find(r#"id="bill-item""#).expect("add row")];
    let rows: i64 = between(items, "<b>₹", "</b>")
        .iter()
        .map(|a| paise(a))
        .sum();
    let items_total = totals_row(html, "Items total").expect("items total");
    let loading = totals_row(html, "Loading").unwrap_or(0);
    let total = totals_row(html, "Bill Amount").expect("bill amount");
    assert_eq!(rows, items_total, "New Bill rows vs items total: {html}");
    assert_eq!(items_total + loading, total, "New Bill total: {html}");
    (rows, loading, total)
}

/// Owner review: every row (lines, crates, loading) adds up to the Owner total.
fn review_total(html: &str) -> i64 {
    let body = between(html, "<tbody>", "</tbody>");
    let rows: i64 = between(body[0], "<tr>", "</tr>")
        .iter()
        .map(|row| paise(between(row, r#"<td class="num">"#, "</td>").last().unwrap()))
        .sum();
    let total = paise(between(html, "<b>Owner total ", "</b>")[0]);
    assert_eq!(rows, total, "review rows vs total: {html}");
    total
}

/// The estimate: rows add up to Items Total; Items Total plus Loading/Delivery is the bill.
fn estimate_total(html: &str) -> i64 {
    let body = between(html, "<tbody>", "</tbody>");
    let rows: i64 = between(body[0], "<tr>", "</tr>")
        .iter()
        .map(|row| paise(between(row, "<td>", "</td>")[3]))
        .sum();
    let items = paise(
        between(html, r#"Items Total</th><td>"#, "</td></tr>")[0]
            .rsplit("<td>")
            .next()
            .unwrap(),
    );
    let loading = paise(between(html, "<th>Loading/Delivery</th><td>", "</td>")[0]);
    let total = paise(between(html, "<th>Bill Amount</th><td>", "</td>")[0]);
    assert_eq!(rows, items, "estimate rows vs items total: {html}");
    assert_eq!(items + loading, total, "estimate total: {html}");
    total
}

/// The bill image: row amounts in the AMOUNT column add up to Items Total; Items Total plus
/// Loading/Delivery is the Bill Amount.
fn image_total(trial: &Trial, bill: DraftId) -> i64 {
    let (_, image) = trial.bill_image(bill).unwrap();
    let texts: Vec<(i32, i32, &str)> = image
        .ops
        .iter()
        .filter_map(|op| match op {
            DrawOp::Text { x, y, text, .. } => Some((*x, *y, text.as_str())),
            _ => None,
        })
        .collect();
    let at = |label: &str| {
        let (_, y, _) = texts.iter().find(|t| t.2 == label).expect(label);
        let value = texts
            .iter()
            .rfind(|t| t.1 == *y && t.2.starts_with('₹'))
            .expect(label);
        (*y, paise(value.2))
    };
    let (items_y, items) = at("Items Total");
    // The shared image is the full invoice (`invoiceHTML`: "Amount"); the canvas says "AMOUNT".
    let amount = |t: &&(i32, i32, &str)| t.2 == "Amount" || t.2 == "AMOUNT";
    let amount_x = texts.iter().find(amount).unwrap().0;
    let header_y = texts.iter().find(amount).unwrap().1;
    let rows: i64 = texts
        .iter()
        // Right-aligned in the column (th padding 7px, td 8px: the edges differ by a pixel).
        .filter(|t| {
            (t.0 - amount_x).abs() <= 1 && t.1 > header_y && t.1 < items_y && t.2.starts_with('₹')
        })
        .map(|t| paise(t.2))
        .sum();
    let (_, loading) = at("Loading/Delivery");
    let (_, total) = at("Bill Amount");
    assert_eq!(rows, items, "image rows vs items total");
    assert_eq!(items + loading, total, "image total");
    total
}

/// The Sales row of invoice `invoice`: its whole-rupee total (`moneyR`).
fn sales_total(html: &str, invoice: u64) -> i64 {
    let key = format!("<b>#{invoice}</b>");
    let row = &html[html.find(&key).expect("sales row")..];
    paise(between(row, r#"<td class="num">"#, "</td>")[0])
}

fn rupees_rounded(paise: i64) -> i64 {
    alm_core::Money::from_minor_units(paise).rounded_rupees() * 100
}

/// Saves a crate template of `components` (`(product, qty, unit)`) at `price`; returns its ID.
pub(crate) fn save_template(
    trial: &mut Trial,
    name: &str,
    price: &str,
    components: &[(u64, &str, &str)],
) -> u64 {
    trial.crate_form = Some(crate::views::CrateForm {
        rows: components
            .iter()
            .map(|(p, q, u)| (p.to_string(), (*q).to_owned(), (*u).to_owned()))
            .collect(),
        ..Default::default()
    });
    let mut terms = vec![
        ("name".to_owned(), name.to_owned()),
        ("price".to_owned(), price.to_owned()),
        ("intent".to_owned(), "save".to_owned()),
    ];
    for (i, (p, q, u)) in components.iter().enumerate() {
        terms.push((format!("product_{i}"), p.to_string()));
        terms.push((format!("qty_{i}"), (*q).to_owned()));
        terms.push((format!("unit_{i}"), (*u).to_owned()));
    }
    trial.crate_submit(terms.into_iter().collect()).unwrap();
    let crates = trial.projected().unwrap().snapshot.crates;
    crates.iter().find(|c| c.name == name).unwrap().id
}

/// A bill to enter on New Bill: item lines, crate rows (template, crates) and a loading rate.
pub(crate) struct Case<'a> {
    pub lines: &'a [(u64, u32)],
    pub crates: &'a [(u64, u32)],
    pub loading: &'a str,
}

/// Enters `case` on New Bill and checks its displayed amounts; returns New Bill's total.
pub(crate) fn enter(trial: &mut Trial, case: &Case<'_>) -> i64 {
    trial.current = None;
    trial.render("bill").unwrap();
    let header = |intent| Header::new(CUSTOMER, DATE, Some(case.loading), intent);
    trial.bill_header(header(HeaderIntent::Select)).unwrap();
    for (product, qty) in case.lines {
        trial
            .bill_add(ProductId::new(*product), NonZeroU32::new(*qty).unwrap())
            .unwrap();
    }
    for (template, crates) in case.crates {
        trial
            .bill_crate_add(*template, NonZeroU32::new(*crates).unwrap())
            .unwrap();
    }
    let (_, _, unsaved) = new_bill_amounts(&trial.render("bill").unwrap());
    trial.bill_header(header(HeaderIntent::Save)).unwrap();
    let (_, _, saved) = new_bill_amounts(&trial.render("bill").unwrap());
    assert_eq!(unsaved, saved);
    saved
}

/// Posts the open saved bill (Owner review, or the crate post for one crate row alone) and
/// checks Owner review, Sales, the estimate and the bill image; returns the posted total.
pub(crate) fn post_and_check(trial: &mut Trial) -> i64 {
    let draft = trial.current.as_ref().unwrap().0.clone();
    let single_crate = draft.lines().is_empty() && draft.crate_rows().len() == 1;
    let reviewed = if single_crate {
        trial.bill_crate_post("0").unwrap();
        None
    } else {
        trial
            .sale_prepare(&crate::billing::Terms {
                date: DATE.into(),
                paid: "0".into(),
            })
            .unwrap();
        let total = review_total(&trial.render("review").unwrap());
        let serial = trial
            .posting
            .pending(alm_core::Role::Owner)
            .unwrap()
            .serial();
        trial.sale_confirm(serial).unwrap();
        Some(total)
    };
    let s = trial.projected().unwrap().snapshot;
    let sale = s
        .sales
        .iter()
        .max_by_key(|sale| sale.invoice)
        .unwrap()
        .clone();
    let posted = sale.total.minor_units();
    if let Some(reviewed) = reviewed {
        assert_eq!(reviewed, posted, "review total vs posted");
    }
    assert_eq!(
        estimate_total(&trial.estimate_html(sale.draft_id).unwrap()),
        posted
    );
    assert_eq!(image_total(trial, sale.draft_id), posted);
    assert_eq!(
        sales_total(&trial.render("sales").unwrap(), sale.invoice),
        rupees_rounded(posted)
    );
    posted
}

fn owner_trial(label: &str) -> (Root, Trial, u64, u64, u64) {
    let root = Root::new(label);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let party = save_template(
        &mut trial,
        "Party Crate",
        "400",
        &[(LIME, "2", "Case"), (SODA, "3", "Case")],
    );
    let soda = save_template(&mut trial, "Soda Case", "90", &[(SODA, "1", "Case")]);
    let pieces = save_template(
        &mut trial,
        "Piece Mix",
        "250",
        &[(LIME, "3", "Piece"), (SODA, "1", "Case")],
    );
    (root, trial, party, soda, pieces)
}

#[test]
fn owner_totals_add_up_on_every_screen_for_item_crate_and_mixed_bills() {
    let (_root, mut trial, party, soda, pieces) = owner_trial("total-invariant-owner");
    let cases = [
        Case {
            lines: &[(LIME, 2), (SODA, 1)],
            crates: &[],
            loading: "",
        },
        Case {
            lines: &[(LIME, 3)],
            crates: &[],
            loading: "2",
        },
        Case {
            lines: &[],
            crates: &[(party, 1)],
            loading: "",
        },
        Case {
            lines: &[],
            crates: &[(soda, 3)],
            loading: "",
        },
        Case {
            lines: &[(LIME, 2)],
            crates: &[(party, 2)],
            loading: "",
        },
        Case {
            lines: &[(SODA, 1), (LIME, 1)],
            crates: &[(soda, 1), (party, 1)],
            loading: "2.25",
        },
        Case {
            lines: &[],
            crates: &[(pieces, 2)],
            loading: "",
        },
        Case {
            lines: &[(LIME, 1)],
            crates: &[(pieces, 2)],
            loading: "",
        },
        Case {
            lines: &[(SODA, 2)],
            crates: &[(pieces, 1), (party, 1)],
            loading: "1.75",
        },
        // Several crate rows and no item line, through Owner review (HTML `saveBill`).
        Case {
            lines: &[],
            crates: &[(soda, 1), (pieces, 1)],
            loading: "",
        },
        Case {
            lines: &[],
            crates: &[(pieces, 1), (soda, 1)],
            loading: "1.50",
        },
    ];
    for case in &cases {
        let shown = enter(&mut trial, case);
        let posted = post_and_check(&mut trial);
        if case.lines.is_empty() {
            assert_eq!(shown, posted, "crate rows only: New Bill prices as posted");
        }
    }
    assert_eq!(trial.projected().unwrap().snapshot.sales.len(), cases.len());
}

#[test]
fn manager_new_bill_totals_add_up_and_no_owner_screen_shows_amounts() {
    let (_root, mut trial, party, _, _) = owner_trial("total-invariant-manager");
    let mut loaded = ProvisionalDraft::new(DraftId::new(0), Vec::new());
    let id = trial
        .posted
        .crate_template_id(alm_core::Role::Owner, party)
        .unwrap();
    trial
        .posted
        .load_crate_template(alm_core::Role::Owner, id, NonZeroU32::MIN, &mut loaded)
        .unwrap();
    let row = loaded.crate_rows()[0].clone();
    trial.set_owner(false).unwrap();
    for (lines, with_crate, loading) in [
        (&[(LIME, 2), (SODA, 1)][..], false, ""),
        (&[(LIME, 1)][..], false, "3"),
        (&[(SODA, 2)][..], true, ""),
        (&[(LIME, 1)][..], true, "1.25"),
    ] {
        trial.current = None;
        trial.render("bill").unwrap();
        trial
            .bill_header(Header::new(
                CUSTOMER,
                DATE,
                Some(loading),
                HeaderIntent::Select,
            ))
            .unwrap();
        for (product, qty) in lines {
            trial
                .bill_add(ProductId::new(*product), NonZeroU32::new(*qty).unwrap())
                .unwrap();
        }
        if with_crate {
            let row = row.clone();
            trial
                .edit_bill(|draft| {
                    draft.add_crate_row(row);
                    Ok(true)
                })
                .unwrap();
        }
        let html = trial.render("bill").unwrap();
        let (_, loading_paise, _) = new_bill_amounts(&html);
        assert_eq!(loading_paise != 0, !loading.is_empty(), "{html}");
        trial
            .bill_header(Header::new(
                CUSTOMER,
                DATE,
                Some(loading),
                HeaderIntent::Save,
            ))
            .unwrap();
        new_bill_amounts(&trial.render("bill").unwrap());
        assert!(
            !trial.render("review").unwrap().contains('₹'),
            "no amounts in review"
        );
        assert!(trial.estimate_html(DraftId::new(1)).is_err());
        assert!(trial.bill_image(DraftId::new(1)).is_err());
    }
}

/// A crate holding a Piece-unit item of a product without piece terms is priced on New Bill:
/// each piece is one packet (`packetQty || 1`), for loading and stock alike.
#[test]
fn a_piece_unit_crate_is_priced_on_new_bill_and_posts() {
    let (_root, mut trial, _, _, pieces) = owner_trial("total-invariant-pieces");
    let before = trial.projected().unwrap().snapshot;
    let on_hand = |s: &alm_core::projection::ProjectionSnapshot, p: u64| {
        s.stock.iter().find(|r| r.product == p).unwrap().on_hand
    };
    let shown = enter(
        &mut trial,
        &Case {
            lines: &[(LIME, 1)],
            crates: &[(pieces, 2)],
            loading: "2",
        },
    );
    let html = trial.render("bill").unwrap();
    assert!(!html.contains("unavailable"), "{html}");
    assert!(html.contains("<b>₹500.00</b>"), "2 crates at ₹250: {html}");
    // 1 Lime + 2 crates × (3 Lime pieces + 1 Soda case) = 9 packets at ₹2.
    assert_eq!(new_bill_amounts(&html).1, 1_800);
    assert_eq!(shown, 11_800 + 50_000 + 1_800);
    post_and_check(&mut trial);
    let after = trial.projected().unwrap().snapshot;
    assert_eq!(on_hand(&before, LIME) - on_hand(&after, LIME), 7);
    assert_eq!(on_hand(&before, SODA) - on_hand(&after, SODA), 2);
}

#[test]
fn displayed_amounts_parse_to_paise() {
    assert_eq!(paise("₹1,036.00"), 103_600);
    assert_eq!(paise("₹1,036"), 103_600);
    assert_eq!(paise("1,03,600.5"), 10_360_050);
    assert_eq!(paise("₹0.05"), 5);
}

/// Restore regenerates every bill image from the archive alone, in either balance mode, as the
/// HTML regenerates `invoiceHTML` from the restored `DB`: an item bill, then (after a receipt)
/// a crate and item bill with a previous balance, both byte for byte as the live images.
#[test]
fn restored_archives_regenerate_bill_images_in_both_balance_modes() {
    use alm_core::confirmation::BalanceMode;
    use alm_core::preferences::Preference;
    let (root, mut trial, party, soda, _) = owner_trial("archive-modes-a");
    enter(
        &mut trial,
        &Case {
            lines: &[(LIME, 2)],
            crates: &[],
            loading: "",
        },
    );
    post_and_check(&mut trial);
    trial
        .receipt_save(crate::receipts::Terms {
            customer: CUSTOMER.into(),
            date: DATE.into(),
            amount: "40".into(),
            mode: crate::receipts::Mode::Cash,
        })
        .unwrap();
    enter(
        &mut trial,
        &Case {
            lines: &[(SODA, 1)],
            crates: &[(party, 1), (soda, 2)],
            loading: "1.50",
        },
    );
    post_and_check(&mut trial);
    let sales = trial.projected().unwrap().snapshot.sales;
    let mut live = Vec::new();
    for mode in [BalanceMode::AsOnBillDate, BalanceMode::Latest] {
        trial
            .settings_preference(Preference::Balance(mode))
            .unwrap();
        for sale in &sales {
            live.push((
                mode,
                format!("sale-{}", sale.draft_id.value()),
                trial.bill_image_png(sale.draft_id).unwrap().1,
            ));
        }
    }
    let backup = trial.backup_create().unwrap();
    let bytes = std::fs::read(root.0.join("backups").join(&backup)).unwrap();
    drop(trial);
    let device_b = Root::new("archive-modes-b");
    std::fs::create_dir_all(device_b.0.join("backups")).unwrap();
    std::fs::write(device_b.0.join("backups").join(&backup), &bytes).unwrap();
    let mut other = Trial::open(&device_b.0, Some(day())).unwrap();
    other.backup_restore(&backup).unwrap();
    let restored = device_b.0.join("restored").join(format!(
        "{}-restored-1.json",
        backup.trim_end_matches(".json")
    ));
    let store =
        alm_store::synthetic_read_model::Store::open(&restored, alm_core::Role::Owner).unwrap();
    for (mode, sale, png) in &live {
        let again =
            crate::outputs::bill_png(&store, alm_core::Role::Owner, sale, day(), *mode).unwrap();
        assert!(
            again == *png,
            "{sale} in {mode:?} regenerated byte for byte"
        );
    }
    // The second bill's "as on bill date" image carries the previous balance (₹241 − ₹40).
    let records = store.records(alm_core::Role::Owner).unwrap();
    let snapshot = records.sales[1].as_on_bill_date.as_ref().unwrap();
    assert_eq!(snapshot.previous_minor, 24_100 - 4_000);
    assert_eq!(
        snapshot.previous_date.map(|d| d.to_string()).as_deref(),
        Some(DATE)
    );
    assert!(matches!(
        records.sales[1].lines[1].kind,
        alm_core::synthetic_read_model::LineKind::Crate { .. }
    ));
}
