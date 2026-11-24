//! `verify-legacy-import`: loads the manifests, records and stored source files from the
//! authority and runs the core verifier on each manifest.
use std::process::ExitCode;

use alm_core::legacy::import::{ImportManifest, ImportedRecord};
use alm_core::legacy::verify::verify;
use alm_sync::{LegacyStore, LocalAuthority};

enum Source {
    Journal(String),
    Postgres,
}

fn parse(args: &[String]) -> Result<(Source, Option<u64>), String> {
    let mut source = None;
    let mut manifest = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--journal" => {
                source = Some(Source::Journal(
                    args.next().ok_or("--journal needs a path")?.clone(),
                ));
            }
            "--postgres" => source = Some(Source::Postgres),
            "--manifest" => {
                manifest = Some(
                    args.next()
                        .and_then(|id| id.parse().ok())
                        .ok_or("--manifest needs a number")?,
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok((source.ok_or(super::USAGE)?, manifest))
}

fn postgres() -> Result<LegacyStore, String> {
    let url = std::env::var("ALM_PG_URL").map_err(|_| "set ALM_PG_URL".to_owned())?;
    let mut client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| e.to_string())?;
    let mut store = LegacyStore::default();
    for row in client
        .query(
            "select id, digest, version, entries::text from public.legacy_manifests order by id",
            &[],
        )
        .map_err(|e| e.to_string())?
    {
        let id: i64 = row.get(0);
        let entries: String = row.get(3);
        store.imports.manifests.push(ImportManifest {
            id: u64::try_from(id).map_err(|e| e.to_string())?,
            digest: row.get(1),
            version: row.get(2),
            entries: serde_json::from_str(&entries).map_err(|e| e.to_string())?,
        });
    }
    for row in client
        .query(
            "select authority_id, kind, legacy_id, body::text, manifest
             from public.legacy_records order by seq",
            &[],
        )
        .map_err(|e| e.to_string())?
    {
        let body: String = row.get(3);
        let manifest: i64 = row.get(4);
        store.imports.records.push(ImportedRecord {
            authority_id: row.get(0),
            kind: row.get(1),
            legacy_id: row.get(2),
            body: serde_json::from_str(&body).map_err(|e| e.to_string())?,
            manifest: u64::try_from(manifest).map_err(|e| e.to_string())?,
        });
    }
    for row in client
        .query("select digest, source from public.legacy_sources", &[])
        .map_err(|e| e.to_string())?
    {
        let source: String = row.get(1);
        store.sources.insert(row.get(0), source.into());
    }
    Ok(store)
}

/// The report lines and whether every checked manifest matched.
pub fn report(store: &LegacyStore, only: Option<u64>) -> (Vec<String>, bool) {
    let LegacyStore { imports, sources } = store;
    let mut lines = Vec::new();
    let mut clean = true;
    let manifests: Vec<_> = imports
        .manifests
        .iter()
        .filter(|m| only.is_none_or(|id| m.id == id))
        .collect();
    if manifests.is_empty() {
        lines.push("no manifest to verify".to_owned());
        return (lines, only.is_none());
    }
    for manifest in manifests {
        let Some(source) = sources.get(&manifest.digest) else {
            clean = false;
            lines.push(format!(
                "manifest {}: no stored source file for {}",
                manifest.id, manifest.digest
            ));
            continue;
        };
        let findings = verify(source.as_bytes(), manifest, imports);
        if findings.is_empty() {
            lines.push(format!(
                "manifest {}: OK ({} entries match the stored file {})",
                manifest.id,
                manifest.entries.len(),
                manifest.digest
            ));
        } else {
            clean = false;
            lines.push(format!(
                "manifest {}: {} finding(s) against the stored file {}",
                manifest.id,
                findings.len(),
                manifest.digest
            ));
            lines.extend(findings.iter().map(|f| format!("  {f}")));
        }
    }
    (lines, clean)
}

pub fn run(args: &[String]) -> ExitCode {
    let (source, manifest) = match parse(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let store = match source {
        Source::Journal(path) => LocalAuthority::legacy_journal(std::path::Path::new(&path))
            .map_err(|e| format!("{path}: {e:?}")),
        Source::Postgres => postgres(),
    };
    let store = match store {
        Ok(store) => store,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let (lines, clean) = report(&store, manifest);
    for line in lines {
        println!("{line}");
    }
    if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
