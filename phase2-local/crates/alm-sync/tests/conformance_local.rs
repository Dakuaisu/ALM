//! The conformance suite against the journaled development authority.
#![forbid(unsafe_code)]

mod conformance;

use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_sync::LocalAuthority;

fn local() -> Option<LocalAuthority> {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    Some(LocalAuthority::new(shop, directory()).with_clock(|| 1_700_000_000_000))
}

conformance_tests!(local());
