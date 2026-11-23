use std::fs;
use std::path::Path;

use alm_phase2_local::Role;
use alm_phase2_local::synthetic_read_model::{Error, Product, Store};

#[path = "../../crates/alm-core/tests/rate_list_replay/model.rs"]
mod model;

const PICK: &str = "A & <B>";
const BUY: i64 = 87_654_321;

pub(super) fn catalog() -> [Product; 3] {
    [
        ("Zebra", "Beta", "6", "pc", 2, 15_000, 12_556),
        ("Zulu", "Alpha", "12", "box", 0, 0, 100),
        (PICK, "Alpha", "1", "L", 4, 20_000, 12_346),
    ]
    .map(
        |(item, company, pack, uom, packet_quantity, mrp_minor, sell_minor)| Product {
            item: item.into(),
            company: company.into(),
            pack: pack.into(),
            uom: uom.into(),
            packet_quantity,
            mrp_minor,
            buy_minor: BUY,
            sell_minor,
            purchased: 0,
            sold: 0,
            on_hand: 0,
            reorder: 0,
        },
    )
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Artifacts([String; 5]);

impl Artifacts {
    pub(super) fn retain(&self, dir: &Path, prefix: &str) {
        for (suffix, content) in [
            "preview.html",
            "print.html",
            "pdf-print.html",
            "whatsapp.txt",
            "whatsapp-uri.txt",
        ]
        .into_iter()
        .zip(&self.0)
        {
            fs::write(dir.join(format!("{prefix}rate-list-{suffix}")), content).unwrap();
        }
    }
}

fn generate(store: &Store, role: Role) -> Result<Artifacts, Error> {
    let products = &store.records(role)?.products;
    let quantities: Vec<_> = products
        .iter()
        .map(|product| match product.packet_quantity {
            0 => String::new(),
            quantity => quantity.to_string(),
        })
        .collect();
    let as_mills = |minor| {
        u32::try_from(minor)
            .ok()
            .and_then(|minor| minor.checked_mul(10))
            .filter(|&mills| mills <= u32::MAX - 1_000)
            .ok_or(Error::InvalidArchive)
    };
    let rows = products
        .iter()
        .zip(&quantities)
        .enumerate()
        .map(|(index, (product, quantity))| {
            Ok(model::RateListRow {
                id: u8::try_from(index + 1).map_err(|_| Error::InvalidArchive)?,
                company: &product.company,
                item: &product.item,
                pack: &product.pack,
                unit: &product.uom,
                quantity,
                mrp_mills: as_mills(product.mrp_minor)?,
                rate_mills: as_mills(product.sell_minor)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut replay = model::Replay::new(rows);
    replay.visit();
    replay.shown_set(false);
    replay.query = PICK.into();
    replay.group_toggle("Alpha");
    let chosen = replay.chosen();
    let [selected] = chosen.as_slice() else {
        return Err(Error::InvalidArchive);
    };
    assert_eq!(model::picker_rate(selected), "₹123");
    replay.query.clear();
    replay.brands.insert("Beta");
    assert!(!replay.shown().contains(&selected.id));
    let preview = replay.preview().unwrap();
    let text = model::text(&chosen, &replay.products);
    let mut sink = model::Sinks::default();
    sink.print(&mut replay, true);
    sink.pdf(&mut replay, true);
    sink.whatsapp(&mut replay);
    assert_eq!(sink.attempts, [("", "_rl", "width=820,height=900"); 2]);
    assert_eq!(sink.hint_durations, [3800]);
    let [print, pdf] = sink.documents.try_into().unwrap();
    let [uri] = sink.chats.try_into().unwrap();
    Ok(Artifacts([preview, print, pdf, text, uri]))
}

pub(super) fn document(store: &Store, path: &Path) -> Artifacts {
    let disk_before = fs::read(path).unwrap();
    let backup_before = store.backup(Role::Owner).unwrap();
    assert!(matches!(
        generate(store, Role::Manager),
        Err(Error::AccessDenied)
    ));
    let artifacts = generate(store, Role::Owner).unwrap();
    let [preview, print, pdf, text, uri] = &artifacts.0;
    assert_eq!(print, pdf);
    assert!(print.contains(preview));
    assert!(print.contains("type=\"application/x-alm-inert\""));
    for anchor in [
        "RATE LIST — Alpha",
        "A &amp; &lt;B&gt;",
        "1 L ×4",
        "₹200",
        "₹123.46",
    ] {
        assert!(
            preview.contains(anchor),
            "missing rate list control: {anchor}"
        );
    }
    assert_eq!(
        text,
        "*Synthetic & Co — Rate List (Alpha)*\n21-Feb-2025\n\n*Alpha*\nA & <B> — ₹123.46\n\n"
    );
    assert_eq!(
        uri,
        "https://wa.me/?text=*Synthetic%20%26%20Co%20%E2%80%94%20Rate%20List%20(Alpha)*%0A21-Feb-2025%0A%0A*Alpha*%0AA%20%26%20%3CB%3E%20%E2%80%94%20%E2%82%B9123.46%0A%0A"
    );
    for content in &artifacts.0 {
        for excluded in [
            "Zebra",
            "Zulu",
            "Synthetic A",
            "87654321",
            "8,76,543.21",
            "876543.21",
        ] {
            assert!(
                !content.contains(excluded),
                "unexpected rate list value: {excluded}"
            );
        }
    }
    assert_eq!(store.backup(Role::Owner).unwrap(), backup_before);
    assert_eq!(fs::read(path).unwrap(), disk_before);
    artifacts
}
