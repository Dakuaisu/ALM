// Offline source-shaped replay of L09-06 and L14-06; it does not execute HTML.
#[path = "contact_reminder_replay/visibility.rs"]
mod visibility;
use visibility::{Row, Surface, click_customer, exercise_surfaces, visible};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CustomerId(u8);

struct Customer {
    id: CustomerId,
    name: &'static str,
    mobile: &'static str,
    opening_paise: i64,
}

struct Sale {
    customer: CustomerId,
    total_paise: i64,
    paid_paise: i64,
    cancelled: bool,
}

struct Payment {
    customer: CustomerId,
    amount_paise: i64,
}

struct Records {
    customers: Vec<Customer>,
    sales: Vec<Sale>,
    payments: Vec<Payment>,
}

impl Records {
    fn balance(&self, id: CustomerId) -> i64 {
        let opening = self
            .customers
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .opening_paise;
        let bills: i64 = self
            .sales
            .iter()
            .filter(|s| s.customer == id && !s.cancelled)
            .map(|s| s.total_paise - s.paid_paise)
            .sum();
        let receipts: i64 = self
            .payments
            .iter()
            .filter(|p| p.customer == id)
            .map(|p| p.amount_paise)
            .sum();
        // Math.round, including negative ties, after summing all amounts.
        (opening + bills - receipts + 50).div_euclid(100)
    }
}

#[test]
fn ageing_visibility_uses_paise_boundary() {
    assert!(!visible(Surface::Ageing, "0000000001", 1, 49));
    assert!(!visible(Surface::Ageing, "0000000001", 1, 50));
    assert!(visible(Surface::Ageing, "0000000001", 1, 51));
}

fn clean_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    let start = digits.len().saturating_sub(10);
    let last = &digits[start..];
    if last.len() == 10 {
        last.into()
    } else {
        String::new()
    }
}

fn encode(text: &str) -> String {
    let mut result = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            result.push(char::from(byte));
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            result.push(u8::from_str_radix(&text[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result).unwrap()
}

fn format_indian(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    if value < 0 {
        out.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        let remaining = digits.len() - i;
        if i > 0 && remaining >= 3 && remaining % 2 == 1 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Capture {
    warnings: Vec<(String, &'static str)>,
    opens: Vec<(String, &'static str)>,
    attachments: Vec<String>,
}

impl Capture {
    fn supplier(&mut self, mobile: &str) -> bool {
        let visible = !mobile.is_empty();
        if visible {
            self.opens
                .push((format!("https://wa.me/91{}", clean_phone(mobile)), "_blank"));
        }
        visible
    }

    fn customer(&mut self, records: &Records, customer: &Customer) {
        let balance = records.balance(customer.id);
        let text = format!(
            "Dear {},\nThis is a gentle reminder from *Synthetic & Co*.\nYour pending balance is *₹{}*.\nKindly clear at your convenience. Thank you! 🙏",
            customer.name,
            format_indian(balance)
        );
        let phone = clean_phone(customer.mobile);
        if phone.is_empty() {
            self.warnings
                .push((format!("No valid mobile for {}", customer.name), "warn"));
        } else {
            self.opens.push((
                format!("https://wa.me/91{phone}?text={}", encode(&text)),
                "_blank",
            ));
        }
    }
}

#[test]
fn contact_reminder_replay() {
    // Given source-shaped synthetic records: current 100 + 500 - 80 + 250 - 30 = 740.
    // Invoice snapshot 420 and ageing row 250 are deliberately different from current.
    let mut records = Records {
        customers: [
            (1, "A & B", "+91 00000 00001", 10_000),
            (2, "A & B", "bad", 90_000),
            (3, "No mobile", "", 10_000),
            (4, "Advance", "0000000001", -10_000),
        ]
        .into_iter()
        .map(|(id, name, mobile, opening_paise)| Customer {
            id: CustomerId(id),
            name,
            mobile,
            opening_paise,
        })
        .collect(),
        sales: [
            (1, 50_000, 8_000, false),
            (1, 25_000, 0, false),
            (1, 999_900, 0, true),
            (2, 900_000, 0, false),
        ]
        .into_iter()
        .map(|(id, total_paise, paid_paise, cancelled)| Sale {
            customer: CustomerId(id),
            total_paise,
            paid_paise,
            cancelled,
        })
        .collect(),
        payments: vec![Payment {
            customer: CustomerId(1),
            amount_paise: 3_000,
        }],
    };
    // When supplier controls and customer handler are invoked, capture only inert requests.
    let mut supplier = Capture::default();
    let blank = supplier.supplier("");
    assert!(!blank);
    assert_eq!(supplier, Capture::default());
    let malformed = supplier.supplier("bad");
    let valid = supplier.supplier("+91 00000 00001");
    assert!(malformed && valid);
    assert_eq!(
        supplier.opens,
        [
            ("https://wa.me/91".into(), "_blank"),
            ("https://wa.me/910000000001".into(), "_blank")
        ]
    );
    assert!(supplier.warnings.is_empty() && supplier.attachments.is_empty());
    let expected = "Dear A & B,\nThis is a gentle reminder from *Synthetic & Co*.\nYour pending balance is *₹740*.\nKindly clear at your convenience. Thank you! 🙏";
    exercise_surfaces(&records, expected);
    // Then a receipt after row creation changes the click-time payload, not the row or snapshot.
    records.payments.push(Payment {
        customer: CustomerId(1),
        amount_paise: 12_500,
    });
    let mut updated = Capture::default();
    assert!(click_customer(
        &mut updated,
        &records,
        Row {
            surface: Surface::Ageing,
            index: 0,
            ageing_due_paise: 25_000
        }
    ));
    let updated_text = decode(updated.opens[0].0.split_once("?text=").unwrap().1);
    assert_eq!(updated_text, expected.replace("₹740", "₹615"));
    println!(
        "supplier visibility expected=[false, true, true] observed={:?}; requests={supplier:?}",
        [blank, malformed, valid]
    );
    println!(
        "customer expected/observed: current=740 updated=615 snapshot=420 ageing=250; observed={} / {}",
        expected, updated_text
    );
    println!("Offline Rust model only; frozen HTML, browser and actual handoff not executed.");
}
