//! `alm-cli`: development checks over an authority's stores (synthetic data only).
//!
//! `alm-cli verify-legacy-import (--journal <path> | --postgres) [--manifest <id>]` re-parses
//! each stored legacy file with the core, rebuilds its import plan and compares it record by
//! record with the authority's manifest: a device's simulated authority (its journal) or the
//! Postgres authority at `ALM_PG_URL`. Exit status 0 when every manifest matches, 1 on a
//! finding, 2 on a usage or read error.
#![forbid(unsafe_code)]

use std::process::ExitCode;

mod verify;

const USAGE: &str =
    "usage: alm-cli verify-legacy-import (--journal <path> | --postgres) [--manifest <id>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, rest)) if command == "verify-legacy-import" => verify::run(rest),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
