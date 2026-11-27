#[path = "rate_list_replay/fixture.rs"]
mod fixture;
#[path = "rate_list_replay/model.rs"]
mod model;

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use fixture::{fixture, public_rows};
use model::{RateListRow, Replay, Sinks};
use std::collections::BTreeSet;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn ids(rows: &[RateListRow<'_>]) -> Vec<u8> {
    rows.iter().map(|p| p.id).collect()
}
fn selected(replay: &Replay<'_>) -> Vec<u8> {
    replay.selected.iter().copied().collect()
}

#[test]
fn l17_selection_and_session_replay() {
    // Given: catalog order differs from picker order, with a later product.
    let all = fixture();
    let mut replay = Replay::new(public_rows(&all[..3]));
    replay.visit();
    assert_eq!(replay.shown(), [3, 2, 1]);
    assert_eq!(selected(&replay), [1, 2, 3]);
    assert_eq!(ids(&replay.chosen()), [1, 2, 3]);
    // When: clear a filtered subset, toggle a filtered group, and add a new ID.
    replay.brands.insert("Alpha");
    replay.query = "zulu".into();
    replay.shown_set(false);
    assert_eq!(replay.shown(), [2]);
    assert_eq!(selected(&replay), [1, 3]);
    replay.group_toggle("Alpha");
    assert_eq!(selected(&replay), [1, 2, 3]);
    replay.group_toggle("Alpha");
    assert_eq!(selected(&replay), [1, 3]);
    replay.query.clear();
    replay.group_toggle("Alpha");
    assert_eq!(selected(&replay), [1, 2, 3]);
    replay.group_toggle("Alpha");
    assert_eq!(selected(&replay), [1]);
    replay.products.push(all[3].row.clone());
    replay.visit();
    assert_eq!(selected(&replay), [1, 4]);
    replay.query = "no match".into();
    assert!(replay.shown().is_empty());
    replay.shown_set(false);
    assert_eq!(ids(&replay.chosen()), [1, 4]);
    let mut no_match = Sinks::default();
    no_match.print(&mut replay, true);
    no_match.pdf(&mut replay, true);
    no_match.whatsapp(&mut replay);
    assert_eq!(no_match.documents.len(), 2);
    assert_eq!(no_match.chats.len(), 1);
    assert_eq!(emitted_ids(&no_match.documents[0]), [1, 4]);
    assert_eq!(emitted_ids(&no_match.documents[1]), [1, 4]);
    assert_eq!(emitted_ids(&replay.preview().unwrap()), [1, 4]);
    assert_eq!(
        no_match.chats[0],
        model::uri(
            "*Synthetic & Co — Rate List (Beta, Other)*\n21-Feb-2025\n\n*Beta*\nZebra — ₹125.56\n\n*Other*\nNew — ₹0\n\n"
        )
    );
    replay.query.clear();
    replay.shown_set(true);
    assert_eq!(selected(&replay), [1, 2, 3, 4]);
    replay.shown_set(false);
    assert_eq!(ids(&replay.chosen()), [1, 4]);
    // Then: reopening in the same session keeps state; reload initializes all IDs.
    replay.query = "zulu".into();
    replay.visit();
    assert_eq!(selected(&replay), [1, 4]);
    assert_eq!(replay.query, "zulu");
    assert_eq!(replay.shown(), [2]);
    let mut reloaded = Replay::new(replay.products.clone());
    reloaded.visit();
    assert_eq!(selected(&reloaded), [1, 2, 3, 4]);
    assert_eq!(reloaded.shown(), [4, 3, 2, 1]);
}

#[test]
fn l17_outputs_are_exact_inert_and_restricted_fields_do_not_leak() {
    // Given: a single selected Alpha item, hidden by the Beta filter.
    let products = fixture();
    let mut replay = Replay::new(public_rows(&products));
    replay.visit();
    replay.selected = BTreeSet::from([3]);
    replay.brands.insert("Beta");
    assert_eq!(replay.shown(), [1]);
    let chosen = replay.chosen();
    assert_eq!(ids(&chosen), [3]);
    let preview = replay.preview().unwrap();
    assert_eq!(model::picker_rate(&chosen[0]), "₹123");
    let expected = concat!(
        "<div style=\"background:#fff;color:#1a1a1a;padding:22px;max-width:760px;margin:0 auto\">\n    <div style=\"text-align:center;border-bottom:3px solid #2563eb;padding-bottom:10px;margin-bottom:14px\">\n      <div style=\"font-size:22px;font-weight:800;color:#1d4ed8\">Synthetic &amp; Co</div>\n      <div style=\"font-size:12px;color:#555\">Test &lt;only&gt; · Mob: </div>\n      <div style=\"font-size:13px;font-weight:700;margin-top:6px\">RATE LIST — Alpha <span style=\"font-weight:400;color:#888\">(21-Feb-2025)</span></div>\n    </div>",
        "<div style=\"font-weight:700;color:#1d4ed8;margin:10px 0 4px;font-size:14px\">Alpha</div>\n    <table style=\"width:100%;border-collapse:collapse;font-size:12.5px;margin-bottom:8px\">\n    <thead><tr style=\"background:#eef2fb\"><th style=\"text-align:left;padding:5px 8px;border:1px solid #dbe3f4\">Item</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Pack</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">MRP</th><th style=\"padding:5px 8px;border:1px solid #dbe3f4\">Our Rate</th></tr></thead><tbody>",
        "<tr><td style=\"padding:4px 8px;border:1px solid #eee\">A &amp; &lt;B&gt;</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:center\">1 L ×4</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right\">₹200</td><td style=\"padding:4px 8px;border:1px solid #eee;text-align:right;font-weight:700\">₹123.46</td></tr></tbody></table>",
        "<div style=\"font-size:11px;color:#888;margin-top:8px\">Rates are per case/packet and subject to change. * E.&O.E.</div></div>"
    );
    assert_eq!(preview, expected);
    let expected_text =
        "*Synthetic & Co — Rate List (Alpha)*\n21-Feb-2025\n\n*Alpha*\nA & <B> — ₹123.46\n\n";
    let expected_uri = "https://wa.me/?text=*Synthetic%20%26%20Co%20%E2%80%94%20Rate%20List%20(Alpha)*%0A21-Feb-2025%0A%0A*Alpha*%0AA%20%26%20%3CB%3E%20%E2%80%94%20%E2%82%B9123.46%0A%0A";
    let actual_text = model::text(&chosen, &replay.products);
    assert_eq!(actual_text, expected_text);
    assert_eq!(model::uri(expected_text), expected_uri);
    // When: invoke each output with inert sinks, never a browser or recipient.
    let mut sinks = Sinks::default();
    sinks.print(&mut replay, true);
    sinks.pdf(&mut replay, true);
    sinks.whatsapp(&mut replay);
    let document = format!(
        "<html><head><title>Rate List</title></head><body style=\"margin:0\">{expected}<script type=\"application/x-alm-inert\">window.onload=()=>window.print()</script></body></html>"
    );
    assert_eq!(sinks.documents, [document.clone(), document.clone()]);
    assert_eq!(sinks.attempts, [("", "_rl", "width=820,height=900"); 2]);
    assert_eq!(sinks.hint_durations, [3800]);
    assert_eq!(emitted_ids(&preview), [3]);
    assert_eq!(emitted_ids(&sinks.documents[0]), [3]);
    assert_eq!(emitted_ids(&sinks.documents[1]), [3]);
    assert_eq!(sinks.chats, [expected_uri]);
    assert_eq!(sinks.notices, ["In the dialog, choose 'Save as PDF'"]);
    for output in [
        &preview,
        &document,
        &sinks.chats[0],
        &expected_text.to_string(),
    ] {
        for p in &products {
            assert!(!output.contains(p.restricted));
        }
    }
    // Then: artifacts are optional and remain inside this crate's target directory.
    if std::env::var("ALM_RATE_LIST_ARTIFACTS").as_deref() == Ok("1") {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&root).unwrap();
        let dir = scratch_retry::create(&root, &format!("rate-list-replay-{nonce}-")).unwrap();
        for (name, content) in [
            ("preview.html", preview.as_str()),
            ("print.html", sinks.documents[0].as_str()),
            ("pdf-print.html", sinks.documents[1].as_str()),
            ("whatsapp.txt", actual_text.as_str()),
            ("whatsapp-uri.txt", sinks.chats[0].as_str()),
        ] {
            let path = dir.join(name);
            fs::write(&path, content).unwrap();
            let saved = fs::read_to_string(path).unwrap();
            for p in &products {
                assert!(!saved.contains(p.restricted));
            }
        }
        println!("Synthetic inert Rate List artifacts: {}", dir.display());
    }
}

#[test]
fn l17_empty_and_blocked_popup_keep_sinks_empty() {
    let mut replay = Replay::new(public_rows(&fixture()));
    replay.visit();
    replay.shown_set(false);
    assert!(replay.preview().is_none());
    let mut empty = Sinks::default();
    empty.print(&mut replay, true);
    empty.pdf(&mut replay, true);
    empty.whatsapp(&mut replay);
    assert!(empty.documents.is_empty() && empty.chats.is_empty());
    assert!(empty.attempts.is_empty());
    assert_eq!(empty.hint_durations, [3800]);
    assert_eq!(
        empty.notices,
        [
            "Tick at least one item first",
            "Tick at least one item first",
            "In the dialog, choose 'Save as PDF'",
            "Tick at least one item first"
        ]
    );
    replay.shown_set(true);
    let mut blocked = Sinks::default();
    blocked.print(&mut replay, false);
    blocked.pdf(&mut replay, false);
    assert!(blocked.documents.is_empty());
    assert_eq!(blocked.attempts, [("", "_rl", "width=820,height=900"); 2]);
    assert_eq!(blocked.hint_durations, [3800]);
    assert_eq!(
        blocked.notices,
        [
            "Pop-up blocked — allow pop-ups for this page to print",
            "Pop-up blocked — allow pop-ups for this page to print",
            "In the dialog, choose 'Save as PDF'"
        ]
    );
}

#[test]
fn l17_multi_brand_output_uses_catalog_order_within_sorted_groups() {
    let mut replay = Replay::new(public_rows(&fixture()));
    replay.visit();
    let chosen = replay.chosen();
    assert_eq!(ids(&chosen), [1, 2, 3, 4]);
    let actual = model::text(&chosen, &replay.products);
    assert_eq!(
        actual,
        "*Synthetic & Co — Rate List*\n21-Feb-2025\n\n*Alpha*\nZulu — ₹1\nA & <B> — ₹123.46\n\n*Beta*\nZebra — ₹125.56\n\n*Other*\nNew — ₹0\n\n"
    );
    let markup = model::html(&chosen, &replay.products);
    assert_eq!(emitted_ids(&markup), [2, 3, 1, 4]);
    let positions: Vec<_> = [
        ">Zulu</td>",
        ">A &amp; &lt;B&gt;</td>",
        ">Zebra</td>",
        ">New</td>",
    ]
    .iter()
    .map(|row| markup.find(row).unwrap())
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(markup.contains(">  ×</td>"));
}

fn emitted_ids(html: &str) -> Vec<u8> {
    html.split("<tr><td style=\"padding:4px 8px;border:1px solid #eee\">")
        .skip(1)
        .map(|row| match row.split("</td>").next().unwrap() {
            "Zebra" => 1,
            "Zulu" => 2,
            "A &amp; &lt;B&gt;" => 3,
            "New" => 4,
            unexpected => panic!("unexpected emitted item: {unexpected}"),
        })
        .collect()
}
