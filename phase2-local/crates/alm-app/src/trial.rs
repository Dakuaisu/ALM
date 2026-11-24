//! The trial's app state: one synthetic account's drafts, fixture, stores and screen state.
use std::path::Path;
use std::sync::Mutex;

use alm_core::confirmation::demo_fixture;
use alm_core::confirmation::manual_cashbook::ManualCashBookDraft;
use alm_core::confirmation::owner_billing::{BillingDirectory, OwnerPosting};
use alm_core::confirmation::{GstBillDraft, GstBillId, GstBillLineInput, GstUnit};
use alm_core::{BillDate, DraftId, LocalSales, Money, ProvisionalDraft, Role};
use alm_store::{GstBillDatabase, ManagerDraftStore, Outbox, ProjectionStore, SqliteSales};
use alm_sync::{LocalAuthority, SaleIntent};

use crate::{debug, gst_print, text, views};

/// The synthetic account a fresh start uses; `switch_account` selects another.
pub(crate) const ACCOUNT: &str = "trial";
/// The synthetic accounts the development switcher offers.
pub(crate) const ACCOUNTS: &[&str] = &[ACCOUNT, "trial-b"];
/// The development authority simulation's journal, shared by every account on this device
/// (it stands in for the hosted server, not for any account's device tables).
const AUTHORITY_JOURNAL: &str = "simulated-authority/journal.txt";
pub(crate) const SELLER: gst_print::Seller<'static> = gst_print::Seller {
    name: "ALM Enterprises (synthetic)",
    address: "Synthetic Market Road, Lucknow",
    fssai: "",
    gstin: "09ABCDE1234F1Z5",
};
pub(crate) const TRIAL_BILL: GstBillId = GstBillId(1);
const GST_LINES: [GstBillLineInput<'static>; 2] = [
    GstBillLineInput {
        item: "Synthetic Cola 600 ml",
        hsn: "22029090",
        quantity: 2,
        inclusive_rate: Money::from_minor_units(14_000),
        gst_basis_points: 4_000,
        packet_qty: 24,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Synthetic Water 1 L",
        unit: GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    },
];

pub(crate) struct Trial {
    pub(crate) drafts: ManagerDraftStore,
    pub(crate) directory: BillingDirectory,
    /// The device's local date, sent by the page at start; tests pass a fixed date.
    pub(crate) today: Option<BillDate>,
    /// DEVELOPMENT SIMULATION of the sales authority (journaled); screens not yet projected
    /// read its fixture directly.
    pub(crate) authority: LocalAuthority,
    pub(crate) account: String,
    /// Confirmation intents, written before sending (this account's device database).
    pub(crate) outbox: Outbox,
    /// Read-only posted data for this account and role, received from the authority.
    pub(crate) projection: ProjectionStore,
    /// The screens' view of the stored projection.
    pub(crate) posted: crate::device::Posted,
    /// The sale as reviewed by `sale_prepare`, sent on confirm.
    pub(crate) reviewed: Option<SaleIntent>,
    pub(crate) current: Option<(ProvisionalDraft, bool)>,
    pub(crate) owner: bool,
    /// Development only: a REST authority the Owner pointed this device at in Settings; the
    /// local simulation otherwise.
    pub(crate) rest: Option<alm_sync::RestAuthority>,
    /// The authority revoked this account: every command is blocked.
    pub(crate) revoked: bool,
    pub(crate) message: String,
    pub(crate) posting: OwnerPosting,
    pub(crate) sales_filter: views::Filter,
    pub(crate) customers: views::CustomersState,
    pub(crate) cash_form: Option<(ManualCashBookDraft, Option<u64>)>,
    pub(crate) dash_period: alm_core::confirmation::DashboardPeriod,
    pub(crate) stock: views::StockState,
    pub(crate) godown_query: String,
    pub(crate) gst_db: GstBillDatabase,
    pub(crate) gst_form: Option<alm_core::confirmation::GstBillFields>,
    /// The bill shown on the invoice view.
    pub(crate) gst_view: GstBillId,
    pub(crate) products: views::ProductsState,
    pub(crate) order: views::OrderState,
    pub(crate) receiving: Option<alm_core::confirmation::PurchaseOrderId>,
    pub(crate) crate_form: Option<views::CrateForm>,
    pub(crate) rates: alm_core::confirmation::rate_list::RateListSelection,
    pub(crate) sale_edit: Option<DraftId>,
    /// Generated files (bill images); on Android the cache dir, shareable via FileProvider.
    pub(crate) outputs: std::path::PathBuf,
    /// Device-local presentation preferences (one file per key).
    pub(crate) prefs: alm_store::preferences::Preferences,
    /// The shop archive holding shared settings (business profile, default reorder level).
    pub(crate) shop: alm_store::synthetic_read_model::Store,
    pub(crate) device_root: std::path::PathBuf,
    /// This device's identity in the authority's activity log.
    pub(crate) device_id: String,
    /// The last restored backup, opened read-only at a fresh path.
    pub(crate) restored: Option<(String, alm_store::synthetic_read_model::Store)>,
    /// The bill or PO number shown on the estimate / PO print views.
    pub(crate) print_view: Option<u64>,
}

pub(crate) type AppState = Mutex<Trial>;

pub(crate) const REVOKED: &str = "Access to this account was revoked by the authority.";

impl Trial {
    pub(crate) fn open(device_root: &Path, today: Option<BillDate>) -> Result<Self, String> {
        let drafts = LocalSales::open_manager_account(device_root, ACCOUNT).map_err(text)?;
        let gst = GstBillDatabase::open_scoped(device_root, ACCOUNT).map_err(text)?;
        let outbox = Outbox::open_scoped(device_root, ACCOUNT).map_err(text)?;
        let projection =
            ProjectionStore::open_scoped(device_root, ACCOUNT, Role::Owner).map_err(text)?;
        let mut authority = demo_fixture::fixture().map_err(debug)?;
        demo_fixture::seed_purchasing(&mut authority).map_err(debug)?;
        // The simulated authority's sample GST bill (seed data, like the demo customers).
        let draft = GstBillDraft {
            customer: "Synthetic GST Buyer",
            cust_addr: "12 Synthetic Lane, Kanpur",
            transporter: "Synthetic Roadways",
            lines: &GST_LINES,
            ..authority
                .new_gst_bill_draft(
                    Role::Owner,
                    TRIAL_BILL,
                    BillDate::new(2025, 4, 2).map_err(debug)?,
                )
                .map_err(debug)?
        };
        authority
            .save_gst_bill(Role::Owner, &draft, &mut |_| {})
            .map_err(debug)?;
        let directory = demo_fixture::directory();
        let prefs_dir = device_root.join("preferences");
        std::fs::create_dir_all(&prefs_dir).map_err(text)?;
        let shop_path = device_root.join("shop.json");
        let shop = if shop_path.exists() {
            alm_store::synthetic_read_model::Store::open(&shop_path, Role::Owner).map_err(debug)?
        } else {
            let records = authority
                .archive_records(Role::Owner, &directory)
                .map_err(debug)?;
            alm_store::synthetic_read_model::Store::create(&shop_path, Role::Owner, records)
                .map_err(debug)?
        };
        shop.apply_product_settings(Role::Owner, &mut authority)
            .map_err(debug)?;
        let journal = device_root.join(AUTHORITY_JOURNAL);
        if let Some(parent) = journal.parent() {
            std::fs::create_dir_all(parent).map_err(text)?;
        }
        let authority =
            LocalAuthority::open(&journal, authority, directory.clone()).map_err(debug)?;
        let mut trial = Self {
            drafts,
            directory,
            today,
            authority,
            account: ACCOUNT.to_owned(),
            outbox,
            projection,
            posted: crate::device::Posted::empty()?,
            reviewed: None,
            current: None,
            owner: true,
            revoked: false,
            rest: crate::dev_authority::load(device_root),
            message: String::new(),
            posting: Default::default(),
            sales_filter: views::Filter::default(),
            customers: views::CustomersState::default(),
            cash_form: None,
            dash_period: alm_core::confirmation::DashboardPeriod::Month,
            stock: views::StockState::default(),
            godown_query: String::new(),
            gst_db: gst,
            gst_form: None,
            gst_view: TRIAL_BILL,
            products: views::ProductsState::default(),
            order: views::OrderState::default(),
            receiving: None,
            crate_form: None,
            rates: Default::default(),
            sale_edit: None,
            outputs: device_root.join("outputs"),
            prefs: alm_store::preferences::Preferences::load(&prefs_dir),
            shop,
            device_root: device_root.to_path_buf(),
            device_id: crate::dev_authority::device_id(device_root)?,
            restored: None,
            print_view: None,
        };
        trial.reconnect()?;
        trial.migrate_gst_tables()?;
        Ok(trial)
    }

    pub(crate) fn saved_ids(&self) -> Result<Vec<u64>, String> {
        let mut ids = Vec::new();
        let mut id = 1;
        while self
            .drafts
            .reopen_draft(DraftId::new(id))
            .map_err(text)?
            .is_some()
        {
            ids.push(id);
            id += 1;
        }
        Ok(ids)
    }

    pub(crate) fn today(&self) -> Result<BillDate, String> {
        self.today
            .ok_or_else(|| "The device date has not been received.".to_owned())
    }

    pub(crate) const fn role(&self) -> Role {
        if self.owner {
            Role::Owner
        } else {
            Role::Manager
        }
    }

    /// Switches the reviewing role and opens that role's projection.
    pub(crate) fn set_owner(&mut self, owner: bool) -> Result<(), String> {
        self.owner = owner;
        self.refresh()
    }

    pub(crate) fn require_owner(&self) -> Result<Role, String> {
        if self.revoked {
            return Err(REVOKED.into());
        }
        match self.role() {
            Role::Owner => Ok(Role::Owner),
            Role::Manager => Err("Access denied: Owner only.".into()),
        }
    }
}
