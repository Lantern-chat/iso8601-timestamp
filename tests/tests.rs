use iso8061_timestamp::Timestamp;

#[test]
fn test_format_iso8061() {
    let now = Timestamp::now_utc();

    let formatted = now.format();

    println!("{}", formatted);
}

#[test]
fn test_format_iso8061_full() {
    let now = Timestamp::now_utc();

    let formatted = now.format();

    println!("{}", formatted);
}

#[test]
fn test_parse_iso8061_reflex() {
    let now = Timestamp::now_utc();

    let formatted = now.format();

    println!("Formatted: {}", formatted);

    let parsed = Timestamp::parse(&formatted).unwrap();

    assert_eq!(formatted, parsed.format());
}

#[test]
fn test_parse_iso8061_variations() {
    let fixtures = [
        "2021-10-17T02:03:01+00:00",
        "2021-10-17t02:03:01+10:00",
        "2021-10-17t02:03+00:00", // without seconds
        "2021-10-17t02:03:01.111+00:00",
        "2021-10-17T02:03:01-00:00",
        "2021-10-17T02:03:01−04:00", // UNICODE MINUS SIGN in offset
        "2021-10-17T02:03:01Z",
        "20211017T020301Z",
        "20211017t020301z",
        "20211017T0203z", // without seconds
        "20211017T020301.123Z",
        "20211017T020301.123+00:00",
        "20211017T020301.123uTc",
    ];

    for fixture in fixtures {
        let parsed = Timestamp::parse(fixture);

        assert!(parsed.is_some(), "Failed to parse: {}", fixture);

        println!("{:?}", parsed.unwrap());
    }
}

#[test]
fn test_unix_timestamp_ms() {
    let now_ts = Timestamp::now_utc();
    let now_ot = now_ts.assume_offset(time::UtcOffset::UTC);

    let unix_ms_a = now_ts.to_unix_timestamp_ms();
    let unix_ms_b = (now_ot.unix_timestamp_nanos() / 1_000_000) as i64;

    assert_eq!(unix_ms_a, unix_ms_b);
}

#[test]
fn test_parse_nanoseconds() {
    let parsed = Timestamp::parse("2021-11-19T04:12:54.000123Z").unwrap();

    let time = time::Time::from_hms_nano(4, 12, 54, 123000).unwrap();
    let date = time::Date::from_calendar_date(2021, time::Month::November, 19).unwrap();

    let expected = Timestamp::from(time::PrimitiveDateTime::new(date, time));

    assert_eq!(parsed, expected);
}
