use std::path::Path;
use std::process::Command;

pub(super) fn render_if_requested(dir: &Path) -> Option<Vec<u8>> {
    let enabled = std::env::var_os("ALM_REPLAY_QUICKLOOK_PNG")?;
    assert_eq!(enabled, "1", "set ALM_REPLAY_QUICKLOOK_PNG to 1 to opt in");
    let svg = dir.join("synthetic-cancelled-estimate.svg");
    let output = dir.join("synthetic-cancelled-estimate.svg.png");
    match std::fs::remove_file(&output) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("remove stale synthetic PNG: {error}"),
    }
    let result = Command::new("qlmanage")
        .args(["-t", "-s", "1440", "-o"])
        .arg(dir)
        .arg(svg)
        .env("TMPDIR", dir)
        .output()
        .expect("opted-in Quick Look renderer must be installed");
    assert!(
        result.status.success(),
        "Quick Look rasterization failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new("sips")
        .args([
            "--cropOffset",
            "1",
            "0",
            "--cropToHeightWidth",
            "940",
            "1440",
        ])
        .arg(&output)
        .output()
        .expect("opted-in Quick Look crop requires sips");
    assert!(
        result.status.success(),
        "Quick Look crop failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let png = std::fs::read(output).expect("Quick Look must write the PNG thumbnail");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(&png[16..24], &[0, 0, 5, 160, 0, 0, 3, 172]);
    Some(png)
}
