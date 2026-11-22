//! Static replay of the pricing and hard-deletion rows in docs/PHASE1_CASES.md.
//! The frozen HTML is a source reference, not a runtime dependency of this target.

#![forbid(unsafe_code)]

#[path = "l20_records_replay/guards.rs"]
mod guards;
#[path = "l20_records_replay/merge.rs"]
mod merge;
#[path = "l20_records_replay/model.rs"]
mod model;
#[path = "l20_records_replay/price_columns.rs"]
mod price_columns;
#[path = "l20_records_replay/pricing.rs"]
mod pricing;
#[path = "l20_records_replay/replay.rs"]
mod replay;
#[path = "l20_records_replay/saved.rs"]
mod saved;
#[path = "l20_records_replay/tombstones.rs"]
mod tombstones;
