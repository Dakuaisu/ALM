use alm_core::confirmation::OpeningBalance;
use alm_core::{Money, RupeesError};

#[test]
fn signed_rupees_accept_one_leading_sign_and_the_unsigned_grammar() {
    let ok = |text: &str| Money::parse_signed_rupees(text).map(Money::minor_units);
    assert_eq!(ok("-250.50"), Ok(-25_050));
    assert_eq!(ok(" +5 "), Ok(500));
    assert_eq!(ok("-.5"), Ok(-50));
    assert_eq!(ok("12"), Ok(1_200));
    assert_eq!(ok("-0"), Ok(0));
    for bad in [
        "", "-", "--5", "+-5", "- 5", "5-", "-5.001", "1e3", "-1,000",
    ] {
        assert_eq!(ok(bad), Err(RupeesError::Invalid), "{bad:?}");
    }
    assert_eq!(ok("-92233720368547758.08"), Err(RupeesError::Overflow));
    assert_eq!(Money::parse_rupees("-5"), Err(RupeesError::Invalid));
}

#[test]
fn opening_amount_blank_is_none_and_negative_is_an_advance() {
    assert_eq!(OpeningBalance::parse_amount("  "), Ok(None));
    assert_eq!(
        OpeningBalance::parse_amount("-1200"),
        Ok(Some(Money::from_minor_units(-120_000)))
    );
    assert_eq!(
        OpeningBalance::parse_amount("abc"),
        Err(RupeesError::Invalid)
    );
}
