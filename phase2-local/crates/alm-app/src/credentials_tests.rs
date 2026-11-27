//! The refresh token across launches, against a stand-in GoTrue on 127.0.0.1 (synthetic,
//! unsigned tokens; the REST URL answers nothing): sign-in keeps only the refresh token, a
//! relaunch signs in silently, a revoked token leads to the sign-in screen, and sign-out,
//! revocation and downgrade delete it.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::credentials::MemoryStore;
use crate::dev_authority::{LoginForm, RestForm};
use crate::{Trial, billing_tests::day};

const USER: &str = "fake-owner-id";

#[derive(Default)]
struct Auth {
    revoked: bool,
    issued: u32,
    password_logins: u32,
}

fn base64url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(TABLE[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}

fn session(auth: &mut Auth, role: &str) -> String {
    auth.issued += 1;
    let exp = 4_102_444_800_u64;
    let claims = format!(
        r#"{{"sub":"{USER}","email":"owner@alm.test","exp":{exp},"app_metadata":{{"role":"{role}"}}}}"#
    );
    let access = format!("e30.{}.c2ln", base64url(claims.as_bytes()));
    format!(
        r#"{{"access_token":"{access}","refresh_token":"refresh-{}","expires_at":{exp}}}"#,
        auth.issued
    )
}

/// Serves `/token` (password and refresh grants) and `/logout`, one request per connection.
fn gotrue(role: &'static str) -> (String, Arc<Mutex<Auth>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(Mutex::new(Auth::default()));
    let shared = state.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let mut auth = shared.lock().unwrap();
            let (status, answer) = if line.contains("grant_type=password") {
                auth.password_logins += 1;
                auth.revoked = false;
                ("200 OK", session(&mut auth, role))
            } else if line.contains("grant_type=refresh_token") {
                if auth.revoked {
                    (
                        "400 Bad Request",
                        r#"{"error_description":"Invalid Refresh Token"}"#.to_owned(),
                    )
                } else {
                    ("200 OK", session(&mut auth, role))
                }
            } else if line.contains("/logout") {
                auth.revoked = true;
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            };
            drop(auth);
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            );
        }
    });
    (url, state)
}

fn device(root: &std::path::Path, store: &MemoryStore) -> Trial {
    let mut trial = Trial::open(root, Some(day())).unwrap();
    trial.credentials = Box::new(store.clone());
    trial.resume_session().unwrap();
    trial
}

fn rest_device(root: &std::path::Path, store: &MemoryStore, auth: &str) -> Trial {
    let mut trial = device(root, store);
    trial
        .settings_rest(&RestForm::new("http://127.0.0.1:9", auth, ""))
        .unwrap();
    trial
}

#[test]
fn a_relaunch_signs_in_silently_with_the_stored_refresh_token_and_sign_out_deletes_it() {
    let root = crate::screen_tests::Root::new("credentials-relaunch");
    let (auth, state) = gotrue("owner");
    let store = MemoryStore::default();
    let mut trial = rest_device(&root.0, &store, &auth);
    trial
        .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
        .unwrap();
    // Only the refresh token is kept, under the account; never the password or access token.
    assert_eq!(store.entries().len(), 1);
    assert_eq!(store.entries()[USER], "refresh-1");
    assert!(
        !store
            .entries()
            .values()
            .any(|v| v.contains("synthetic-password") || v.contains("e30."))
    );
    drop(trial);

    // Relaunch: signed in without a password; the rotated refresh token replaces the old one.
    let mut trial = device(&root.0, &store);
    assert!(!trial.signed_out(), "{}", trial.message);
    assert_eq!(
        trial.signed_in().map(|(email, _)| email),
        Some("owner@alm.test")
    );
    assert_eq!(state.lock().unwrap().password_logins, 1);
    assert_eq!(store.entries()[USER], "refresh-2");
    assert!(
        !trial
            .render("settings")
            .unwrap()
            .contains(r#"data-action="rest_login""#)
    );

    // Sign-out ends the session at the service and deletes the stored token.
    trial.rest_logout().unwrap();
    assert!(store.entries().is_empty());
    assert!(!root.0.join("signed-in-account.txt").exists());
    drop(trial);
    let trial = device(&root.0, &store);
    assert!(trial.signed_out());
}

#[test]
fn a_revoked_refresh_token_leads_to_the_sign_in_screen_and_is_deleted() {
    let root = crate::screen_tests::Root::new("credentials-revoked");
    let (auth, state) = gotrue("owner");
    let store = MemoryStore::default();
    let mut trial = rest_device(&root.0, &store, &auth);
    trial
        .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
        .unwrap();
    drop(trial);
    // Signed out elsewhere: the service revoked every refresh token of the session.
    state.lock().unwrap().revoked = true;
    let mut trial = device(&root.0, &store);
    assert!(trial.signed_out());
    assert_eq!(trial.message, "Your session ended: sign in again.");
    assert!(store.entries().is_empty(), "the refused token is deleted");
    assert!(
        trial
            .render("bill")
            .unwrap()
            .contains(r#"data-action="rest_login""#)
    );
}

#[test]
fn a_token_without_a_role_and_a_stale_marker_leave_nothing_stored() {
    let root = crate::screen_tests::Root::new("credentials-no-role");
    let (auth, _) = gotrue("none");
    let store = MemoryStore::default();
    let mut trial = rest_device(&root.0, &store, &auth);
    assert!(
        trial
            .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
            .is_err()
    );
    assert!(store.entries().is_empty());
    // A marker without a stored token (deleted elsewhere) is dropped at launch.
    std::fs::write(root.0.join("signed-in-account.txt"), USER).unwrap();
    let trial = device(&root.0, &store);
    assert!(trial.signed_out());
    assert!(!root.0.join("signed-in-account.txt").exists());
}

#[test]
fn a_revocation_or_downgrade_seen_by_the_device_deletes_the_stored_token() {
    for downgrade in [false, true] {
        let root = crate::screen_tests::Root::new("credentials-purge");
        let (auth, _) = gotrue("owner");
        let store = MemoryStore::default();
        let mut trial = rest_device(&root.0, &store, &auth);
        trial
            .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
            .unwrap();
        assert_eq!(store.entries().len(), 1);
        if downgrade {
            let _ = trial.downgrade(alm_core::Role::Manager);
        } else {
            trial.revoke().unwrap();
        }
        assert!(store.entries().is_empty(), "downgrade {downgrade}");
        assert!(!root.0.join("signed-in-account.txt").exists());
        // DEC-034: the session ends at once; the device asks for a sign-in.
        assert!(
            trial.session.is_none() && trial.signed_out(),
            "downgrade {downgrade}"
        );
        assert!(trial.require_session().is_err());
        // A later refresh is not written back.
        let _ = trial.session_token();
        assert!(store.entries().is_empty(), "downgrade {downgrade}");
    }
}

fn files_containing(dir: &std::path::Path, needle: &[u8], found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_containing(&path, needle, found);
        } else if std::fs::read(&path)
            .unwrap()
            .windows(needle.len())
            .any(|w| w == needle)
        {
            found.push(path);
        }
    }
}

#[test]
fn a_credential_store_that_refuses_the_token_leaves_the_session_in_memory_only() {
    let root = crate::screen_tests::Root::new("credentials-unavailable");
    let (auth, state) = gotrue("owner");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.credentials = Box::new(crate::credentials::UnavailableStore);
    trial.resume_session().unwrap();
    trial
        .settings_rest(&RestForm::new("http://127.0.0.1:9", &auth, ""))
        .unwrap();
    trial
        .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
        .unwrap();
    // Signed in, told the session will not be remembered.
    assert_eq!(
        trial.signed_in().map(|(email, _)| email),
        Some("owner@alm.test")
    );
    assert!(
        trial.message.contains("will not be remembered"),
        "{}",
        trial.message
    );
    let settings = trial.render("settings").unwrap();
    assert!(settings.contains("will not be remembered"));
    assert!(!settings.contains(r#"data-action="rest_login""#));
    // A refresh still works in memory and is not written anywhere.
    trial.session_token().unwrap();
    assert!(!root.0.join("signed-in-account.txt").exists());
    let mut found = Vec::new();
    for needle in ["refresh-", "synthetic-password", "e30."] {
        files_containing(&root.0, needle.as_bytes(), &mut found);
    }
    assert!(found.is_empty(), "plaintext secrets on disk: {found:?}");
    // Sign-out works although the store cannot be reached.
    trial.rest_logout().unwrap();
    assert!(trial.signed_out());
    drop(trial);
    // Relaunch: nothing to resume; the sign-in screen shows.
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.credentials = Box::new(crate::credentials::UnavailableStore);
    trial.resume_session().unwrap();
    assert!(trial.signed_out());
    assert_eq!(state.lock().unwrap().password_logins, 1);
    // A marker left by an earlier build with a store that now cannot be read is dropped.
    std::fs::write(root.0.join("signed-in-account.txt"), USER).unwrap();
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.credentials = Box::new(crate::credentials::UnavailableStore);
    trial.resume_session().unwrap();
    assert!(trial.signed_out());
    assert!(!root.0.join("signed-in-account.txt").exists());
}

/// The real Keychain / Credential Manager under a test-only service name. With
/// `ALM_EXPECT_NO_KEYCHAIN=1` (the macOS CI job runs this with an empty `HOME`, which has no
/// default keychain) the store must refuse and sign-in must still succeed, not remembered.
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn sign_in_with_the_platform_store_is_remembered_or_says_it_is_not() {
    let expect_failure = std::env::var("ALM_EXPECT_NO_KEYCHAIN").is_ok_and(|v| v == "1");
    let root = crate::screen_tests::Root::new("credentials-platform");
    let (auth, _) = gotrue("owner");
    let store = || crate::credentials::PlatformStore::new("in.alm.trial.test.session");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.credentials = Box::new(store());
    trial
        .settings_rest(&RestForm::new("http://127.0.0.1:9", &auth, ""))
        .unwrap();
    trial
        .rest_login(&LoginForm::new("owner@alm.test", "synthetic-password"))
        .unwrap();
    assert!(!trial.signed_out());
    let stored = crate::credentials::CredentialStore::load(&store(), USER);
    if expect_failure {
        assert!(stored.is_err(), "the store was expected to be unavailable");
    }
    match stored {
        Ok(token) => {
            assert!(!trial.not_remembered, "{}", trial.message);
            assert!(token.is_some_and(|t| t.starts_with("refresh-")));
            drop(trial);
            let mut trial = Trial::open(&root.0, Some(day())).unwrap();
            trial.credentials = Box::new(store());
            trial.resume_session().unwrap();
            assert!(!trial.signed_out(), "{}", trial.message);
            trial.rest_logout().unwrap();
            assert_eq!(
                crate::credentials::CredentialStore::load(&store(), USER).unwrap(),
                None
            );
        }
        Err(_) => {
            assert!(trial.not_remembered);
            assert!(
                trial.message.contains("will not be remembered"),
                "{}",
                trial.message
            );
            assert!(!root.0.join("signed-in-account.txt").exists());
            trial.rest_logout().unwrap();
            assert!(trial.signed_out());
        }
    }
}
