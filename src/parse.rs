#![allow(clippy::never_loop)]

use time::{Date, Duration, Month, PrimitiveDateTime, Time};

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[cfg(any(test, not(feature = "verify")))]
#[inline(always)]
fn parse_2(s: &[u8]) -> u8 {
    // SAFETY: This function is only called with slices of length 2
    unsafe { assume!(s.len() == 2) };

    // NOTE: Despite doing the same as the loop below, this is a hair faster
    // (like a single clock cycle) due to instruction-level parallelism
    (s[0] & 0x0f) * 10 + (s[1] & 0x0f)
}

#[cfg(any(test, not(feature = "verify")))]
#[inline(always)]
fn parse_4(s: &[u8]) -> u16 {
    // SAFETY: This function is only called with slices of length 4
    unsafe { assume!(s.len() == 4) };

    let mut digits = u32::from_ne_bytes({
        let mut buf = [0; 4];
        buf.copy_from_slice(s);
        buf
    });

    // On LE: s[0] is at bits 0-7 (TENS of pair 0), s[1] at 8-15 (UNITS of pair 0),
    //        s[2] at 16-23 (TENS of pair 1), s[3] at 24-31 (UNITS of pair 1).
    //        => tens are in the 0x000f_000f nibbles; units in 0x0f00_0f00 nibbles.
    // On BE: s[0] is at bits 24-31 (TENS of pair 0), s[1] at 16-23 (UNITS of pair 0),
    //        s[2] at 8-15 (TENS of pair 1), s[3] at 0-7 (UNITS of pair 1).
    //        => tens are in the 0x0f00_0f00 nibbles; units in 0x000f_000f nibbles.
    #[cfg(target_endian = "little")]
    {
        digits = ((digits & 0x0f00_0f00) >> 8) + ((digits & 0x000f_000f) * 10);
        digits = ((digits & 0x00ff_00ff) >> 16) + ((digits & 0x0000_00ff) * 100);
    }
    #[cfg(target_endian = "big")]
    {
        digits = ((digits & 0x0f00_0f00) >> 8) * 10 + (digits & 0x000f_000f);
        digits = ((digits & 0x00ff_0000) >> 16) * 100 + (digits & 0x0000_00ff);
    }

    digits as u16
}

macro_rules! impl_fp {
    ($($t:ty),*) => {$(
        impl FastParse for $t {
            #[inline(always)]
            fn parse(s: &[u8]) -> Option<Self> {
                #[allow(unused_mut)]
                let mut overflow = false;
                let mut num: $t = 0;

                #[cfg(not(feature = "verify"))]
                match s.len() {
                    0 => return None,
                    2 => return Some(parse_2(s) as $t),
                    4 => return Some(parse_4(s) as $t),
                    _ => {
                        for byte in s {
                            num = num.wrapping_mul(10) + (byte & 0x0f) as $t;
                        }
                    }
                }

                #[cfg(feature = "verify")]
                for byte in s {
                    let digit = byte.wrapping_sub(b'0');
                    overflow |= digit > 9;
                    num = num.wrapping_mul(10) + digit as $t;
                }

                match overflow {
                    false => Some(num),
                    true => None,
                }
            }
        }
    )*};
}

impl_fp!(u8, u16, u32);

#[inline]
pub fn parse_iso8601(b: &[u8]) -> Option<PrimitiveDateTime> {
    let (mut offset, negate) = match b.first().copied() {
        Some(c @ (b'+' | b'-' | 0xe2)) => {
            let mut offset = 1;

            if unlikely!(c == 0xe2) {
                // check for UTF8 Unicode MINUS SIGN
                if unlikely!(b.get(offset..(offset + 2)) != Some(&[0x88u8, 0x92u8] as &[u8])) {
                    return None;
                }

                offset += 2;
            }

            (offset, (c != b'+') as i32)
        }
        Some(_) => (0, 0),
        None => return None,
    };

    macro_rules! parse {
        ($len:expr, $ty:ty $(, $eat_byte:expr)?) => {loop {
            if let Some(chunk) = b.get(offset..(offset + $len)) {
                if let Some(res) = <$ty as FastParse>::parse(chunk) {
                    offset += $len;

                    $(
                        // conditional increment is slightly faster than branchless
                        if let Some($eat_byte) = b.get(offset) {
                            offset += 1;
                        }
                    )?

                    break res;
                }
            }

            return None;
        }};
    }

    // NOTE: converting u16 to i16 is fine since it's less than 9999
    let mut year = parse!(4, u16, b'-') as i32; // YYYY-?

    // branchless conditional negation seems faster for i16
    // done immediately after parsing to avoid keeping the negate register
    year = (year ^ -negate) + negate;

    let month = parse!(2, u8, b'-'); // MM-?
    let day = parse!(2, u8); // DD

    // NOTE: Inlining this is cheaper than `Month::try_from(month).ok()?`
    let month = match month {
        1 => Month::January,
        2 => Month::February,
        3 => Month::March,
        4 => Month::April,
        5 => Month::May,
        6 => Month::June,
        7 => Month::July,
        8 => Month::August,
        9 => Month::September,
        10 => Month::October,
        11 => Month::November,
        12 => Month::December,
        _ => return None,
    };

    #[cfg(feature = "verify")]
    unsafe {
        assume!(-9999 <= year && year <= 9999);
    }

    let Ok(date) = Date::from_calendar_date(year, month, day) else {
        return None;
    };

    let mut date_time = PrimitiveDateTime::new(date, Time::MIDNIGHT);

    match b.get(offset) {
        Some(next) => {
            offset += 1; // T

            match next {
                b'T' | b't' | b' ' | b'_' => {}
                b'z' | b'Z' if offset == b.len() => return Some(date_time), // date-only with Z suffix
                _ => return None,
            }
        }

        // date-only, None means it's at the end of the string
        None => return Some(date_time),
    }

    let hour = parse!(2, u8, b':'); // HH:?
    let minute = parse!(2, u8, b':'); // mm:?

    let mut second = 0;
    let mut nanosecond = 0;
    let mut factor: u32 = 100_000_000; // up to 9 decimal places
    let mut frac_seconds = true;

    loop {
        if let Some(b'0'..=b'9') = b.get(offset) {
            second = parse!(2, u8);
            frac_seconds = false;
        }

        loop {
            if !matches!(b.get(offset), Some(b'.' | b',')) {
                break;
            }

            offset += 1;

            // NOTE: After 9 decimal places, this does nothing other than consume digits,
            // as factor will be zero, so nanosecond will not change
            while let Some(&c) = b.get(offset) {
                let d = c.wrapping_sub(b'0');

                if d > 9 {
                    break; // break on non-numeric input
                }

                nanosecond += d as u32 * factor;
                factor /= 10;
                offset += 1;
            }

            if unlikely!(frac_seconds) {
                let total_ns = nanosecond as u64 * 60;
                second += (total_ns / 1_000_000_000) as u8;
                nanosecond = (total_ns % 1_000_000_000) as u32;
            }

            break;
        }

        // if leap seconds, ignore the parsed value and set it to just before 60
        // doing it this way avoids duplicate code to consume the extra characters
        // NOTE: This will also "fix" malformed seconds input
        if unlikely!(second > 59) {
            // but don't neglect invalid input if necessary
            #[cfg(feature = "verify")]
            if unlikely!(second > 60) {
                return None;
            }

            second = 59;
            nanosecond = 999_999_999;
        }

        break;
    }

    // SAFETY: These values are verified to be within bounds
    unsafe {
        assume!(nanosecond <= 999_999_999);
        assume!(second <= 59);

        // if input is verified, it's impossible for these values to go over 2 digits
        #[cfg(feature = "verify")]
        {
            assume!(hour <= 99);
            assume!(minute <= 99);
        }
    }

    date_time = match Time::from_hms_nano(hour, minute, second, nanosecond) {
        Ok(time) => date_time.replace_time(time),
        _ => return None,
    };

    let tz = b.get(offset).copied();

    offset += 1;

    match tz {
        // Z
        Some(b'Z' | b'z') if likely!(offset == b.len()) => Some(date_time),

        // timezone, like +00:00
        Some(c @ (b'+' | b'-' | 0xe2)) => {
            if unlikely!(c == 0xe2) {
                // check for UTF8 Unicode MINUS SIGN
                if unlikely!(b.get(offset..(offset + 2)) != Some(&[0x88u8, 0x92u8] as &[u8])) {
                    return None;
                }
                offset += 2;
            }

            let tz_offset_hour = parse!(2, u8, b':') as i64;

            let mut tz_offset_minute = 0;
            if likely!(offset != b.len()) {
                tz_offset_minute = parse!(2, u8) as i64;
            }

            if unlikely!(offset != b.len()) {
                return None;
            }

            if tz_offset_hour == 0 && tz_offset_minute == 0 {
                return Some(date_time);
            }

            #[cfg(feature = "verify")]
            if unlikely!(tz_offset_hour > 23 || tz_offset_minute > 59) {
                return None;
            }

            let tz_offset = Duration::seconds(60 * 60 * tz_offset_hour + tz_offset_minute * 60);

            // these generate function calls regardless, so avoid
            // negating the offset and just chose which call to make
            let checked_op: fn(PrimitiveDateTime, Duration) -> Option<PrimitiveDateTime> = match c != b'+' {
                true => PrimitiveDateTime::checked_add as _,
                false => PrimitiveDateTime::checked_sub as _,
            };

            checked_op(date_time, tz_offset)
        }

        // Parse trailing "UTC", but it does nothing, same as Z
        Some(b'U' | b'u') => match b.get(offset..(offset + 2)) {
            None => None,
            Some(tc) => {
                // avoid multiple branches when this loop is unrolled
                let invalid = ((tc[0] | 0x20) != b't') | ((tc[1] | 0x20) != b'c');

                if unlikely!(invalid || (offset + 2) != b.len()) {
                    return None;
                }

                Some(date_time)
            }
        },
        None => Some(date_time),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let i = u32::parse(b"1234567890");

        assert_eq!(i, Some(1234567890));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_parse_int2() {
        for i in 0..=99 {
            let s = format!("{i:02}");
            let res = parse_2(s.as_bytes());
            assert_eq!(res, i);
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_parse_int4() {
        for i in 0..=9999 {
            let s = format!("{i:04}");
            let res = parse_4(s.as_bytes());
            assert_eq!(res, i);
        }
    }

    fn p(s: &str) -> Option<PrimitiveDateTime> {
        parse_iso8601(s.as_bytes())
    }

    fn dt(year: i32, month: u8, day: u8, hour: u8, min: u8, sec: u8, nano: u32) -> PrimitiveDateTime {
        use core::convert::TryFrom;
        let month = Month::try_from(month).unwrap();
        PrimitiveDateTime::new(
            Date::from_calendar_date(year, month, day).unwrap(),
            Time::from_hms_nano(hour, min, sec, nano).unwrap(),
        )
    }

    // --- Valid formats ---

    #[test]
    fn test_parse_full_z() {
        assert_eq!(p("2021-10-17T10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_milliseconds() {
        assert_eq!(
            p("2021-10-17T10:30:00.123Z"),
            Some(dt(2021, 10, 17, 10, 30, 0, 123_000_000))
        );
    }

    #[test]
    fn test_parse_nanoseconds() {
        assert_eq!(
            p("2021-10-17T10:30:00.123456789Z"),
            Some(dt(2021, 10, 17, 10, 30, 0, 123_456_789))
        );
    }

    #[test]
    fn test_parse_compact_datetime() {
        assert_eq!(p("20211017T103000Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_compact_datetime_with_sub_seconds() {
        assert_eq!(
            p("20211017T103000.456Z"),
            Some(dt(2021, 10, 17, 10, 30, 0, 456_000_000))
        );
    }

    #[test]
    fn test_parse_date_only() {
        assert_eq!(p("2021-10-17"), Some(dt(2021, 10, 17, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_date_only_compact() {
        assert_eq!(p("20211017"), Some(dt(2021, 10, 17, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_datetime_no_seconds() {
        assert_eq!(p("2021-10-17T10:30Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_lowercase_z() {
        assert_eq!(p("2021-10-17T10:30:00z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Separators ---

    #[test]
    fn test_parse_lowercase_t_separator() {
        assert_eq!(p("2021-10-17t10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_space_separator() {
        assert_eq!(p("2021-10-17 10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_underscore_separator() {
        assert_eq!(p("2021-10-17_10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Timezone offsets ---

    #[test]
    fn test_parse_positive_offset_applied() {
        // +05:00 means local is 5h ahead of UTC, so UTC = local - 5h
        assert_eq!(
            p("2021-10-17T15:30:00+05:00"),
            Some(dt(2021, 10, 17, 10, 30, 0, 0))
        );
    }

    #[test]
    fn test_parse_negative_offset_applied() {
        // -05:00 means local is 5h behind UTC, so UTC = local + 5h
        assert_eq!(
            p("2021-10-17T05:30:00-05:00"),
            Some(dt(2021, 10, 17, 10, 30, 0, 0))
        );
    }

    #[test]
    fn test_parse_offset_no_colon() {
        assert_eq!(
            p("2021-10-17T15:30:00+0500"),
            Some(dt(2021, 10, 17, 10, 30, 0, 0))
        );
    }

    #[test]
    fn test_parse_offset_hour_only() {
        assert_eq!(p("2021-10-17T15:30:00+05"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_offset_hour_only_negative() {
        assert_eq!(p("2021-10-17T05:30:00-05"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_offset_trailing_colon() {
        // "+05:" treated leniently as +05:00
        assert_eq!(p("2021-10-17T15:30:00+05:"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_offset_single_digit_minute_rejected() {
        assert_eq!(p("2021-10-17T10:30:00+05:3"), None);
    }

    #[test]
    fn test_parse_zero_offset_plus() {
        assert_eq!(
            p("2021-10-17T10:30:00+00:00"),
            Some(dt(2021, 10, 17, 10, 30, 0, 0))
        );
    }

    #[test]
    fn test_parse_zero_offset_minus() {
        // -00:00 should equal +00:00 (no adjustment)
        assert_eq!(
            p("2021-10-17T10:30:00-00:00"),
            Some(dt(2021, 10, 17, 10, 30, 0, 0))
        );
    }

    #[test]
    fn test_parse_utc_keyword_uppercase() {
        assert_eq!(p("2021-10-17T10:30:00UTC"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_utc_keyword_lowercase() {
        assert_eq!(p("2021-10-17T10:30:00utc"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Year sign prefix ---

    #[test]
    fn test_parse_explicit_positive_year() {
        assert_eq!(p("+2021-10-17T10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_negative_year() {
        assert_eq!(p("-0001-01-01T00:00:00Z"), Some(dt(-1, 1, 1, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_negative_zero_year() {
        // -0000 and +0000 are both year 0
        assert_eq!(p("-0000-01-01T00:00:00Z"), p("+0000-01-01T00:00:00Z"));
    }

    // --- Leap seconds ---

    #[test]
    fn test_parse_leap_second_clamped() {
        // :60 is clamped to :59.999_999_999
        assert_eq!(
            p("2021-06-30T23:59:60Z"),
            Some(dt(2021, 6, 30, 23, 59, 59, 999_999_999))
        );
    }

    // :61 is also clamped without `verify`; with `verify` it returns None
    #[cfg(not(feature = "verify"))]
    #[test]
    fn test_parse_invalid_second_clamped_without_verify() {
        assert_eq!(
            p("2021-10-17T10:30:61Z"),
            Some(dt(2021, 10, 17, 10, 30, 59, 999_999_999))
        );
    }

    #[cfg(feature = "verify")]
    #[test]
    fn test_parse_invalid_second_rejected_with_verify() {
        assert_eq!(p("2021-10-17T10:30:61Z"), None);
    }

    // --- Extra precision beyond 9 decimal places ---

    #[test]
    fn test_parse_extra_precision_truncated() {
        // 10th decimal digit is silently dropped
        assert_eq!(
            p("2021-10-17T10:30:00.1234567890Z"),
            Some(dt(2021, 10, 17, 10, 30, 0, 123_456_789))
        );
    }

    // --- UTC offset range validation (gated on `verify`) ---
    //
    // With `verify`: out-of-range hours (> 23) or minutes (> 59) return None.
    // Without `verify`: the offset is applied as-is, producing a silently wrong timestamp.

    #[cfg(feature = "verify")]
    #[test]
    fn test_parse_offset_hour_out_of_range() {
        assert_eq!(p("2021-10-17T10:30:00+99:00"), None);
    }

    #[cfg(not(feature = "verify"))]
    #[test]
    fn test_parse_offset_hour_out_of_range_wrong_without_verify() {
        // +99:00 applies a ~4-day shift instead of returning None
        assert!(p("2021-10-17T10:30:00+99:00") != Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[cfg(feature = "verify")]
    #[test]
    fn test_parse_offset_minute_out_of_range() {
        assert_eq!(p("2021-10-17T10:30:00+00:60"), None);
    }

    #[cfg(not(feature = "verify"))]
    #[test]
    fn test_parse_offset_minute_out_of_range_wrong_without_verify() {
        // +00:60 applies a 1-hour shift instead of returning None
        assert!(p("2021-10-17T10:30:00+00:60") != Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[cfg(feature = "verify")]
    #[test]
    fn test_parse_offset_both_out_of_range() {
        assert_eq!(p("2021-10-17T10:30:00+25:99"), None);
    }

    // --- Edge case: trailing colon after minutes silently accepted ---
    //
    // The colon consumed by the minutes parse! is treated as the seconds separator,
    // leaving offset past the colon with no digit to trigger the seconds block.

    #[test]
    fn test_parse_trailing_colon_after_minutes() {
        assert_eq!(p("2021-10-17T10:30:"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Edge case: mixed date separators silently accepted ---
    //
    // Hyphens are consumed opportunistically; absent hyphens are tolerated anywhere.

    #[test]
    fn test_parse_hyphen_only_after_year() {
        assert_eq!(p("2021-1017T10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_hyphen_only_after_month() {
        assert_eq!(p("202110-17T10:30:00Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Edge case: bare decimal separator with no following digits ---
    //
    // The digit loop breaks immediately when the next byte is not 0-9, leaving nanoseconds = 0.

    #[test]
    fn test_parse_period_no_digits() {
        assert_eq!(p("2021-10-17T10:30:00.Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    #[test]
    fn test_parse_comma_decimal_no_digits() {
        assert_eq!(p("2021-10-17T10:30:00,Z"), Some(dt(2021, 10, 17, 10, 30, 0, 0)));
    }

    // --- Date-only with Z suffix ---

    #[test]
    fn test_parse_date_only_z_suffix() {
        assert_eq!(p("2021-10-17Z"), Some(dt(2021, 10, 17, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_date_only_lowercase_z_suffix() {
        assert_eq!(p("2021-10-17z"), Some(dt(2021, 10, 17, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_date_only_compact_z_suffix() {
        assert_eq!(p("20211017Z"), Some(dt(2021, 10, 17, 0, 0, 0, 0)));
    }

    #[test]
    fn test_parse_date_only_z_not_final_byte() {
        // Z is only accepted when it's the final byte
        assert_eq!(p("2021-10-17Z!"), None);
    }

    // --- Invalid inputs ---

    #[test]
    fn test_parse_empty() {
        assert_eq!(p(""), None);
    }

    #[test]
    fn test_parse_just_plus() {
        assert_eq!(p("+"), None);
    }

    #[test]
    fn test_parse_trailing_garbage_after_z() {
        assert_eq!(p("2021-10-17T10:30:00Zextra"), None);
    }

    #[test]
    fn test_parse_trailing_garbage_after_offset() {
        assert_eq!(p("2021-10-17T10:30:00+05:00extra"), None);
    }

    #[test]
    fn test_parse_invalid_month() {
        assert_eq!(p("2021-13-17T10:30:00Z"), None);
    }

    #[test]
    fn test_parse_invalid_day() {
        assert_eq!(p("2021-10-32T10:30:00Z"), None);
    }

    #[test]
    fn test_parse_invalid_hour() {
        assert_eq!(p("2021-10-17T25:30:00Z"), None);
    }

    #[test]
    fn test_parse_invalid_minute() {
        assert_eq!(p("2021-10-17T10:60:00Z"), None);
    }

    #[test]
    fn test_parse_utc_with_trailing_garbage() {
        assert_eq!(p("2021-10-17T10:30:00UTCx"), None);
    }

    // --- Fractional time components (ISO 8601 §4.2.2.4) ---
    //
    // ISO 8601 allows a decimal fraction on the lowest-order time component present.
    // The parser only handles fractional seconds; fractional minutes and fractional hours
    // are not supported and will return None.

    #[test]
    fn test_parse_fractional_seconds() {
        // Standard case: fractional seconds are fully supported
        assert_eq!(
            p("2021-10-17T10:30:00.5Z"),
            Some(dt(2021, 10, 17, 10, 30, 0, 500_000_000))
        );
    }

    #[test]
    fn test_parse_fractional_minutes() {
        // 0.5 fractional minutes = 30 seconds
        assert_eq!(p("2021-10-17T10:30.5Z"), Some(dt(2021, 10, 17, 10, 30, 30, 0)));
    }
}
