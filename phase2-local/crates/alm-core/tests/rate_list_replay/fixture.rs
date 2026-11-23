use super::model::RateListRow;

pub struct Product {
    pub row: RateListRow<'static>,
    pub restricted: &'static str,
}

pub fn fixture() -> Vec<Product> {
    vec![
        Product {
            row: RateListRow {
                id: 1,
                company: "Beta",
                item: "Zebra",
                pack: "6",
                unit: "pc",
                quantity: "2",
                mrp_mills: 150_000,
                rate_mills: 125_555,
            },
            restricted: "RESTRICTED_BUY_BETA",
        },
        Product {
            row: RateListRow {
                id: 2,
                company: "Alpha",
                item: "Zulu",
                pack: "12",
                unit: "box",
                quantity: "",
                mrp_mills: 0,
                rate_mills: 1_000,
            },
            restricted: "RESTRICTED_BUY_ZULU",
        },
        Product {
            row: RateListRow {
                id: 3,
                company: "Alpha",
                item: "A & <B>",
                pack: "1",
                unit: "L",
                quantity: "4",
                mrp_mills: 200_000,
                rate_mills: 123_455,
            },
            restricted: "RESTRICTED_BUY_ALPHA",
        },
        Product {
            row: RateListRow {
                id: 4,
                company: "",
                item: "New",
                pack: "",
                unit: "",
                quantity: "",
                mrp_mills: 0,
                rate_mills: 0,
            },
            restricted: "RESTRICTED_BUY_NEW",
        },
    ]
}

pub fn public_rows(products: &[Product]) -> Vec<RateListRow<'static>> {
    products.iter().map(|product| product.row.clone()).collect()
}
