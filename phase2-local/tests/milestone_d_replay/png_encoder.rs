use std::cell::RefCell;
use std::rc::Rc;

pub(super) const MIME: &str = "image/png";
pub(super) const PRIMARY: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0x7a, 0x5e, 0xab, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];
pub(super) const FALLBACK: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5, 0x1c, 0x0c,
    0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64, 0xf8, 0x0f, 0x00,
    0x01, 0x05, 0x01, 0x01, 0x27, 0x18, 0xe3, 0x66, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];
const PRIMARY_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGNgAAIAAAUAAXpeqz8AAAAASUVORK5CYII=";
pub(super) const FALLBACK_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

#[derive(Clone, Copy, Debug)]
pub(super) enum Mode {
    Blob,
    Missing,
    Null,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Event {
    DrawImage,
    ToBlob(&'static str),
    ToDataUrl(&'static str),
    Decode,
    Callback(Outcome),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    SameBlob,
    DecodedFixture,
    Null,
}

#[derive(Debug)]
pub(super) struct Blob {
    mime: &'static str,
    pub(super) bytes: Vec<u8>,
}

pub(super) struct Encoder {
    pub(super) mode: Mode,
    pub(super) events: RefCell<Vec<Event>>,
}

impl Encoder {
    pub(super) fn new(mode: Mode) -> Self {
        Self {
            mode,
            events: RefCell::default(),
        }
    }

    // Source-shaped model of htmlToPNG after image load, not an HTML runtime.
    fn after_load(
        &self,
        to_blob: Option<Option<Rc<Blob>>>,
        callback: impl FnOnce(Option<Rc<Blob>>),
    ) {
        self.events.borrow_mut().push(Event::DrawImage);
        let data_url = || {
            self.events.borrow_mut().push(Event::ToDataUrl(MIME));
            self.events.borrow_mut().push(Event::Decode);
            Some(Rc::new(decode(PRIMARY_URL)))
        };
        match to_blob {
            Some(result) => {
                self.events.borrow_mut().push(Event::ToBlob(MIME));
                callback(result);
            }
            None => callback(data_url()),
        }
    }

    pub(super) fn render(&self) -> Option<Rc<Blob>> {
        let supplied = Rc::new(Blob {
            mime: MIME,
            bytes: PRIMARY.to_vec(),
        });
        let to_blob = match self.mode {
            Mode::Blob => Some(Some(Rc::clone(&supplied))),
            Mode::Missing => None,
            Mode::Null => Some(None),
        };
        let mut delivered = None;
        self.after_load(to_blob, |result| {
            let outcome = match &result {
                Some(blob) => {
                    assert_eq!(blob.mime, MIME);
                    assert_eq!(blob.bytes, PRIMARY);
                    if Rc::ptr_eq(blob, &supplied) {
                        Outcome::SameBlob
                    } else {
                        Outcome::DecodedFixture
                    }
                }
                None => Outcome::Null,
            };
            self.events.borrow_mut().push(Event::Callback(outcome));
            delivered = result;
        });
        delivered
    }

    pub(super) fn assert_trace(&self, caller: &str) {
        use Event::*;
        let expected: &[Event] = match self.mode {
            Mode::Blob => &[DrawImage, ToBlob("image/png"), Callback(Outcome::SameBlob)],
            Mode::Missing => &[
                DrawImage,
                ToDataUrl("image/png"),
                Decode,
                Callback(Outcome::DecodedFixture),
            ],
            Mode::Null => &[DrawImage, ToBlob("image/png"), Callback(Outcome::Null)],
        };
        let events = self.events.borrow();
        println!("{caller} {:?} helper: {events:?}", self.mode);
        assert_eq!(events.as_slice(), expected);
    }
}

// Scoped to known, padded fixture URLs; not a general-purpose data URL parser.
pub(super) fn decode(url: &str) -> Blob {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let encoded = url.strip_prefix("data:image/png;base64,").unwrap();
    let mut bytes = Vec::new();
    let mut buffer = 0_u16;
    let mut bits = 0_u8;
    for byte in encoded.trim_end_matches('=').bytes() {
        let sextet = ALPHABET.iter().position(|entry| *entry == byte).unwrap();
        buffer = (buffer << 6) | u16::try_from(sextet).unwrap();
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits).to_le_bytes()[0]);
        }
    }
    Blob { mime: MIME, bytes }
}

#[test]
fn null_callback_does_not_retry() {
    let encoder = Encoder::new(Mode::Null);
    let result = encoder.render();
    encoder.assert_trace("null regression control");
    assert!(result.is_none());
}
