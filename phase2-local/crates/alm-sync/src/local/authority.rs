//! The simulation as a `SalesAuthority`: the Owner gate, the journal and the lost-response hook.
use alm_core::Role;
use alm_core::projection::ProjectionBatch;

use super::{LocalAuthority, rejected};
use crate::{AuthorityError, Intent, Outcome, RequestId, SalesAuthority};

impl SalesAuthority for LocalAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        let known = self.requests.contains_key(request.as_str());
        let stamp = (self.clock)();
        let outcome = self.commit(request, intent, stamp)?;
        if !known {
            self.append(&format!(
                "R\t{}\t{}\t{stamp}",
                request.as_str(),
                intent.canonical()
            ))?;
        }
        if std::mem::take(&mut self.lose_next) {
            return Err(AuthorityError::Unavailable);
        }
        Ok(outcome)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        Ok(self
            .requests
            .get(request.as_str())
            .and_then(|(_, outcome)| Outcome::parse(outcome)))
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        if bill_id.is_empty() || bill_id.len() > 64 || bill_id.contains(['\t', '\n']) {
            return Err(AuthorityError::Invalid);
        }
        if let Some(number) = self.state.gst.get(bill_id) {
            return Ok(*number);
        }
        let stored = self
            .state
            .fixture
            .next_gst_bill_number(role)
            .map_err(rejected)?
            .0;
        let number = self
            .state
            .gst
            .values()
            .max()
            .map_or(stored, |max| stored.max(max + 1));
        self.append(&format!("G\t{bill_id}\t{number}"))?;
        self.state.gst.insert(bill_id.to_owned(), number);
        self.checkpoint += 1;
        Ok(number)
    }

    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        let snapshot = if checkpoint == Some(self.checkpoint) {
            None
        } else {
            Some(
                self.state
                    .fixture
                    .projection(role, &self.directory, &self.state.cashbook)
                    .map_err(rejected)?,
            )
        };
        Ok(ProjectionBatch {
            role,
            checkpoint: self.checkpoint,
            as_of_ms: (self.clock)(),
            snapshot,
        })
    }
}
