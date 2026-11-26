//! One-way move of the device GST tables into the authority path. The device tables stay as
//! the migration source and are never written again; each bill and deletion marker is sent
//! once under a request ID derived from the account and bill, so reopening repeats nothing.
use alm_core::Role;
use alm_core::confirmation::GstBillStore as _;
use alm_store::OutboxStatus;
use alm_sync::Intent;

use crate::{Trial, refusal, text};

impl Trial {
    /// Sends `intent` under a fixed request ID unless that request already has an answer.
    fn send_once(&mut self, request: &str, intent: &Intent) -> Result<(), String> {
        if let Some(entry) = self.outbox.get(request).map_err(text)?
            && matches!(
                entry.status,
                OutboxStatus::Confirmed | OutboxStatus::Rejected
            )
        {
            return Ok(());
        }
        self.outbox
            .record(request, &intent.canonical())
            .map_err(refusal)?;
        self.deliver(Role::Owner, request, intent).map(drop)
    }

    /// Moves this account's device GST bills and deletion markers into the authority. A bill
    /// the authority refuses (another bill under its ID or number) stays in the device table
    /// and is reported; nothing is dropped.
    pub(crate) fn migrate_gst_tables(&mut self) -> Result<(), String> {
        if self.role() != Role::Owner {
            return Ok(());
        }
        let ledger = self.gst_db.load_gst_bills().map_err(refusal)?;
        let mut kept = Vec::new();
        for bill in &ledger.bills {
            let request = format!("gstimport:{}:{}", self.account, bill.id.0);
            let intent = Intent::ImportGstBill {
                bill: Some(bill.to_record_text().map_err(refusal)?),
                deleted: None,
            };
            if self.send_once(&request, &intent).is_err() {
                kept.push(bill.number.0);
            }
        }
        for id in &ledger.deleted {
            let request = format!("gstimport-del:{}:{}", self.account, id.0);
            let intent = Intent::ImportGstBill {
                bill: None,
                deleted: Some(id.0),
            };
            if self.send_once(&request, &intent).is_err() {
                kept.push(id.0);
            }
        }
        if !kept.is_empty() {
            self.message = format!(
                "{} GST record(s) on this device were not moved to the authority and stay in the device table.",
                kept.len()
            );
        }
        Ok(())
    }
}
