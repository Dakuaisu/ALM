#[path = "gst_preview_replay/archive_restore.rs"]
mod archive_restore;
#[path = "gst_preview_replay/artifacts.rs"]
mod artifacts;
#[path = "gst_preview_replay/fixture.rs"]
mod fixture;
#[path = "gst_preview_replay/markup.rs"]
mod markup;
#[path = "gst_preview_replay/rounding.rs"]
mod rounding;
#[path = "gst_preview_replay/sink.rs"]
mod sink;

use sink::{Action, Event, Sink};
use std::time::Duration;

const PAGE_START: &str = "<div style=\"border:1px solid #c2c8d2;font-family:";

fn page_ids(page: &str) -> Vec<u32> {
    page.split(">ITEM-")
        .skip(1)
        .map(|row| {
            let (id, rest) = row.split_at(3);
            assert!(rest.starts_with("</td>"));
            id.parse().unwrap()
        })
        .collect()
}

#[test]
fn l06_05_saved_invoice_rows_and_summary_follow_source_page_boundaries() {
    // Given: independent literal page ranges from the frozen source cases.
    let cases: Vec<(usize, Vec<Vec<u32>>, &str, &str)> = vec![
        (12, vec![(1..=12).collect()], "108.00", "1,416.00"),
        (13, vec![(1..=13).collect(), vec![]], "117.00", "1,534.00"),
        (20, vec![(1..=20).collect(), vec![]], "180.00", "2,360.00"),
        (
            32,
            vec![(1..=20).collect(), (21..=32).collect()],
            "288.00",
            "3,776.00",
        ),
        (
            33,
            vec![(1..=20).collect(), (21..=33).collect(), vec![]],
            "297.00",
            "3,894.00",
        ),
    ];
    let artifacts = artifacts::Sandbox::new();
    for (count, expected, tax, grand) in cases {
        let invoice = fixture::reopened(count);
        let preview = markup::render(&invoice);

        // When: both independent actions generate HTML that is written and read back.
        let mut sink = Sink::default();
        sink.invoke(Action::Print, &invoice, true);
        sink.invoke(Action::SavePdf, &invoice, true);
        assert_eq!(sink.documents.len(), 2);
        assert_eq!(sink.documents[0].html, sink.documents[1].html);
        for (action, document) in [Action::Print, Action::SavePdf]
            .into_iter()
            .zip(&sink.documents)
        {
            let html = artifacts.readback(count, action, &document.html);
            assert!(!html.is_empty(), "artifact readback is empty");
            assert_eq!(html, document.html);
            assert!(html.contains(&preview));
            assert!(html.contains("<title>GST-Invoice-24-25-47</title>"));
            assert_eq!(html.matches("<script").count(), 1);
            assert!(html.contains("<script type=\"application/x-alm-inert\">"));
            let pages: Vec<_> = html.split(PAGE_START).skip(1).collect();

            // Then: the read-back HTML contains the literal rows and only a final summary.
            assert_eq!(pages.len(), expected.len(), "{count} rows");
            let mut observed = Vec::new();
            for (index, (page, ids)) in pages.iter().zip(&expected).enumerate() {
                let page_rows = page_ids(page);
                assert_eq!(page_rows, *ids, "{count} rows, page {index}");
                let final_page = index + 1 == pages.len();
                observed.push((
                    page_rows.first().copied(),
                    page_rows.last().copied(),
                    final_page,
                ));
                for marker in [
                    ">CGST</td>",
                    ">SGST</td>",
                    "Grand Total",
                    "Amount Chargeable (in words)",
                    "This is a Computer Generated Invoice",
                ] {
                    assert_eq!(
                        page.matches(marker).count(),
                        usize::from(final_page),
                        "{count} rows: {marker}"
                    );
                }
                if final_page {
                    for kind in ["CGST", "SGST"] {
                        assert!(page.contains(&format!(">{kind}</td><td style=\"padding:3px 7px;border-bottom:1px solid #eaedf1;text-align:right\">{tax}</td>")));
                    }
                    assert!(page.contains(&format!(">&#8377; {grand}</td>")));
                }
                assert_eq!(page.contains("Continued on page"), !final_page);
                assert_eq!(page.contains("page-break-after:always"), !final_page);
                assert_eq!(page.matches("Description of Goods").count(), 1);
                if !final_page {
                    assert!(page.contains(&format!("Continued on page {} &rarr;", index + 2)));
                }
                if pages.len() > 1 {
                    assert!(page.contains(&format!("(Page {} of {})", index + 1, pages.len())));
                }
            }
            assert_eq!(
                pages
                    .iter()
                    .flat_map(|page| page_ids(page))
                    .collect::<Vec<_>>(),
                (1..=u32::try_from(count).unwrap()).collect::<Vec<_>>()
            );
            println!(
                "GST {count} rows {action:?}: page first/last item ID and summary flag {observed:?}"
            );
        }
    }
}

#[test]
fn l06_04_print_and_save_pdf_each_render_before_opening_the_same_document() {
    // Given: a saved and reopened standalone invoice.
    let invoice = fixture::reopened(13);
    let mut sink = Sink::default();

    // When: each preview action is invoked with an available inert popup.
    sink.invoke(Action::Print, &invoice, true);
    sink.invoke(Action::SavePdf, &invoice, true);

    // Then: each action renders independently and writes the same titled document.
    assert_eq!(
        sink.events,
        [
            Event::Render(Action::Print),
            Event::Open(Action::Print, "", "_p", "width=840,height=920"),
            Event::Write(Action::Print),
            Event::Close(Action::Print),
            Event::Render(Action::SavePdf),
            Event::Open(Action::SavePdf, "", "_p", "width=840,height=920"),
            Event::Write(Action::SavePdf),
            Event::Close(Action::SavePdf),
        ]
    );
    assert_eq!(sink.documents.len(), 2);
    for document in &sink.documents {
        assert!(
            document
                .html
                .contains("<title>GST-Invoice-24-25-47</title>")
        );
        assert!(
            document
                .html
                .contains("Synthetic &lt;Buyer&gt; &amp; &quot;Co&quot;")
        );
        assert!(!document.html.contains("Synthetic <Buyer>"));
        assert!(document.html.contains(
            "<td style=\"padding:3px 7px;border-bottom:1px solid #eaedf1\">ITEM-001</td>"
        ));
        assert!(document.html.contains("<script type=\"application/x-alm-inert\">window.onload=function(){setTimeout(function(){window.print();},350);}</script>"));
        assert_eq!(document.print_after_load, Duration::from_millis(350));
    }
    assert_eq!(sink.documents[0].html, sink.documents[1].html);
}

#[test]
fn l06_04_blocked_popup_warns_once_per_action_after_rendering() {
    // Given: a saved invoice and an independent blocked popup for each action.
    let invoice = fixture::reopened(12);
    for action in [Action::Print, Action::SavePdf] {
        let mut sink = Sink::default();

        // When: the action attempts to open its popup.
        sink.invoke(action, &invoice, false);

        // Then: the render precedes the attempt, with one warning and no document.
        assert_eq!(
            sink.events,
            [
                Event::Render(action),
                Event::Open(action, "", "_p", "width=840,height=920"),
                Event::Warning(
                    action,
                    "Pop-up blocked — allow pop-ups for this page to print",
                    "warn"
                ),
            ]
        );
        assert!(sink.documents.is_empty());
    }
}

#[test]
fn l06_04_actions_regenerate_when_reopened_invoice_changes() {
    // Given: two reopened versions with the same title and different line counts.
    let first = fixture::reopened(12);
    let second = fixture::reopened(13);
    let mut sink = Sink::default();

    // When: Print and Save PDF receive the respective versions.
    sink.invoke(Action::Print, &first, true);
    sink.invoke(Action::SavePdf, &second, true);

    // Then: the latter document contains the new row rather than a cached body.
    assert!(!sink.documents[0].html.contains(">ITEM-013</td>"));
    assert!(sink.documents[1].html.contains(">ITEM-013</td>"));
    assert!(
        sink.documents
            .iter()
            .all(|doc| doc.html.contains("<title>GST-Invoice-24-25-47</title>"))
    );
}
