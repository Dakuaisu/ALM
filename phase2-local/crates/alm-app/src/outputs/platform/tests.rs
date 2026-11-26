use std::sync::Mutex;

use tauri::ipc::{InvokeResponseBody, IpcResponse};

use super::{ShareFile, ShareOutput, bill_image_share};
use crate::{
    Trial,
    billing_tests::{day, posted},
    ipc::{IpcDraft, State},
    outputs::export::export_file,
    screen_tests::Root,
};

fn draft() -> IpcDraft {
    InvokeResponseBody::Json(r#""1""#.into())
        .deserialize()
        .unwrap()
}

fn owner(root: &Root) -> Mutex<Trial> {
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    posted(&mut trial.authority);
    trial.refresh().unwrap();
    Mutex::new(trial)
}

#[test]
fn image_command_returns_owner_generated_png_bytes_without_saving() {
    let root = Root::new("share-owner-image");
    let state = owner(&root);
    let output = bill_image_share(
        State::for_test(&state, "bill_image_share").unwrap(),
        draft(),
        Some(true),
    )
    .unwrap();
    #[derive(serde::Deserialize)]
    struct ReceivedFile {
        name: String,
        mime: String,
        bytes: Vec<u8>,
    }
    let file: ReceivedFile = output.body().unwrap().deserialize().unwrap();
    assert_eq!(
        (file.name.as_str(), file.mime.as_str()),
        ("Bill-1.png", "image/png")
    );
    assert_eq!(&file.bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(!state.lock().unwrap().outputs.join(file.name).exists());
}

#[test]
fn export_command_returns_each_generated_file_with_its_mime() {
    let root = Root::new("share-owner-export");
    let state = owner(&root);
    for (kind, suffix, expected_mime) in [
        ("csv", ".csv", "text/csv"),
        ("xls", ".xls", "application/vnd.ms-excel"),
    ] {
        let output = export_file(
            State::for_test(&state, "export_file").unwrap(),
            kind.into(),
            Some(true),
        )
        .unwrap();
        let ShareOutput::File(ShareFile { name, mime, bytes }) = output else {
            panic!("expected generated file")
        };
        assert!(name.ends_with(suffix));
        assert_eq!(mime, expected_mime);
        assert!(!bytes.is_empty());
        assert!(!state.lock().unwrap().outputs.join(name).exists());
    }
}

#[test]
fn generated_file_commands_deny_a_manager_even_after_owner_access() {
    let root = Root::new("share-manager");
    let state = owner(&root);
    assert!(
        bill_image_share(
            State::for_test(&state, "bill_image_share").unwrap(),
            draft(),
            Some(true),
        )
        .is_ok()
    );
    state.lock().unwrap().set_owner(false).unwrap();
    for mode in [Some(true), Some(false), None] {
        assert!(
            bill_image_share(
                State::for_test(&state, "bill_image_share").unwrap(),
                draft(),
                mode,
            )
            .is_err()
        );
        assert!(
            export_file(
                State::for_test(&state, "export_file").unwrap(),
                "csv".into(),
                mode,
            )
            .is_err()
        );
    }
}
