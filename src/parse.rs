use core::convert::TryFrom;

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

#[inline]
fn parse_3(s: &[u8]) -> Option<u16> {
    unsafe { assume!(s.len() == 3) };

    let hundreds = s[0].wrapping_sub(b'0') as u16;

    #[allow(unused_mut, unused_assignments)]
    let mut overflow = false;

    #[cfg(feature = "verify")]
    {
        overflow = hundreds > 9;
    }

    match parse_2(&s[1..3]) {
        Some(tens) if !overflow => Some(tens + hundreds * 100),
        _ => None,
    }
}

macro_rules! impl_fp {
    ($($t:ty),*) => {$(
        impl FastParse for $t {
            #[inline]
            fn parse(s: &[u8]) -> Option<Self> {
                #[cfg(feature = "verify")]
                match s.len() {
                    2 => return parse_2(s).map(|v| v as $t),
                    3 => return parse_3(s).map(|v| v as $t),
                    4 => return parse_4(s).map(|v| v as $t),
                    _ => {}
                }

                #[cfg(not(feature = "verify"))]
                unsafe {
                    match s.len() {
                        2 => return Some(parse_2(s).unwrap_unchecked() as $t),
                        3 => return Some(parse_3(s).unwrap_unchecked() as $t),
                        4 => return Some(parse_4(s).unwrap_unchecked() as $t),
                        _ => {}
                    }
                }

                let mut num = 0;
                let mut overflow = false;

                for byte in s {
                    let digit = byte.wrapping_sub(b'0');
                    overflow |= digit > 9;
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

pub fn parse_iso8601(ts: &str) -> Option<PrimitiveDateTime> {
    let b = ts.as_bytes();

    #[inline(always)]
    fn parse_offset<T: FastParse>(b: &[u8], offset: usize, len: usize) -> Option<T> {
        b.get(offset..(offset + len)).and_then(|x| T::parse(x))
    }

    #[inline(always)]
    fn is_byte(b: &[u8], offset: usize, byte: u8) -> usize {
        match b.get(offset) {
            Some(&b) => (b == byte) as usize,
            None => 0,
        }
    }

    let mut offset = 0;

    let year = parse_offset::<u16>(b, offset, 4)?;
    offset += 4;
    offset += is_byte(b, offset, b'-'); // YYYY-?

    //println!("YEAR: {}", year);

    let month = parse_offset::<u8>(b, offset, 2)?;
    offset += 2;
    offset += is_byte(b, offset, b'-'); // MM-?

    //println!("MONTH: {}", month);

    let day = parse_offset::<u8>(b, offset, 2)?;
    offset += 2; // DD

    //println!("DAY: {}", day);

    let ymd = Date::from_calendar_date(year as i32, Month::try_from(month).ok()?, day).ok()?;

    //println!("{}-{}-{}", year, month, day);

    // if no T (or space), then return
    if !matches!(b.get(offset).map(|c| *c | 32), Some(b't' | b' ')) {
        return None;
    }

    offset += 1; // T

    let hour = parse_offset::<u8>(b, offset, 2)?;
    offset += 2;
    offset += is_byte(b, offset, b':');

    //println!("HOUR: {}", hour);

    let minute = parse_offset::<u8>(b, offset, 2)?;
    offset += 2;
    offset += is_byte(b, offset, b':');

    //println!("MINUTE: {}", minute);

    let maybe_time;

    // if the next character is a digit, parse seconds and milliseconds, otherwise move on
    match b.get(offset) {
        Some(b'0'..=b'9') => {
            let second = parse_offset::<u8>(b, offset, 2)?;
            offset += 2;

            if matches!(b.get(offset), Some(b'.' | b',')) {
                offset += 1;

                let mut factor: u32 = 100_000_000; // up to 9 decimal places
                let mut nanosecond: u32 = 0;

                while let Some(c) = b.get(offset) {
                    let d = c.wrapping_sub(b'0');

                    if unlikely!(d > 9) {
                        break; // break on non-numeric input
                    }

                    nanosecond += d as u32 * factor;
                    factor /= 10;
                    offset += 1;
                }

                // if leap seconds, ignore the parsed value and set it to just before 60
                // doing it this way avoids duplicate code to consume the extra characters
                if unlikely!(second == 60) {
                    maybe_time = Time::from_hms_nano(hour, minute, 59, 999_999_999);
                } else {
                    maybe_time = Time::from_hms_nano(hour, minute, second, nanosecond);
                }
            } else if unlikely!(second == 60) {
                maybe_time = Time::from_hms_nano(hour, minute, 59, 999_999_999);
            } else {
                maybe_time = Time::from_hms(hour, minute, second)
            }
        }
        _ => maybe_time = Time::from_hms(hour, minute, 0),
    }

    let mut date_time = PrimitiveDateTime::new(
        ymd,
        match maybe_time {
            Ok(time) => time,
            _ => return None,
        },
    );

    let tz = b.get(offset);

    offset += 1;

    match tz.copied() {
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

            let offset_hour = parse_offset::<u8>(b, offset, 2)? as u64;
            offset += 2;
            offset += is_byte(b, offset, b':');
            let offset_minute = parse_offset::<u8>(b, offset, 2)? as u64;
            offset += 2;

            let mut offset_seconds = (60 * 60 * offset_hour + offset_minute * 60) as i64;

            if c != b'+' {
                offset_seconds *= -1;
            }

            date_time = date_time.checked_add(Duration::seconds(offset_seconds))?;
        }

        // Parse trailing "UTC", but it does nothing, same as Z
        Some(b'U' | b'u') => match b.get(offset..(offset + 2)) {
            None => return None,
            Some(tc) => {
                for (c, r) in tc.iter().zip(b"tc") {
                    if (*c | 32) != *r {
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
    fn test_parse_int3() {
        for i in 0..=999 {
            let s = format!("{:03}", i);
            let res = parse_3(s.as_bytes());
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
