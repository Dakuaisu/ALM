use alm_phase2_local::Role;

#[path = "l20_mirror_replay/cases.rs"]
mod cases;
#[path = "l20_mirror_replay/database.rs"]
mod database;
#[path = "l20_mirror_replay/fixture.rs"]
mod fixture;
#[path = "l20_mirror_replay/persistence.rs"]
mod persistence;
#[path = "l20_mirror_replay/refresh.rs"]
mod refresh;
#[path = "l20_mirror_replay/store.rs"]
mod store;
#[path = "l20_mirror_replay/summary.rs"]
mod summary;

const OWNER: Role = Role::Owner;
