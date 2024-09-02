use iso8601_timestamp::Timestamp;
use time::UtcOffset;

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
        "+2013-10-07T08:23:19",
        "−2013-10-07T08:23:19",
        "2013-10-07 04:23:19.120-04:00",
    ];

    for fixture in fixtures {
        let parsed = Timestamp::parse(fixture);

        assert!(parsed.is_some(), "Failed to parse: {}", fixture);

        println!("{:?}", parsed.unwrap());
    }
}

#[test]
fn test_parse_negative() {
    let ts = iso8601_timestamp::datetime!(-0004-12-16 10:00 AM);

    let tsf = ts.format();

    assert_eq!(tsf, "-0004-12-16T10:00:00.000Z");
    assert_eq!(Timestamp::parse("−0004-12-16T10:00:00.000Z"), Some(ts));
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

    let ts = Timestamp::parse("2011-06-17T18:30+04:00").unwrap();

    assert_eq!(ts, Timestamp::parse("2011-06-17T14:30:00.000Z").unwrap());
    assert_eq!(ts, Timestamp::parse("+2011-06-17T14:30:00.000Z").unwrap());
    assert_eq!(
        ts,
        Timestamp::from(time::macros::datetime!(2011-06-17 18:30+04:00))
    );
    assert_eq!(
        Timestamp::parse("2011-06-17T18:30-04:00").unwrap(),
        Timestamp::from(time::macros::datetime!(2011-06-17 18:30-04:00))
    );
    assert_eq!(
        Timestamp::parse("2000-01-01T00:00:00+11:00").unwrap(),
        Timestamp::from(time::macros::datetime!(2000-01-01 00:00:00+11:00))
    );
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
    ts2 = Timestamp::from(ts.assume_offset(offset));

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
