//! `alm-cli verify-legacy-import`: an honest import verifies; a plan altered by the device
//! while sending the true file (its digest matches, so the authority accepts it) is caught
//! record by record. Against a simulated authority's journal, and (opt-in, `ALM_PG_URL` set to
//! a disposable migrated database) against Postgres.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use alm_core::Role;
use alm_core::confirmation::demo_fixture::{directory, fixture};
use alm_core::legacy::import::{ImportPlan, plan_file};
use alm_sync::{Intent, LocalAuthority, Outcome, RequestId, SalesAuthority};

const A: &[u8] = include_bytes!("../../alm-core/tests/legacy/rehearsal-a.json");
const B: &[u8] = include_bytes!("../../alm-core/tests/legacy/rehearsal-b.json");

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("alm-cli-verify-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// B's plan with one payment's mapped amount changed after mapping; the file is untouched, so
/// its digest still matches.
fn tampered_b() -> ImportPlan {
    let (_, mut plan) = plan_file(B).unwrap();
    let record = plan
        .records
        .iter_mut()
        .find(|r| r.kind == "payment")
        .expect("a payment in B");
    let amount = record.body["amount"].as_i64().unwrap();
    record.body["amount"] = serde_json::json!(amount + 100_000);
    plan
}

fn verify(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_alm-cli"))
        .arg("verify-legacy-import")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn submit(authority: &mut LocalAuthority, tag: &str, plan: ImportPlan) -> Outcome {
    authority
        .submit(
            Role::Owner,
            &RequestId::new(&format!("verify-{tag}")).unwrap(),
            &Intent::ImportLegacy(Box::new(plan)),
        )
        .unwrap()
}

fn journal(path: &Path) -> LocalAuthority {
    LocalAuthority::open(path, fixture().unwrap(), directory()).unwrap()
}

#[test]
fn an_honest_import_verifies_and_a_tampered_plan_is_caught_in_the_journal() {
    let scratch = Scratch::new("journal");
    let path = scratch.0.join("journal.txt");
    let mut authority = journal(&path);
    assert_eq!(
        submit(&mut authority, "a", plan_file(A).unwrap().1),
        Outcome::Created(1)
    );
    let path_text = path.to_str().unwrap();
    let (code, out) = verify(&["--journal", path_text]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("manifest 1: OK"), "{out}");

    // The authority accepts the altered plan: it checks only the file's digest.
    assert_eq!(
        submit(&mut authority, "b", tampered_b()),
        Outcome::Created(2)
    );
    drop(authority);
    let (code, out) = verify(&["--journal", path_text]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("manifest 1: OK"), "{out}");
    assert!(out.contains("manifest 2: 1 finding(s)"), "{out}");
    assert!(
        out.contains("payment") && out.contains("differs from the file's mapping"),
        "{out}"
    );
    let (code, out) = verify(&["--journal", path_text, "--manifest", "1"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(verify(&["--journal"]).0, 2);
}

#[test]
fn a_tampered_plan_is_caught_in_postgres() {
    let Ok(url) = std::env::var("ALM_PG_URL") else {
        return;
    };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    client
        .batch_execute(
            "TRUNCATE public.legacy_resolutions, public.legacy_records, public.legacy_manifests,
                 public.legacy_sources;
             DELETE FROM public.command_requests WHERE request_id LIKE 'cli-verify-%';
             SELECT set_config('request.jwt.claims',
                 '{\"app_metadata\": {\"role\": \"owner\"}}', false);
             SELECT set_config('alm.allow_claim_roles', 'on', false);",
        )
        .unwrap();
    for (tag, plan) in [("a", plan_file(A).unwrap().1), ("b", tampered_b())] {
        let terms = serde_json::to_string(&plan).unwrap();
        client
            .query(
                "SELECT public.import_legacy($1, $2::text::jsonb)",
                &[&format!("cli-verify-{tag}"), &terms],
            )
            .unwrap();
    }
    // A file changed after mapping is refused by the server.
    let (_, mut file) = plan_file(A).unwrap();
    file.source = file
        .source
        .replacen("\"amount\": 91,", "\"amount\": 92,", 1);
    let refused = client.query(
        "SELECT public.import_legacy('cli-verify-c', $1::text::jsonb)",
        &[&serde_json::to_string(&file).unwrap()],
    );
    let error = refused.unwrap_err();
    assert_eq!(
        error.as_db_error().map(|e| (e.code().code(), e.message())),
        Some(("ALM05", "invalid_terms: source digest"))
    );
    let (code, out) = verify(&["--postgres"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("manifest 1: OK"), "{out}");
    assert!(out.contains("manifest 2: 1 finding(s)"), "{out}");
}
