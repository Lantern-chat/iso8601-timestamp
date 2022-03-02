use core::convert::TryFrom;
use core::time::Duration;

use time::{Date, Month, PrimitiveDateTime, Time};

/// Trait implemented locally for very fast parsing of small unsigned integers
trait FastParse: Sized {
    fn parse(s: &[u8]) -> Option<Self>;
}

#[inline]
fn parse_2(s: &[u8]) -> u16 {
    unsafe { assume!(s.len() == 2) };

    let mut buf = [0; 2];
    buf.copy_from_slice(s);

    let digits = u16::from_le_bytes(buf);
    ((digits & 0x0f00) >> 8) + ((digits & 0x0f) * 10)
}

#[inline]
fn parse_4(s: &[u8]) -> u16 {
    unsafe { assume!(s.len() == 4) };

    let mut buf = [0; 4];
    buf.copy_from_slice(s);

    let mut digits = u32::from_le_bytes(buf);
    digits = ((digits & 0x0f000f00) >> 8) + ((digits & 0x000f000f) * 10);
    digits = ((digits & 0x00ff00ff) >> 16) + ((digits & 0x000000ff) * 100);
    digits as u16
}

#[inline]
fn parse_3(s: &[u8]) -> u16 {
    unsafe { assume!(s.len() == 3) };

    parse_2(&s[1..3]) + (s[0] - b'0') as u16 * 100
}

// TODO: Parse 5 and 6?

macro_rules! impl_fp {
    ($($t:ty),*) => {$(
        impl FastParse for $t {
            #[inline]
            fn parse(s: &[u8]) -> Option<Self> {
                match s.len() {
                    2 => return Some(parse_2(s) as $t),
                    4 => return Some(parse_4(s) as $t),
                    3 => return Some(parse_3(s) as $t),
                    //1 => return Some((s[0].wrapping_sub(b'0')) as $t),
                    _ => {}
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

pub fn parse_iso8061(ts: &str) -> Option<PrimitiveDateTime> {
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

    // only parsed 4 digits
    unsafe { assume!(year <= 9999) };

    let ymd = Date::from_calendar_date(year as i32, Month::try_from(month).ok()?, day).ok()?;

    //println!("{}-{}-{}", year, month, day);

    // if no T, then return
    if b.get(offset).map(|c| *c | 32) != Some(b't') {
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

            if b.get(offset).copied() == Some(b'.') {
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

    //println!("SECOND: {}", second);

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
        Some(b'z' | b'Z') => {}

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

            let dur = Duration::from_secs(60 * 60 * offset_hour + offset_minute * 60);

            if c == b'+' {
                date_time += dur;
            } else {
                date_time -= dur;
            }
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
            assert_eq!(res, i);
        }
    }

    #[test]
    fn test_parse_int3() {
        for i in 0..=999 {
            let s = format!("{:03}", i);
            let res = parse_3(s.as_bytes());
            assert_eq!(res, i);
        }
    }

    #[test]
    fn test_parse_int4() {
        for i in 0..=9999 {
            let s = format!("{:04}", i);
            let res = parse_4(s.as_bytes());
            assert_eq!(res, i);
        }
    }
}
