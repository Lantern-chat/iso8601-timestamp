use time::{Date, Duration, Month, PrimitiveDateTime, Time};

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[cfg(any(test, not(feature = "verify")))]
#[inline(always)]
fn parse_2(s: &[u8]) -> u8 {
    unsafe { assume!(s.len() == 2) };

    // NOTE: Despite doing the same as the loop below, this is a hair faster
    // (like a single clock cycle) due to instruction-level parallelism
    (s[0] & 0x0f) * 10 + (s[1] & 0x0f)
}

#[cfg(any(test, not(feature = "verify")))]
#[inline(always)]
fn parse_4(s: &[u8]) -> u16 {
    unsafe { assume!(s.len() == 4) };

    let mut digits = u32::from_le_bytes({
        let mut buf = [0; 4];
        buf.copy_from_slice(s);
        buf
    });

    digits = ((digits & 0x0f000f00) >> 8) + ((digits & 0x000f000f) * 10);
    digits = ((digits & 0x00ff00ff) >> 16) + ((digits & 0x000000ff) * 100);

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
    let negate = matches!(b.first(), Some(b'-')) as i32;
    let mut offset = negate as usize;

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
        Some(b'T' | b't' | b' ' | b'_') => {
            offset += 1; // T
        }
        // date-only, None means it's at the end of the string
        None => return Some(date_time),
        _ => return None,
    }

    let hour = parse!(2, u8, b':'); // HH:?
    let minute = parse!(2, u8, b':'); // mm:?

    let mut second = 0;
    let mut nanosecond = 0;

    if let Some(b'0'..=b'9') = b.get(offset) {
        second = parse!(2, u8);

        if let Some(b'.' | b',') = b.get(offset) {
            offset += 1;

            let mut factor: u32 = 100_000_000; // up to 9 decimal places

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
    }

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
                if likely!(b.get(offset..(offset + 2)) == Some(&[0x88, 0x92])) {
                    offset += 2;
                } else {
                    return None;
                }
            }

            let tz_offset_hour = parse!(2, u8, b':') as i64;
            let tz_offset_minute = parse!(2, u8) as i64;

            if unlikely!(offset != b.len()) {
                return None;
            }

            if tz_offset_hour == 0 && tz_offset_minute == 0 {
                return Some(date_time);
            }

            let tz_offset = Duration::seconds(60 * 60 * tz_offset_hour + tz_offset_minute * 60);

            // these generate function calls regardless, so avoid
            // negating the offset and just chose which call to make
            let checked_op: fn(PrimitiveDateTime, Duration) -> Option<PrimitiveDateTime> = match c != b'+' {
                true => PrimitiveDateTime::checked_sub as _,
                false => PrimitiveDateTime::checked_add as _,
            };

            checked_op(date_time, tz_offset)
        }

        // Parse trailing "UTC", but it does nothing, same as Z
        Some(b'U' | b'u') => match b.get(offset..(offset + 2)) {
            None => None,
            Some(tc) => {
                // avoid multiple branches when this loop is unrolled
                let mut invalid = false;
                for (c, r) in tc.iter().zip(b"tc") {
                    invalid |= (*c | 0x20) != *r;
                }

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

    #[test]
    fn test_parse_int2() {
        for i in 0..=99 {
            let s = format!("{i:02}");
            let res = parse_2(s.as_bytes());
            assert_eq!(res, i);
        }
    }

    #[test]
    fn test_parse_int4() {
        for i in 0..=9999 {
            let s = format!("{i:04}");
            let res = parse_4(s.as_bytes());
            assert_eq!(res, i);
        }
    }
}
