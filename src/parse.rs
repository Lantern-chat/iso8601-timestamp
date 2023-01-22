use time::{Date, Duration, Month, PrimitiveDateTime, Time};

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[inline(always)]
unsafe fn copy_buf<const N: usize>(s: &[u8]) -> [u8; N] {
    assume!(s.len() == N);

    let mut buf = [0; N];
    buf.copy_from_slice(s);
    buf
}

#[inline(always)]
#[allow(dead_code)]
fn overflows2(x: u16) -> bool {
    const U: u16 = !0 / 255;
    0 != (x.wrapping_add(U * (127 - 9)) | x) & (U * 128)
}

/// https://graphics.stanford.edu/~seander/bithacks.html#HasMoreInWord
#[inline(always)]
#[allow(dead_code)]
fn overflows4(x: u32) -> bool {
    const U: u32 = !0 / 255;
    0 != (x.wrapping_add(U * (127 - 9)) | x) & (U * 128)
}

#[allow(dead_code)]
#[inline(always)]
fn overflows(s: &[u8]) -> bool {
    let mut overflow = false;

    for &byte in s {
        overflow |= (byte < b'0') || (byte > b'9');
    }

    overflow
}

#[inline]
fn parse_2(s: &[u8]) -> Option<u16> {
    let mut digits = u16::from_le_bytes(unsafe { copy_buf::<2>(s) });

    #[cfg(feature = "verify")]
    if overflows(s) {
        return None;
    }

    // NOTE: This may be slower than brute-force
    //if overflows2(digits - 0x3030) {
    //    return None;
    //}

    digits = ((digits & 0x0f00) >> 8) + ((digits & 0x0f) * 10);

    Some(digits)
}

#[inline]
fn parse_4(s: &[u8]) -> Option<u16> {
    let mut digits = u32::from_le_bytes(unsafe { copy_buf::<4>(s) });

    #[cfg(feature = "verify")]
    if overflows4(digits - 0x30303030) {
        return None;
    }

    digits = ((digits & 0x0f000f00) >> 8) + ((digits & 0x000f000f) * 10);
    digits = ((digits & 0x00ff00ff) >> 16) + ((digits & 0x000000ff) * 100);

    Some(digits as u16)
}

macro_rules! impl_fp {
    ($($t:ty),*) => {$(
        impl FastParse for $t {
            #[inline(always)]
            fn parse(s: &[u8]) -> Option<Self> {
                #[cfg(feature = "verify")]
                match s.len() {
                    2 => return parse_2(s).map(|v| v as $t),
                    4 => return parse_4(s).map(|v| v as $t),
                    _ => {}
                }

                #[cfg(not(feature = "verify"))]
                unsafe {
                    match s.len() {
                        2 => return Some(parse_2(s).unwrap_unchecked() as $t),
                        4 => return Some(parse_4(s).unwrap_unchecked() as $t),
                        _ => {}
                    }
                }

                let mut num = 0;

                #[allow(unused_mut)]
                let mut overflow = false;

                for byte in s {
                    let digit = byte.wrapping_sub(b'0');

                    #[cfg(feature = "verify")]
                    { overflow |= digit > 9 };

                    num = (num * 10) + digit as $t;
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

pub fn parse_iso8601(b: &[u8]) -> Option<PrimitiveDateTime> {
    let negate = matches!(b.get(0), Some(b'-')) as i16;
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

    let mut year = parse!(4, u16, b'-') as i16; // YYYY-?
    year = (year ^ -negate) + negate; // branchless conditional negation

    //println!("YEAR: {}", year);

    let month = parse!(2, u8, b'-'); // MM-?

    //println!("MONTH: {}", month);

    let day = parse!(2, u8); // DD

    //println!("DAY: {}", day);

    let ymd = Date::from_calendar_date(
        year as i32,
        // NOTE: Inlining this is cheaper than `Month::try_from(month).ok()?`
        match month {
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
        },
        day,
    )
    .ok()?;

    //println!("{}-{}-{}", year, month, day);

    match b.get(offset) {
        Some(b'T' | b't' | b' ' | b'_') => {
            offset += 1; // T
        }
        // date-only, None means it's at the end of the string
        None => return Some(PrimitiveDateTime::new(ymd, Time::MIDNIGHT)),
        _ => return None,
    }

    let hour = parse!(2, u8, b':'); // HH:?

    //println!("HOUR: {}", hour);

    let minute = parse!(2, u8, b':'); // mm:?

    //println!("MINUTE: {}", minute);

    let mut second = 0;
    let mut nanosecond = 0;

    if let Some(b'0'..=b'9') = b.get(offset) {
        second = parse!(2, u8);

        if let Some(b'.' | b',') = b.get(offset) {
            offset += 1;

            let mut factor: u32 = 100_000_000; // up to 9 decimal places

            while let Some(&c) = b.get(offset) {
                let d = c.wrapping_sub(b'0');

                if unlikely!(d > 9) {
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

    unsafe { assume!(nanosecond <= 999_999_999 && second <= 59) };

    let mut date_time = match Time::from_hms_nano(hour, minute, second, nanosecond) {
        Ok(time) => PrimitiveDateTime::new(ymd, time),
        _ => return None,
    };

    let tz = b.get(offset).copied();

    offset += 1;

    match tz {
        // Z
        Some(b'Z' | b'z') => {}

        // timezone, like +00:00
        Some(c @ b'+' | c @ b'-' | c @ 0xe2) => {
            if c == 0xe2 {
                // check for UTF8 Unicode MINUS SIGN
                if likely!(b.get(offset..(offset + 2)) == Some(&[0x88, 0x92])) {
                    offset += 2;
                } else {
                    return None;
                }
            }

            let offset_hour = parse!(2, u8, b':') as u64;
            let offset_minute = parse!(2, u8) as u64;

            let mut offset_seconds = (60 * 60 * offset_hour + offset_minute * 60) as i64;

            //let negate = (c != b'+') as i64;
            //offset_seconds = (offset_seconds ^ -negate) + negate;

            if c != b'+' {
                offset_seconds = -offset_seconds;
            }

            date_time = date_time.checked_add(Duration::seconds(offset_seconds))?;
        }

        // Parse trailing "UTC", but it does nothing, same as Z
        Some(b'U' | b'u') => match b.get(offset..(offset + 2)) {
            None => return None,
            Some(tc) => {
                // // convert to u16 and make lowercase
                // let tc = 0x2020 | unsafe { u16::from_le_bytes(copy_buf::<2>(tc)) };
                // if tc != u16::from_le_bytes(*b"tc") {
                //     return None;
                // }

                // if ((tc[1] as u16) << 8 | tc[0] as u16 | 0x2020) != u16::from_le_bytes(*b"tc") {
                //     return None;
                // }

                for (c, r) in tc.iter().zip(b"tc") {
                    if (*c | 0x20) != *r {
                        return None;
                    }
                }

                offset += 2;
            }
        },
        _ => return None,
    }

    if unlikely!(offset != b.len()) {
        return None;
    }

    Some(date_time)
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
            let s = format!("{:02}", i);
            let res = parse_2(s.as_bytes());
            assert_eq!(res, Some(i));
        }
    }

    #[test]
    fn test_parse_int4() {
        for i in 0..=9999 {
            let s = format!("{:04}", i);
            let res = parse_4(s.as_bytes());
            assert_eq!(res, Some(i));
        }
    }

    #[test]
    fn test_is_digit() {
        fn is_digit_simple(i: &[u8]) -> bool {
            for &b in i {
                if b > 9 {
                    return false;
                }
            }
            true
        }

        for i in 0..u16::MAX {
            assert_eq!(
                !overflows2(i),
                is_digit_simple(&i.to_le_bytes()),
                "{:?}",
                i.to_le_bytes()
            );
        }

        for i in 0..u32::MAX {
            assert_eq!(
                !overflows4(i),
                is_digit_simple(&i.to_le_bytes()),
                "{:?}",
                i.to_le_bytes()
            );
        }
    }
}
