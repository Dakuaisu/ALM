//! What a refusal says on screen: the HTML's toast wording where it has one, plain words
//! otherwise; never a Rust type name or debug output. The refusal is read from its debug text
//! (the innermost known variant wins), so every core, sync and store error maps here.

/// The HTML's toast for a bill or form row without an item or quantity (`saveBill`).
pub(crate) const NO_ITEM: &str = "Add at least one item with quantity";
/// The HTML's toast for a missing or invalid amount; also what a refused amount reads as.
pub(crate) const ENTER_AMOUNT: &str = "Enter amount";
/// DEC-034: what a Manager reads for an Owner-only action.
pub(crate) const OWNER_ONLY: &str = alm_core::permissions::OWNER_ONLY;
const FALLBACK: &str = "That didn't work — check the entries and try again.";

fn wording(token: &str) -> Option<&'static str> {
    Some(match token {
        "MissingBillableItems" | "MissingItems" | "NoUsableRows" | "TooManyLines" => NO_ITEM,
        "Empty" => "Add at least one item to the crate",
        "EmptyCart" => "Cart is empty — press ➕ on the items you want to order",
        "MissingCustomerName" | "MissingCustomer" | "PickCustomer" | "Customer" => {
            "Pick a customer"
        }
        "MissingBillDate" | "Date" => "Pick the bill date",
        "MissingDate" | "MissingSaleDate" | "DateOutOfRange" => "Pick the date",
        "InvalidGstin" => "GSTIN must be exactly 15 characters",
        "InvalidEway" => "E-Way Bill number must be 12 digits",
        "ItemNameRequired" => "Enter item name",
        "MissingSupplier" => "Enter supplier",
        "InvalidCount" | "InvalidFridgeCount" => "Enter the counted quantity",
        "InvalidCases" | "QuantityOutOfRange" => "Enter quantity (cases)",
        "InvalidReceipt" => "Enter at least one quantity to receive",
        "InvalidAmount" | "Invalid" | "InvalidPrice" => ENTER_AMOUNT,
        "LoadingCharge" => "Enter the loading charge in rupees per packet.",
        "OrderNotFound" => "PO not found",
        "OrderNotPending" => "This purchase order is already closed.",
        "StaleOrderNote" => "This purchase order changed — open it again.",
        "AccessDenied" | "Denied" => "Only the Owner can do this.",
        "StaleReview" => {
            "Prices, stock or the bill changed since this was reviewed — review it again."
        }
        "InsufficientStock" => "Not enough stock for this bill.",
        "DraftAlreadyPosted" | "AlreadyPosted" => "This bill is already posted.",
        "RequestConflict" | "Conflict" | "ProductIdInUse" => {
            "Another change got there first — open it again and retry."
        }
        "DraftNotFound" | "NotFound" | "RowNotFound" | "SaleNotFound" | "ProductNotFound"
        | "SupplierNotFound" => "Not found — it may have been deleted.",
        "UnknownProduct" => "Select a connected product.",
        "InvalidDraft" | "UnsupportedCrateRow" => {
            "This bill can't be priced as entered — check its items and crates."
        }
        "ProductInUse" => "This product is on posted bills.",
        "MissingPieceMrp" => "Set the item's MRP to price it by the piece.",
        "Cancelled" => "This bill is cancelled.",
        "Deleted" => "This bill was deleted.",
        "Overflow" | "AmountOverflow" | "ReceiptOverflow" | "VersionOverflow" => {
            "The amount is too large."
        }
        "UnsupportedRounding" | "InvalidMetadata" | "MissingTax" | "Calculation" => {
            "This amount can't be split into exact GST."
        }
        "UnsupportedSalesRecord"
        | "UnsupportedRecord"
        | "InvalidLineDisplay"
        | "UnknownFrozenCost" => "This bill can't be shown here.",
        "NoSavedDraft" | "ActiveDraftMismatch" | "DurableStoreRequired" => {
            "Save the bill as a draft first."
        }
        "NoReview" => "Review the sale before confirming.",
        "ReviewLimit" => "Too many reviews are open — confirm or clear one first.",
        "Unavailable" => "Sync failed — check URL & internet",
        "SignedOut" => "Signed out — sign in again.",
        "Refused" => "Sign-in refused — check the email and password.",
        "InvalidArchive" | "NotJson" | "InvalidSnapshot" | "InvalidOutput" => "Invalid backup file",
        "Io"
        | "Storage"
        | "Sqlite"
        | "Corrupt"
        | "Json"
        | "System"
        | "InvalidPath"
        | "InvalidAccountScope"
        | "AccountStoreMismatch" => "Could not save — use folder/cloud backup!",
        "NoProducts" => "This backup holds no products.",
        "NotQuarantined" | "AlreadyResolved" | "AlreadyImported" => {
            "This record was already handled."
        }
        "NoReason" => "Enter a reason.",
        "Unresolved" => "Resolve the held records first.",
        "Missing" | "DigestMismatch" => "The legacy file changed or is missing — choose it again.",
        _ => return None,
    })
}

const PIECES: &str = "The crate's pieces don't make whole packets.";

/// New Bill's pricing refusal; a crate amount refused means pieces that don't make packets.
pub(crate) fn pricing(
    error: &alm_core::confirmation::owner_billing::NewBillPricingError,
) -> String {
    use alm_core::confirmation::ConfirmationError;
    use alm_core::confirmation::owner_billing::NewBillPricingError;
    match error {
        NewBillPricingError::Crates(ConfirmationError::InvalidAmount) => PIECES.to_owned(),
        other => user_message(&format!("{other:?}")),
    }
}

/// A crate row's pricing refusal (the crate post on New Bill).
pub(crate) fn crate_pricing(error: &alm_core::confirmation::ConfirmationError) -> String {
    match error {
        alm_core::confirmation::ConfirmationError::InvalidAmount => PIECES.to_owned(),
        other => user_message(&format!("{other:?}")),
    }
}

/// SQL and REST refusals carry the authority's reason text (`invalid_terms: whole packs`).
fn reason(text: &str) -> Option<&'static str> {
    let text = text.to_ascii_lowercase();
    Some(if text.contains("whole packs") {
        PIECES
    } else if text.contains("stock") {
        "Not enough stock for this bill."
    } else if text.contains("customer") {
        "Pick a customer"
    } else if text.contains("date") {
        "Pick the bill date"
    } else if text.contains("stale") {
        "Prices, stock or the bill changed since this was reviewed — review it again."
    } else if text.contains("posted") {
        "This bill is already posted."
    } else {
        return None;
    })
}

/// The on-screen words for a refusal given its debug text.
pub(crate) fn user_message(debug: &str) -> String {
    let mut found = None;
    let mut word = String::new();
    let mut quoted: Option<String> = None;
    let mut escaped = false;
    for ch in debug.chars().chain(std::iter::once(' ')) {
        if let Some(text) = quoted.as_mut() {
            if escaped {
                escaped = false;
                text.push(ch);
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                let text = quoted.take().unwrap_or_default();
                if let Some(message) = wording(&text).or_else(|| reason(&text)) {
                    found = Some(message.to_owned());
                } else if text.contains(' ') && !text.contains(['{', '}', ':']) {
                    found = Some(text);
                }
            } else {
                text.push(ch);
            }
            continue;
        }
        if ch == '"' {
            quoted = Some(String::new());
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
            continue;
        }
        if let Some(message) = wording(&word) {
            found = Some(message.to_owned());
        }
        word.clear();
    }
    found.unwrap_or_else(|| FALLBACK.to_owned())
}

#[cfg(test)]
mod tests;
