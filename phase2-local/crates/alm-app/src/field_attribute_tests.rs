//! Every URL, email, password and code field is typed verbatim: no auto-capitalising,
//! correcting or spell-checking, and the right type and keyboard.
use crate::Trial;
use crate::billing_tests::day;
use crate::screen_tests::Root;

fn inputs(html: &str) -> Vec<&str> {
    html.match_indices("<input ")
        .map(|(at, _)| &html[at..at + html[at..].find('>').unwrap()])
        .collect()
}

fn attr<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = input.find(&key)? + key.len();
    Some(&input[start..start + input[start..].find('"')?])
}

/// `(name or placeholder start, type, inputmode)` that each listed field must carry.
fn check(html: &str, expected: &[(&str, &str, Option<&str>)]) {
    for (field, kind, mode) in expected {
        let input = inputs(html)
            .into_iter()
            .find(|i| {
                attr(i, "name") == Some(*field)
                    || attr(i, "placeholder").is_some_and(|p| p.starts_with(field))
            })
            .unwrap_or_else(|| panic!("no field {field}"));
        assert_eq!(attr(input, "autocapitalize"), Some("off"), "{input}");
        assert_eq!(attr(input, "autocorrect"), Some("off"), "{input}");
        assert_eq!(attr(input, "spellcheck"), Some("false"), "{input}");
        assert_eq!(attr(input, "type"), Some(*kind), "{input}");
        assert_eq!(attr(input, "inputmode"), *mode, "{input}");
    }
}

#[test]
fn url_email_password_and_code_fields_are_typed_verbatim() {
    let root = Root::new("field-attributes");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let settings = trial.render("settings").unwrap();
    check(
        &settings,
        &[
            ("url", "url", Some("url")),
            ("auth", "url", Some("url")),
            ("apikey", "text", None),
            ("gstin", "text", None),
            ("fssai", "text", Some("numeric")),
            ("https://xxxx.supabase.co", "url", Some("url")),
            ("anon public key", "text", None),
            ("secret token", "password", None),
            ("https://script.google.com", "url", Some("url")),
        ],
    );
    let login = crate::dev_authority::login_screen(
        &crate::dev_authority::RestConfig {
            url: "http://127.0.0.1:3000".into(),
            auth: "http://127.0.0.1:9999".into(),
            apikey: String::new(),
        },
        "",
    );
    check(
        &login,
        &[
            ("email", "email", Some("email")),
            ("password", "password", None),
        ],
    );
    trial.gst_form = Some(
        trial
            .posted
            .new_gst_bill_fields(alm_core::Role::Owner, day())
            .unwrap(),
    );
    let gst = trial.render("gstsales").unwrap();
    check(
        &gst,
        &[
            ("gstin", "text", None),
            ("cust_email", "email", Some("email")),
            ("cust_state_code", "text", Some("numeric")),
            ("pos_code", "text", Some("numeric")),
            ("eway_number", "text", Some("numeric")),
            ("vehicle_no", "text", None),
            ("lr_no", "text", None),
            ("hsn_0", "text", Some("numeric")),
        ],
    );
}
