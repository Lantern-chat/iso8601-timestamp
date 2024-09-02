#![cfg(feature = "std")]

use std::time::SystemTime;

use iso8601_timestamp::Timestamp;

#[test]
fn test_unix_timestamp_ms() {
    let now_ts = Timestamp::now_utc();
    let now_ot = now_ts.assume_offset(time::UtcOffset::UTC);

    let unix_ms_a = now_ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds();
    let unix_ms_b = now_ot.unix_timestamp_nanos() / 1_000_000;

    assert_eq!(unix_ms_a, unix_ms_b);
}

#[test]
fn test_roundtrip_std() {
    let now = SystemTime::now();
    let ts = Timestamp::from(now);
    let n2 = SystemTime::from(ts);

    assert_eq!(now, n2);
}

#[test]
fn test_format_iso8601() {
    let now = Timestamp::now_utc();

    let formatted = now.format();

    println!("{formatted}");

    assert_eq!(Timestamp::UNIX_EPOCH.format(), "1970-01-01T00:00:00.000Z");
}

#[test]
fn test_format_iso8601_full() {
    let now = Timestamp::now_utc();

    println!("{}", now.format());
    println!("{}", now.format_nanoseconds());
    println!("{}", now.format_microseconds());
}

#[test]
fn test_parse_iso8601_reflex() {
    let now = Timestamp::now_utc();

    let formatted = now.format();

    println!("Formatted: {formatted}");

    let parsed = Timestamp::parse(&formatted).unwrap();

    assert_eq!(formatted, parsed.format());
}
