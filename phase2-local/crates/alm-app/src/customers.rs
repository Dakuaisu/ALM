use crate::ipc::State;
use alm_core::confirmation::OpeningBalance;

use crate::{AppState, refusal, with};

#[derive(serde::Deserialize)]
pub struct Filter {
    q: String,
    mode: String,
}

#[derive(serde::Deserialize)]
pub struct Opening {
    pub customer: String,
    pub amount: String,
    pub date: String,
}

#[tauri::command(async)]
pub fn customers_filter(state: State<'_, AppState>, terms: Filter) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.require(alm_core::permissions::Capability::Customers)?;
        let view = &mut trial.customers;
        match terms.mode.as_str() {
            "due" => (view.due_only, view.query) = (true, String::new()),
            "all" => (view.due_only, view.query) = (false, String::new()),
            _ => (view.due_only, view.query) = (false, terms.q),
        }
        Ok(())
    })
}

#[tauri::command(async)]
pub fn customer_open(state: State<'_, AppState>, id: String) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.require_owner()?;
        trial
            .directory
            .customer(&trial.posted, &id)
            .map_err(refusal)?;
        trial.customers.open = Some(id);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn customer_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.customers.open = None;
        trial.customers.form = None;
        Ok(())
    })
}

/// The Add / Edit Customer form (`editCustomer`): `new`, or a customer's ID.
#[tauri::command(async)]
pub fn customer_edit(state: State<'_, AppState>, id: String) -> Result<String, String> {
    with(&state, "customers", |trial| trial.customer_edit(&id))
}

#[derive(serde::Deserialize)]
pub struct Contact {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mobile: String,
    #[serde(default)]
    pub addr1: String,
    #[serde(default)]
    pub addr2: String,
}

#[tauri::command(async)]
pub fn customer_save(state: State<'_, AppState>, terms: Contact) -> Result<String, String> {
    with(&state, "customers", |trial| trial.customer_save(terms))
}

impl crate::Trial {
    pub(crate) fn customer_edit(&mut self, id: &str) -> Result<(), String> {
        self.require(alm_core::permissions::Capability::SaveCustomer)?;
        let contact = if id == "new" {
            alm_core::confirmation::owner_billing::CustomerContact {
                id: None,
                name: String::new(),
                mobile: String::new(),
                addr1: String::new(),
                addr2: String::new(),
            }
        } else {
            let c = self.directory.customer(&self.posted, id).map_err(refusal)?;
            alm_core::confirmation::owner_billing::CustomerContact {
                id: Some(c.id),
                name: c.name,
                mobile: c.mobile,
                addr1: c.addr1,
                addr2: c.addr2,
            }
        };
        self.customers.form = Some(contact);
        Ok(())
    }

    /// Checks the form on a copy of the directory as the authority will, then sends it
    /// (DEC-034: the Owner and a Manager).
    pub(crate) fn customer_save(&mut self, form: Contact) -> Result<(), String> {
        let role = self.require(alm_core::permissions::Capability::SaveCustomer)?;
        let contact = alm_core::confirmation::owner_billing::CustomerContact {
            id: Some(form.id).filter(|id| !id.is_empty()),
            name: form.name,
            mobile: form.mobile,
            addr1: form.addr1,
            addr2: form.addr2,
        };
        let (mut directory, mut scratch) = (self.directory.clone(), self.posted.fixture.clone());
        directory
            .save_customer(&mut scratch, role, &contact)
            .map_err(|error| error.warning().unwrap_or_else(|| refusal(error)))?;
        self.send(&alm_sync::Intent::SaveCustomer(contact))?;
        self.customers.form = None;
        self.message = "Customer saved ✓".into();
        Ok(())
    }
}

pub fn save_opening(trial: &mut crate::Trial, terms: Opening) -> Result<(), String> {
    trial.require_owner()?;
    let customer = trial
        .directory
        .customer(&trial.posted, &terms.customer)
        .map_err(refusal)?;
    let amount = OpeningBalance::parse_amount(&terms.amount).map_err(|_| {
        "Enter an amount with up to two decimal places (negative for an advance).".to_owned()
    })?;
    let date = if terms.date.is_empty() {
        None
    } else {
        Some(terms.date.parse().map_err(refusal)?)
    };
    let (id, name) = (customer.id.to_owned(), customer.name.to_owned());
    trial.send(&alm_sync::Intent::SaveOpening {
        customer: id.clone(),
        amount,
        date,
    })?;
    trial.customers.open = Some(id);
    trial.message = format!("Opening balance saved for {name}.");
    Ok(())
}

#[tauri::command(async)]
pub fn customer_opening(state: State<'_, AppState>, terms: Opening) -> Result<String, String> {
    with(&state, "customers", |trial| save_opening(trial, terms))
}
