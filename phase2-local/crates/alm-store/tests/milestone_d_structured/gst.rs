use std::path::Path;

use alm_core::confirmation::{GstError, GstInvoice, GstJurisdiction};
use alm_core::synthetic_read_model::{
    GstArchiveError, HistoricalGst, HistoricalGstLine, Sale, SaleId,
};
use alm_core::{BillDate, DraftLine, ProductId, Role};
use alm_store::synthetic_read_model::Store;
use serde_json::json;

pub fn snapshot(sale: &Sale, draft: &[DraftLine]) -> HistoricalGst {
    let lines = sale
        .lines
        .iter()
        .zip(draft)
        .zip(super::ordinary_gst_fixture::GST_METADATA)
        .enumerate()
        .map(|(position, ((line, draft), tax))| {
            assert_eq!(draft.product_id(), tax.product);
            assert_eq!(line.item, tax.name);
            HistoricalGstLine {
                position,
                sale_line: line.clone(),
                product: tax.product,
                hsn: tax.product_hsn.unwrap().into(),
                rate: tax.product_rate.unwrap(),
                cess: tax.product_cess,
            }
        })
        .collect();
    HistoricalGst {
        sale: SaleId::new(sale.id.clone()),
        jurisdiction: GstJurisdiction::Intra,
        lines,
    }
}

pub fn document(store: &Store, path: &Path) -> GstInvoice {
    let before = store.backup(Role::Owner).unwrap();
    let disk = std::fs::read(path).unwrap();
    let mut documents = Vec::new();
    store
        .emit_ordinary_gst(
            Role::Owner,
            &SaleId::new("sale-cancelled-id".into()),
            &mut |doc| documents.push(doc),
        )
        .unwrap();
    let [doc]: [GstInvoice; 1] = documents.try_into().unwrap();
    assert_eq!(
        (doc.invoice, doc.date, doc.customer.as_str(), doc.cancelled),
        (
            1,
            BillDate::new(2025, 2, 21).unwrap(),
            "Synthetic Buyer",
            true
        )
    );
    assert_eq!(
        [
            doc.total,
            doc.taxable,
            doc.tax,
            doc.cgst,
            doc.sgst,
            doc.igst
        ]
        .map(|m| m.minor_units()),
        [41_000, 35_000, 6_000, 3_000, 3_000, 0]
    );
    assert_eq!(
        doc.groups
            .iter()
            .map(|g| (
                g.hsn.as_str(),
                g.combined_rate,
                g.taxable.minor_units(),
                g.tax.minor_units(),
                g.cgst.minor_units(),
                g.sgst.minor_units(),
                g.igst.minor_units()
            ))
            .collect::<Vec<_>>(),
        [
            ("22029920", 1_800, 30_000, 5_400, 2_700, 2_700, 0),
            ("21069099", 1_200, 5_000, 600, 300, 300, 0)
        ]
    );
    assert_eq!(
        doc.lines
            .iter()
            .map(|line| (
                line.product,
                line.name.as_str(),
                line.quantity.get(),
                line.hsn.as_str(),
                line.rate,
                line.cess,
                line.inclusive_rate.minor_units(),
                line.inclusive.minor_units(),
                line.taxable.minor_units()
            ))
            .collect::<Vec<_>>(),
        [
            (
                ProductId::new(1),
                "Synthetic A",
                2,
                "22029920",
                1_800,
                0,
                17_700,
                35_400,
                30_000
            ),
            (
                ProductId::new(2),
                "Synthetic B",
                1,
                "21069099",
                1_200,
                0,
                5_600,
                5_600,
                5_000
            ),
        ]
    );
    assert_eq!(store.backup(Role::Owner).unwrap(), before);
    assert_eq!(std::fs::read(path).unwrap(), disk);
    doc
}

pub fn json(doc: &GstInvoice) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "invoice": doc.invoice, "date": doc.date.to_string(), "customer": doc.customer,
        "cancelled": doc.cancelled, "jurisdiction": format!("{:?}", doc.jurisdiction),
        "total_minor": doc.total.minor_units(), "taxable_minor": doc.taxable.minor_units(),
        "tax_minor": doc.tax.minor_units(), "cgst_minor": doc.cgst.minor_units(),
        "sgst_minor": doc.sgst.minor_units(), "igst_minor": doc.igst.minor_units(),
        "lines": doc.lines.iter().map(|l| json!({"product": l.product, "name": l.name,
            "quantity": l.quantity.get(), "hsn": l.hsn, "rate": l.rate, "cess": l.cess,
            "inclusive_rate_minor": l.inclusive_rate.minor_units(), "taxable_rate_minor": l.taxable_rate.minor_units(),
            "inclusive_minor": l.inclusive.minor_units(), "taxable_minor": l.taxable.minor_units()})).collect::<Vec<_>>(),
        "groups": doc.groups.iter().map(|g| json!({"hsn":g.hsn,"rate":g.combined_rate,
            "taxable_minor":g.taxable.minor_units(),"tax_minor":g.tax.minor_units(),
            "cgst_minor":g.cgst.minor_units(),"sgst_minor":g.sgst.minor_units(),"igst_minor":g.igst.minor_units()})).collect::<Vec<_>>()
    })).unwrap()
}

pub fn controls(store: &Store, path: &Path, dir: &Path) {
    let before = store.backup(Role::Owner).unwrap();
    let disk = std::fs::read(path).unwrap();
    let mut calls = 0;
    assert_eq!(
        store.emit_ordinary_gst(
            Role::Manager,
            &SaleId::new("sale-cancelled-id".into()),
            &mut |_| { calls += 1 }
        ),
        Err(GstArchiveError::AccessDenied)
    );
    assert_eq!(
        store.emit_ordinary_gst(Role::Manager, &SaleId::new("unknown".into()), &mut |_| {
            calls += 1
        }),
        Err(GstArchiveError::AccessDenied)
    );
    assert_eq!(calls, 0);
    assert_eq!(store.backup(Role::Owner).unwrap(), before);
    assert_eq!(std::fs::read(path).unwrap(), disk);

    let mut missing: serde_json::Value = serde_json::from_slice(&before).unwrap();
    missing.as_object_mut().unwrap().remove("gst");
    let missing_path = dir.join("gst-missing.json");
    Store::restore(
        &missing_path,
        Role::Owner,
        &serde_json::to_vec(&missing).unwrap(),
    )
    .unwrap();
    let missing = Store::open(&missing_path, Role::Owner).unwrap();
    let missing_before = missing.backup(Role::Owner).unwrap();
    let missing_disk = std::fs::read(&missing_path).unwrap();
    assert_eq!(
        missing.emit_ordinary_gst(
            Role::Owner,
            &SaleId::new("sale-cancelled-id".into()),
            &mut |_| calls += 1
        ),
        Err(GstArchiveError::MissingTax)
    );
    assert_eq!(calls, 0);
    assert_eq!(missing.backup(Role::Owner).unwrap(), missing_before);
    assert_eq!(std::fs::read(missing_path).unwrap(), missing_disk);

    let mut archive: serde_json::Value = serde_json::from_slice(&before).unwrap();
    archive["gst"][0]["lines"][1]["rate"] = 1_201.into();
    let control_path = dir.join("gst-rounding.json");
    Store::restore(
        &control_path,
        Role::Owner,
        &serde_json::to_vec(&archive).unwrap(),
    )
    .unwrap();
    let control = Store::open(&control_path, Role::Owner).unwrap();
    let before = control.backup(Role::Owner).unwrap();
    let disk = std::fs::read(&control_path).unwrap();
    assert_eq!(
        control.emit_ordinary_gst(
            Role::Owner,
            &SaleId::new("sale-cancelled-id".into()),
            &mut |_| calls += 1
        ),
        Err(GstArchiveError::Calculation(GstError::UnsupportedRounding))
    );
    assert_eq!(calls, 0);
    assert_eq!(control.backup(Role::Owner).unwrap(), before);
    assert_eq!(std::fs::read(control_path).unwrap(), disk);

    let saved: serde_json::Value =
        serde_json::from_slice(&store.backup(Role::Owner).unwrap()).unwrap();
    let mut position = saved.clone();
    position["gst"][0]["lines"][0]["position"] = 1.into();
    let mut mismatch = saved.clone();
    mismatch["gst"][0]["lines"][0]["sale_line"]["item"] = "different".into();
    let mut amount = saved;
    amount["records"]["sales"][0]["total_minor"] = 41_100.into();
    for (name, archive) in [
        ("position", position),
        ("line", mismatch),
        ("amount", amount),
    ] {
        let invalid_path = dir.join(format!("gst-invalid-{name}.json"));
        Store::restore(
            &invalid_path,
            Role::Owner,
            &serde_json::to_vec(&archive).unwrap(),
        )
        .unwrap();
        let invalid = Store::open(&invalid_path, Role::Owner).unwrap();
        let before = invalid.backup(Role::Owner).unwrap();
        let disk = std::fs::read(&invalid_path).unwrap();
        assert_eq!(
            invalid.emit_ordinary_gst(
                Role::Owner,
                &SaleId::new("sale-cancelled-id".into()),
                &mut |_| calls += 1
            ),
            Err(GstArchiveError::InvalidArchive)
        );
        assert_eq!(calls, 0);
        assert_eq!(invalid.backup(Role::Owner).unwrap(), before);
        assert_eq!(std::fs::read(invalid_path).unwrap(), disk);
    }
}
