use iso8601_timestamp::Timestamp;
use time::UtcOffset;

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

#[test]
fn test_parse_iso8601_variations() {
    let fixtures = [
        "2021-10-17T02:03:01+00:00",
        "2021-10-17t02:03:01+10:00",
        "2021-10-17",
        "20211017",
        "2021-10-17 02:03:01+00:00",
        "2021-10-17T02:03+00:00",
        "2021-10-17t02:03+10:00",
        "2021-10-17 02:03+00:00",
        "-2021-10-17T02:03+00:00",
        "-2021-10-17t02:03+10:00",
        "-2021-10-17 02:03+00:00",
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
        "2013-10-07 08:23:19.120Z",
        "2013-10-07T08:23:19.120Z",
        "2013-10-07 08:23:19,120Z",
        "2013-10-07T08:23:19,120Z",
        "2013-10-07T08:23:19.120",
        "2013-10-07 08:23:19,120",
        "2013-10-07T08:23:19",
        "2013-10-07 04:23:19.120-04:00",
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

    let unix_ms_a = now_ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds();
    let unix_ms_b = now_ot.unix_timestamp_nanos() / 1_000_000;

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

#[test]
fn test_invalid() {
    let parsed = Timestamp::parse("-868686868");

    assert!(parsed.is_none());
}

#[test]
fn test_offset() {
    let ts: time::PrimitiveDateTime = time::macros::datetime!(2014-4-12 4:00 PM);
    let o = time::UtcOffset::from_hms(-4, 30, 0).unwrap();

    let ts = Timestamp::from(ts);

    let formatted = ts.format_with_offset(o);

    assert_eq!("2014-04-12T16:00:00.000-04:30", &*formatted);
}

#[rustfmt::skip]
#[test]
fn test_all_formats() {
    use generic_array::typenum as t;

    let ts = Timestamp::from(time::macros::datetime!(2014-4-12 4:00 PM));
    let mut ts2 = ts;

    macro_rules! test_cfg {
        ($f:ty, $o:ty, $p:ty, $offset:expr, $expected:expr) => {
            let res = ts.format_raw::<$f, $o, $p>($offset);
            assert_eq!(res, $expected);
            assert_eq!(Timestamp::parse(res.as_ref()), Some(ts2));
        }
    }

    test_cfg!(t::True, t::False, t::U0, UtcOffset::UTC, "2014-04-12T16:00:00Z");
    test_cfg!(t::True, t::False, t::U1, UtcOffset::UTC, "2014-04-12T16:00:00.0Z");
    test_cfg!(t::True, t::False, t::U2, UtcOffset::UTC, "2014-04-12T16:00:00.00Z");
    test_cfg!(t::True, t::False, t::U3, UtcOffset::UTC, "2014-04-12T16:00:00.000Z");
    test_cfg!(t::True, t::False, t::U4, UtcOffset::UTC, "2014-04-12T16:00:00.0000Z");
    test_cfg!(t::True, t::False, t::U5, UtcOffset::UTC, "2014-04-12T16:00:00.00000Z");
    test_cfg!(t::True, t::False, t::U6, UtcOffset::UTC, "2014-04-12T16:00:00.000000Z");
    test_cfg!(t::True, t::False, t::U7, UtcOffset::UTC, "2014-04-12T16:00:00.0000000Z");
    test_cfg!(t::True, t::False, t::U8, UtcOffset::UTC, "2014-04-12T16:00:00.00000000Z");
    test_cfg!(t::True, t::False, t::U9, UtcOffset::UTC, "2014-04-12T16:00:00.000000000Z");

    test_cfg!(t::False, t::False, t::U0, UtcOffset::UTC, "20140412T160000Z");
    test_cfg!(t::False, t::False, t::U1, UtcOffset::UTC, "20140412T160000.0Z");
    test_cfg!(t::False, t::False, t::U2, UtcOffset::UTC, "20140412T160000.00Z");
    test_cfg!(t::False, t::False, t::U3, UtcOffset::UTC, "20140412T160000.000Z");
    test_cfg!(t::False, t::False, t::U4, UtcOffset::UTC, "20140412T160000.0000Z");
    test_cfg!(t::False, t::False, t::U5, UtcOffset::UTC, "20140412T160000.00000Z");
    test_cfg!(t::False, t::False, t::U6, UtcOffset::UTC, "20140412T160000.000000Z");
    test_cfg!(t::False, t::False, t::U7, UtcOffset::UTC, "20140412T160000.0000000Z");
    test_cfg!(t::False, t::False, t::U8, UtcOffset::UTC, "20140412T160000.00000000Z");
    test_cfg!(t::False, t::False, t::U9, UtcOffset::UTC, "20140412T160000.000000000Z");

    let offset = UtcOffset::from_hms(15, 30, 0).unwrap();
    ts2 = Timestamp::from(ts.assume_offset(-offset));

    test_cfg!(t::True, t::True, t::U0, offset, "2014-04-12T16:00:00+15:30");
    test_cfg!(t::True, t::True, t::U1, offset, "2014-04-12T16:00:00.0+15:30");
    test_cfg!(t::True, t::True, t::U2, offset, "2014-04-12T16:00:00.00+15:30");
    test_cfg!(t::True, t::True, t::U3, offset, "2014-04-12T16:00:00.000+15:30");
    test_cfg!(t::True, t::True, t::U4, offset, "2014-04-12T16:00:00.0000+15:30");
    test_cfg!(t::True, t::True, t::U5, offset, "2014-04-12T16:00:00.00000+15:30");
    test_cfg!(t::True, t::True, t::U6, offset, "2014-04-12T16:00:00.000000+15:30");
    test_cfg!(t::True, t::True, t::U7, offset, "2014-04-12T16:00:00.0000000+15:30");
    test_cfg!(t::True, t::True, t::U8, offset, "2014-04-12T16:00:00.00000000+15:30");
    test_cfg!(t::True, t::True, t::U9, offset, "2014-04-12T16:00:00.000000000+15:30");

    test_cfg!(t::False, t::True, t::U0, offset, "20140412T160000+15:30");
    test_cfg!(t::False, t::True, t::U1, offset, "20140412T160000.0+15:30");
    test_cfg!(t::False, t::True, t::U2, offset, "20140412T160000.00+15:30");
    test_cfg!(t::False, t::True, t::U3, offset, "20140412T160000.000+15:30");
    test_cfg!(t::False, t::True, t::U4, offset, "20140412T160000.0000+15:30");
    test_cfg!(t::False, t::True, t::U5, offset, "20140412T160000.00000+15:30");
    test_cfg!(t::False, t::True, t::U6, offset, "20140412T160000.000000+15:30");
    test_cfg!(t::False, t::True, t::U7, offset, "20140412T160000.0000000+15:30");
    test_cfg!(t::False, t::True, t::U8, offset, "20140412T160000.00000000+15:30");
    test_cfg!(t::False, t::True, t::U9, offset, "20140412T160000.000000000+15:30");
}
