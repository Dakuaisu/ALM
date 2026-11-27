use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const MAX_ATTEMPTS: usize = 1024;

pub(crate) fn create_with(
    mut candidate: impl FnMut(usize) -> PathBuf,
    mut create: impl FnMut(&Path) -> io::Result<()>,
    windows: bool,
) -> io::Result<PathBuf> {
    for attempt in 0..MAX_ATTEMPTS {
        let path = candidate(attempt);
        match create(&path) {
            Ok(()) => return Ok(path),
            Err(error)
                if (error.kind() == ErrorKind::AlreadyExists
                    || (windows && error.kind() == ErrorKind::PermissionDenied))
                    && attempt + 1 < MAX_ATTEMPTS => {}
            Err(error) => return Err(error),
        }
    }
    unreachable!("scratch retry limit is nonzero")
}

pub(crate) fn create(base: &Path, prefix: &str) -> io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    create_with(
        |_| {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            base.join(format!("{prefix}{}-{sequence}", std::process::id()))
        },
        |path| std::fs::create_dir(path),
        cfg!(windows),
    )
}
