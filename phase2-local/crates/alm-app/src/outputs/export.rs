//! Structured exports and the estimate / PO print views.
use crate::ipc::State;

use super::platform::{ShareFile, ShareOutput, handoff};
use crate::{AppState, Trial, refusal};

/// The structured exports (`exportSalesCSV`, `exportExcel`) over the live archive records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Export {
    SalesCsv,
    Workbook,
}

impl Trial {
    pub(crate) fn export(&self, kind: Export) -> Result<(String, String), String> {
        let role = self.require_owner()?;
        let records = self
            .posted
            .archive_records(role, &self.directory)
            .map_err(refusal)?;
        let model = alm_core::synthetic_read_model::ReadModel::create(role, records, |_| Ok(()))
            .map_err(refusal)?;
        let today = self.today()?;
        Ok(match kind {
            Export::SalesCsv => (
                format!("ALM-sales-{today}.csv"),
                model.sales_csv(role).map_err(refusal)?,
            ),
            Export::Workbook => (
                format!("ALM-{today}.xls"),
                model.workbook(role).map_err(refusal)?,
            ),
        })
    }
}

/// Opens the estimate (`tab` "estimate", bill draft ID) or PO print (`tab` "poprint", PO number).
#[tauri::command(async)]
pub fn print_open(state: State<'_, AppState>, id: String, q: String) -> Result<String, String> {
    let valid = match q.as_str() {
        "estimate" => alm_core::DraftId::parse(&id).is_ok(),
        "poprint" => !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()),
        _ => return Err("Unknown print view.".into()),
    };
    if !valid {
        return Err("Unknown print view.".into());
    }
    crate::with(&state, &q, |trial| {
        trial.require(if q == "poprint" {
            alm_core::permissions::Capability::PrintPurchaseOrder
        } else {
            alm_core::permissions::Capability::PrintEstimate
        })?;
        trial.print_view = Some(id);
        Ok(())
    })
}

/// Generates an export for sharing, or saves and reveals it on desktop.
#[tauri::command(async)]
pub fn export_file(
    state: State<'_, AppState>,
    q: String,
    web_share: Option<bool>,
) -> Result<ShareOutput, String> {
    let (kind, mime) = match q.as_str() {
        "csv" => (Export::SalesCsv, "text/csv"),
        "xls" => (Export::Workbook, "application/vnd.ms-excel"),
        _ => return Err("Unknown export.".into()),
    };
    let trial = state.lock()?;
    let (name, text) = trial.export(kind)?;
    handoff(
        &trial,
        ShareFile {
            name,
            mime,
            bytes: text.into_bytes(),
        },
        web_share,
    )
}
