pub(super) fn json(raw: &serde_json::Value) {
    assert_eq!(raw["records"]["sales"][0]["id"], "sale-cancelled-id");
    assert_eq!(raw["records"]["sales"][1]["id"], "sale-active-id");
    assert_eq!(
        raw["records"]["sales"][0]["lines"][0]["item"],
        "Synthetic A"
    );
    assert_eq!(raw["records"]["sales"][0]["lines"][0]["quantity"], 2);
    assert_eq!(
        raw["records"]["sales"][0]["lines"][1]["item"],
        "Synthetic B"
    );
    assert_eq!(raw["records"]["sales"][0]["lines"][1]["quantity"], 1);
    assert_eq!(raw["records"]["sales"][1]["lines"][0]["quantity"], 6);
    assert_eq!(
        raw["records"]["sales"][1]["lines"][0]["packet_quantity"],
        12
    );
    assert_eq!(raw["records"]["sales"][1]["lines"][1]["quantity"], 2);
    assert_eq!(
        raw["records"]["sales"][1]["lines"][1]["kind"]["Crate"]["components"][0]["quantity"],
        3
    );
    assert_eq!(raw["records"]["receipts"][0]["id"], "receipt-standalone-id");
    assert_eq!(raw["records"]["receipts"][0]["date"], "2025-03-01");
    assert_eq!(raw["records"]["receipts"][0]["mode"], "Online");
    assert_eq!(raw["records"]["receipts"][0]["amount_minor"], 5000);
}

pub(super) fn projections(csv: &str, xml: &str) {
    for name in ["Products", "Customers", "Sales", "Stock"] {
        assert!(xml.contains(&format!("ss:Name=\"{name}\"")));
    }
    assert_eq!(xml.matches("<Worksheet ").count(), 4);
    assert!(xml.find("2025-02-21").unwrap() < xml.find("2025-01-01").unwrap());
    assert!(xml.contains(concat!(
        "<Row><Cell><Data ss:Type=\"String\">2025-02-21</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">1</Data></Cell>",
        "<Cell><Data ss:Type=\"String\">Synthetic Buyer</Data></Cell>",
        "<Cell><Data ss:Type=\"String\">Synthetic A</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">177</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">2</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">354</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">410</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">100</Data></Cell>",
        "<Cell><Data ss:Type=\"String\">CANCELLED</Data></Cell></Row>"
    )));
    assert!(xml.contains(concat!(
        "<Cell><Data ss:Type=\"String\">Synthetic B</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">56</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">1</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">56</Data></Cell>"
    )));
    assert!(xml.contains(concat!(
        "<Cell><Data ss:Type=\"Number\">56</Data></Cell>",
        "<Cell><Data ss:Type=\"String\"></Data></Cell>",
        "<Cell><Data ss:Type=\"String\"></Data></Cell>",
        "<Cell><Data ss:Type=\"String\">CANCELLED</Data></Cell>"
    )));
    for absent in [
        "sale-cancelled-id",
        "sale-active-id",
        "receipt-standalone-id",
        "hidden-component-id",
        "2025-03-01",
    ] {
        assert!(!csv.contains(absent));
        assert!(!xml.contains(absent));
    }
    let sales_sheet = xml
        .split("<Worksheet ss:Name=\"Sales\">")
        .nth(1)
        .unwrap()
        .split("</Worksheet>")
        .next()
        .unwrap();
    assert!(!sales_sheet.contains(">Mode<") && !sales_sheet.contains(">Pack<"));
    assert!(sales_sheet.contains(concat!("<Cell><Data ss:Type=\"String\">Control item</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">1</Data></Cell><Cell><Data ss:Type=\"Number\">6</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">6</Data></Cell><Cell><Data ss:Type=\"Number\">11</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">3</Data></Cell><Cell><Data ss:Type=\"String\"></Data></Cell>")));
    assert!(sales_sheet.contains(concat!("<Cell><Data ss:Type=\"String\">Crate: Control bundle</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">2.5</Data></Cell><Cell><Data ss:Type=\"Number\">2</Data></Cell>",
        "<Cell><Data ss:Type=\"Number\">5</Data></Cell><Cell><Data ss:Type=\"String\"></Data></Cell>",
        "<Cell><Data ss:Type=\"String\"></Data></Cell><Cell><Data ss:Type=\"String\"></Data></Cell>")));
}
