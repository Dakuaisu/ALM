#[path = "common/scratch_retry.rs"]
mod scratch_retry;

use std::io::{self, ErrorKind};
use std::path::PathBuf;

#[test]
fn creates_distinct_real_directories() {
    let base = std::env::temp_dir();
    let first = scratch_retry::create(&base, "alm-scratch-retry-test-").unwrap();
    let second = scratch_retry::create(&base, "alm-scratch-retry-test-").unwrap();
    assert_ne!(first, second);
    std::fs::remove_dir(first).unwrap();
    std::fs::remove_dir(second).unwrap();
}

#[test]
fn retries_collisions_with_fresh_names() {
    let mut attempts = Vec::new();
    let result = scratch_retry::create_with(
        |n| PathBuf::from(n.to_string()),
        |path| {
            attempts.push(path.to_owned());
            if attempts.len() == 1 {
                Err(io::Error::from(ErrorKind::AlreadyExists))
            } else {
                Ok(())
            }
        },
        false,
    )
    .unwrap();
    assert_eq!(attempts, [PathBuf::from("0"), PathBuf::from("1")]);
    assert_eq!(result, PathBuf::from("1"));
}

#[test]
fn windows_permission_denied_retries_but_other_platforms_fail_promptly() {
    for (windows, expected) in [(true, scratch_retry::MAX_ATTEMPTS), (false, 1)] {
        let mut calls = 0;
        let error = scratch_retry::create_with(
            |n| PathBuf::from(n.to_string()),
            |_| {
                calls += 1;
                Err(io::Error::from(ErrorKind::PermissionDenied))
            },
            windows,
        )
        .unwrap_err();
        assert_eq!(calls, expected);
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    }
}

#[test]
fn windows_permission_denied_then_success_uses_a_fresh_name() {
    let mut attempts = Vec::new();
    let path = scratch_retry::create_with(
        |n| PathBuf::from(n.to_string()),
        |path| {
            attempts.push(path.to_owned());
            if attempts.len() == 1 {
                Err(io::Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(())
            }
        },
        true,
    )
    .unwrap();
    assert_eq!(attempts, [PathBuf::from("0"), PathBuf::from("1")]);
    assert_eq!(path, PathBuf::from("1"));
}

#[test]
fn exhaustion_is_bounded_and_unrelated_errors_fail_immediately() {
    for (kind, expected) in [
        (ErrorKind::AlreadyExists, scratch_retry::MAX_ATTEMPTS),
        (ErrorKind::NotFound, 1),
    ] {
        let mut calls = 0;
        let error = scratch_retry::create_with(
            |n| PathBuf::from(n.to_string()),
            |_| {
                calls += 1;
                Err(io::Error::from(kind))
            },
            true,
        )
        .unwrap_err();
        assert_eq!(calls, expected);
        assert_eq!(error.kind(), kind);
    }
}
