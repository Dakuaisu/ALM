use alm_core::confirmation::{ConfirmationFixture, ReceiptMode};
use alm_core::receive_payment::{Customer, ReceivePayment};
use alm_core::{Money, Role};
use tauri::State;

use crate::billing::{CUSTOMER, customer_id};
use crate::{AppState, debug, with};

#[derive(serde::Deserialize)]
pub enum Mode {
    Cash,
    Online,
}

#[derive(serde::Deserialize)]
pub struct Terms {
    pub date: String,
    pub amount: String,
    pub mode: Mode,
}

pub fn save(fixture: &mut ConfirmationFixture, role: Role, terms: Terms) -> Result<(), String> {
    if role != Role::Owner {
        return Err("Access denied: Owner only.".into());
    }
    let date = terms.date.parse().map_err(debug)?;
    let amount = parse_amount(&terms.amount)
        .ok_or("Enter a positive amount with up to two decimal places.")?;
    let mode = match terms.mode {
        Mode::Cash => ReceiptMode::Cash,
        Mode::Online => ReceiptMode::Online,
    };
    let customers = [Customer {
        id: customer_id(),
        name: CUSTOMER.into(),
    }];
    let mut payment = ReceivePayment::default();
    payment.input(CUSTOMER, &customers);
    payment
        .save(role, fixture, date, amount, mode)
        .map_err(debug)
}

fn parse_amount(text: &str) -> Option<Money> {
    let (whole, fraction) = text.trim().split_once('.').unwrap_or((text.trim(), ""));
    if !whole
        .bytes()
        .chain(fraction.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole = whole.parse::<i64>().ok()?.checked_mul(100)?;
    let cents = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        2 => fraction.parse::<i64>().ok()?,
        _ => return None,
    };
    whole
        .checked_add(cents)
        .filter(|&value| value > 0)
        .map(Money::from_minor_units)
}

#[tauri::command(async)]
pub fn receipt_save(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "receipt", |trial| {
        let role = trial.require_owner()?;
        save(&mut trial.authority, role, terms)?;
        trial.message = "Standalone payment recorded in the local simulation.".into();
        Ok(())
    })
}
