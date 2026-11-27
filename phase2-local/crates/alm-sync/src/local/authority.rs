//! The simulation as a `SalesAuthority`: the DEC-034 matrix gate and Manager limits (as the
//! SQL `require_permission` and `manager_limits`), the journal and the lost-response hook.
use alm_core::Role;
use alm_core::permissions::{self, Capability};
use alm_core::projection::ProjectionBatch;

use super::{LocalAuthority, rejected};
use crate::{AuthorityError, Intent, Lookup, Outcome, RequestId, SalesAuthority, wire};

impl LocalAuthority {
    /// The role the authority grants for a call claiming `role`: never more than either.
    pub(super) const fn effective(&self, role: Role) -> Result<Role, AuthorityError> {
        match (self.granted, role) {
            (None, _) => Err(AuthorityError::Denied),
            (Some(Role::Manager), _) | (_, Role::Manager) => Ok(Role::Manager),
            (Some(Role::Owner), Role::Owner) => Ok(Role::Owner),
        }
    }

    fn gate(&self, role: Role, capability: Capability) -> Result<Role, AuthorityError> {
        let role = self.effective(role)?;
        if permissions::allows(role, capability) {
            Ok(role)
        } else {
            Err(AuthorityError::Denied)
        }
    }

    /// A Manager sees only the requests its own account sent; `false` when unknown.
    fn known_to(&self, role: Role, request: &RequestId) -> Result<bool, AuthorityError> {
        match self.senders.get(request.as_str()) {
            None => Ok(false),
            Some(sender) if role == Role::Owner || *sender == self.caller_of(role).1 => Ok(true),
            Some(_) => Err(AuthorityError::Denied),
        }
    }

    /// The Manager's narrower rules on shared commands. A retry of its own stored request
    /// replays as stored.
    fn manager_limits(&self, request: &RequestId, intent: &Intent) -> Result<(), AuthorityError> {
        if self.known_to(Role::Manager, request)? {
            return Ok(());
        }
        if let Intent::SaveCashDay { date, .. } = intent {
            let today = alm_core::projection::shop_today((self.clock)());
            let closed = self
                .state
                .cashbook
                .rows(Role::Owner)
                .map_err(rejected)?
                .iter()
                .any(|row| row.date == *date);
            if today != Some(*date) || closed {
                return Err(AuthorityError::Denied);
            }
        }
        Ok(())
    }
}

impl SalesAuthority for LocalAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        self.caller = self.caller_of(role).clone();
        if self.effective(role)? == Role::Manager {
            // As SQL: the function's matrix row first, then the Manager's own stored request
            // replays (or conflicts) as stored, else the capability and its limits apply.
            let functions = permissions::sql_functions();
            let shared = permissions::rule(intent.capability()).sql.iter().any(|f| {
                functions
                    .iter()
                    .any(|(name, manager)| name == f && *manager)
            });
            if !shared {
                return Err(AuthorityError::Denied);
            }
            if !self.known_to(Role::Manager, request)? {
                self.gate(Role::Manager, intent.capability())?;
                self.manager_limits(request, intent)?;
            }
        } else {
            self.gate(role, intent.capability())?;
        }
        let known = self.requests.contains_key(request.as_str());
        let stamp = (self.clock)();
        let outcome = self.commit(request, intent, stamp)?;
        if !known {
            self.append(&format!(
                "R\t{}\t{}\t{stamp}\t{}\t{}",
                request.as_str(),
                intent.canonical(),
                self.caller.0,
                self.caller.1
            ))?;
        }
        if std::mem::take(&mut self.lose_next) {
            return Err(AuthorityError::Unavailable);
        }
        Ok(outcome)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        let role = self.gate(role, Capability::RequestStatus)?;
        if !self.known_to(role, request)? {
            return Ok(None);
        }
        Ok(self
            .requests
            .get(request.as_str())
            .and_then(|(_, outcome)| Outcome::parse(outcome)))
    }

    fn lookup(
        &self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Lookup, AuthorityError> {
        let role = self.gate(role, Capability::RequestLookup)?;
        if !self.known_to(role, request)? {
            return Ok(Lookup::NotCommitted);
        }
        Ok(match self.requests.get(request.as_str()) {
            None => Lookup::NotCommitted,
            Some((stored, _)) if *stored != intent.canonical() => Lookup::OtherIntent,
            Some((_, outcome)) => {
                Lookup::Committed(Outcome::parse(outcome).ok_or(AuthorityError::Invalid)?)
            }
        })
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        let role = self.gate(role, Capability::AllocateGstNumber)?;
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
        // The authority answers with the role it grants now, never more than asked for.
        let role = self.gate(role, Capability::Pull)?;
        let requested = role;
        let snapshot = if checkpoint == Some(self.checkpoint) {
            None
        } else {
            let mut snapshot = self
                .state
                .fixture
                .projection(role, &self.directory, &self.state.cashbook)
                .map_err(rejected)?;
            if let Some(owner) = snapshot.owner.as_mut() {
                owner.book.activity.clone_from(&self.state.activity);
                owner.book.legacy.clone_from(&self.state.legacy);
            }
            Some(snapshot)
        };
        // The same per-screen rows `pull_projection` sends, read back through the one decoder
        // every transport uses; the simulation's read state travels only as its `dev_state`.
        let batch = ProjectionBatch {
            role,
            checkpoint: self.checkpoint,
            as_of_ms: (self.clock)(),
            snapshot,
            rows: None,
        };
        let mut value = wire::encode_batch(&batch);
        if !value["rows"].is_null() {
            let claim = crate::wire::role_claim(role);
            let mut changes = self
                .changes
                .lock()
                .map_err(|_| AuthorityError::Unavailable)?;
            changes.sync(claim, self.checkpoint, &value["rows"]);
            // Changes only for a device holding this role's rows at a retained checkpoint.
            if let Some(base) = checkpoint.filter(|_| role == requested)
                && let Some(delta) = changes.delta(claim, base, &value["rows"])
            {
                value["full"] = false.into();
                value["rows"] = serde_json::Value::Null;
                value["delta"] = delta;
            } else {
                value["row_hashes"] = crate::delta::hashes(&value["rows"]);
            }
        }
        wire::decode_batch(&value, role)
    }
}
