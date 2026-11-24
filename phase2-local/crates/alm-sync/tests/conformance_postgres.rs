//! Opt-in: the conformance suite against the migrated local schema through the dev Postgres
//! authority. Set `ALM_PG_URL` to a disposable database with both migrations applied (and,
//! on plain Postgres, `supabase/tests/plain_postgres/bootstrap.sql`); without it every test
//! returns early.
#![forbid(unsafe_code)]

mod conformance;
mod pg_authority;

use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_sync::LocalAuthority;
use pg_authority::PgAuthority;

/// Seeded with the same shop as the local authority's.
fn postgres() -> Option<PgAuthority> {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    PgAuthority::connect(&LocalAuthority::new(shop, directory()))
}

conformance_tests!(postgres());
