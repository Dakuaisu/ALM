//! The HTML's number and text helpers: `esc`, `inr`, `fmtN`, `numToWords`, `rupeesWords`.
use alm_core::Money;

/// `esc`: the HTML escapes `& < > "` only.
pub fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn indian_digits(mut whole: u64) -> String {
    let mut groups = vec![format!("{:03}", whole % 1000)];
    whole /= 1000;
    while whole > 0 {
        groups.push(format!("{:02}", whole % 100));
        whole /= 100;
    }
    let mut text = groups.into_iter().rev().collect::<Vec<_>>().join(",");
    while text.len() > 1 && (text.starts_with('0') || text.starts_with(',')) {
        text.remove(0);
    }
    text
}

/// `inr`: en-IN grouping with exactly two decimals.
pub fn inr(money: Money) -> String {
    let minor = money.minor_units();
    let sign = if minor < 0 { "-" } else { "" };
    let magnitude = minor.unsigned_abs();
    format!(
        "{sign}{}.{:02}",
        indian_digits(magnitude / 100),
        magnitude % 100
    )
}

/// `fmtN` for whole numbers.
pub(super) fn fmt_n(value: u64) -> String {
    indian_digits(value)
}

/// A rate in basis points as the HTML prints a JS number: `20`, `2.5`, `9`.
pub(super) fn percent(basis_points: u32) -> String {
    let text = format!("{}.{:02}", basis_points / 100, basis_points % 100);
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// `numToWords` (L4843-4862).
fn words(mut n: u64) -> String {
    const A: [&str; 20] = [
        "",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
    ];
    const T: [&str; 10] = [
        "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
    ];
    let two = |x: u64| -> String {
        if x < 20 {
            A[x as usize].to_owned()
        } else if x.is_multiple_of(10) {
            T[(x / 10) as usize].to_owned()
        } else {
            format!("{} {}", T[(x / 10) as usize], A[(x % 10) as usize])
        }
    };
    let three = |x: u64| -> String {
        let hundreds = if x >= 100 {
            format!(
                "{} Hundred{}",
                A[(x / 100) as usize],
                if !x.is_multiple_of(100) { " " } else { "" }
            )
        } else {
            String::new()
        };
        let rest = if !x.is_multiple_of(100) {
            two(x % 100)
        } else {
            String::new()
        };
        hundreds + &rest
    };
    if n == 0 {
        return "Zero".into();
    }
    let mut text = String::new();
    let crore = n / 10_000_000;
    n %= 10_000_000;
    let lakh = n / 100_000;
    n %= 100_000;
    let thousand = n / 1000;
    n %= 1000;
    if crore > 0 {
        text += &format!("{} Crore ", three(crore));
    }
    if lakh > 0 {
        text += &format!("{} Lakh ", two(lakh));
    }
    if thousand > 0 {
        text += &format!("{} Thousand ", two(thousand));
    }
    if n > 0 {
        text += &three(n);
    }
    text.trim().to_owned()
}

/// `rupeesWords` (L4864-4870).
pub(super) fn rupees_words(money: Money) -> String {
    let minor = money.minor_units().unsigned_abs();
    let mut text = format!("INR {}", words(minor / 100));
    if !minor.is_multiple_of(100) {
        text += &format!(" and {} paise", words(minor % 100));
    }
    text + " Only"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formats_follow_the_html_helpers() {
        assert_eq!(inr(Money::from_minor_units(12_345_678)), "1,23,456.78");
        assert_eq!(inr(Money::from_minor_units(5)), "0.05");
        assert_eq!(percent(2_000), "20");
        assert_eq!(percent(250), "2.5");
        assert_eq!(
            rupees_words(Money::from_minor_units(52_500)),
            "INR Five Hundred Twenty Five Only"
        );
        assert_eq!(
            rupees_words(Money::from_minor_units(12_525_050)),
            "INR One Lakh Twenty Five Thousand Two Hundred Fifty and Fifty paise Only"
        );
        assert_eq!(esc("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }
}
