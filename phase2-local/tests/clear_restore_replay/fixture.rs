use alm_phase2_local::BillDate;
use alm_phase2_local::Money;
use alm_phase2_local::confirmation::BalanceMode;
use alm_phase2_local::preferences::{Preference, Preferences, Theme};
use alm_phase2_local::synthetic_read_model::{Customer, Product, Receipt, Records, Sale};
use std::path::Path;

pub(super) fn preferences(directory: &Path) {
    let mut prefs = Preferences::load(directory);
    prefs.set(Preference::Theme(Theme::Dark));
    prefs.set(Preference::Privacy(true));
    prefs.set(Preference::Balance(BalanceMode::Latest));
}

pub(super) fn check_preferences(directory: &Path) {
    let prefs = Preferences::load(directory);
    assert_eq!(prefs.theme(), Theme::Dark);
    assert_eq!(prefs.balance_mode(), BalanceMode::Latest);
    assert_eq!(prefs.present_money(Money::from_minor_units(1)), None);
}

pub(super) fn empty_records() -> Records {
    Records {
        reorder_default: 5,
        products: vec![],
        customers: vec![],
        sales: vec![],
        receipts: vec![],
    }
}

pub(super) fn marked_records() -> Records {
    let date = BillDate::new(2024, 1, 2).unwrap();
    Records {
        reorder_default: 11,
        products: vec![Product {
            item: "marked product".into(),
            company: "fixture".into(),
            pack: "case".into(),
            uom: "each".into(),
            packet_quantity: 1,
            mrp_minor: 100,
            buy_minor: 50,
            sell_minor: 80,
            purchased: 8,
            sold: 2,
            on_hand: 6,
            reorder: 3,
        }],
        customers: vec![Customer {
            name: "marked customer".into(),
            mobile: "".into(),
            address: "".into(),
            balance_minor: 70,
        }],
        sales: vec![Sale {
            id: "marked sale".into(),
            date,
            number: 1,
            customer: "marked customer".into(),
            mode: "cash".into(),
            lines: vec![],
            total_minor: 80,
            paid_minor: 10,
            cancelled: false,
        }],
        receipts: vec![Receipt {
            id: "marked receipt".into(),
            date,
            mode: "cash".into(),
            customer: "marked customer".into(),
            amount_minor: 10,
        }],
    }
}

pub(super) fn imported_records() -> Records {
    let mut records = marked_records();
    records.reorder_default = 27;
    records.products[0].item = "imported product".into();
    records.products[0].on_hand = 19;
    records.customers[0].name = "imported customer".into();
    records.sales[0].id = "imported sale".into();
    records.sales[0].customer = "imported customer".into();
    records.receipts[0].id = "imported receipt".into();
    records.receipts[0].customer = "imported customer".into();
    records
}
