use time::{PrimitiveDateTime, UtcOffset};

use crate::ts_str::{TimestampStr, TimestampStrStorage};

const fn make_table() -> [[u8; 2]; 100] {
    let mut table = [[0; 2]; 100];

    let mut i: u8 = 0;
    while i < 10 {
        let mut j: u8 = 0;
        while j < 10 {
            table[(i as usize) * 10 + (j as usize)] = [i + b'0', j + b'0'];
            j += 1;
        }
        i += 1;
    }

    table
}

const LOOKUP: [[u8; 2]; 100] = make_table();

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

#[cfg(target_arch = "x86")]
use core::arch::x86::{_mm_prefetch, _MM_HINT_T0};

#[rustfmt::skip]
#[allow(unused_assignments)]
#[inline(always)]
pub fn format_iso8601<S: TimestampStrStorage>(ts: PrimitiveDateTime, offset: UtcOffset) -> TimestampStr<S> {
    let lookup = LOOKUP.as_ptr();

    // prefetch the table while datetime parts are being destructured
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    unsafe { _mm_prefetch::<_MM_HINT_T0>(lookup as _) }

    // decompose timestamp
    //let (year, month, day) = get_ymd(ts.date());
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, nanoseconds) = ts.as_hms_nano();

    let mut buf = S::init();
    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {unsafe {
            let mut value = $s;
            let mut len = $len;

            // tell the compiler that the max value is known
            assume!(value <= $max);

            let buf = buf.as_mut_ptr().add(pos);

            // process 2 digits per iteration, this loop will likely be unrolled
            while len >= 2 {
                // skip modulus if on last 2 digits, made non-branching when unrolled
                let d1 = if len > 2 { value % 100 } else { value };

                len -= 2;
                buf.add(len).copy_from_nonoverlapping(lookup.add(d1 as usize) as *const u8, 2);

                value /= 100;
            }

            // handle remainder
            if len == 1 {
                *buf = (value as u8) + b'0';
            }

            pos += $len;

            if S::IS_FULL { pos += 1; }
        }};
    }

    write_num!(year as u16,     4, 9999);   // YYYY-
    write_num!(month as u8,     2, 12);     // MM-
    write_num!(day,             2, 31);     // DDT?
    if !S::IS_FULL { pos += 1; }            // T
    write_num!(hour,            2, 59);     // HH:
    write_num!(minute,          2, 59);     // mm:
    write_num!(second,          2, 59);     // ss.?
    if !S::IS_FULL { pos += 1; }            // .

    match S::PRECISION {
        3 => write_num!(nanoseconds / 1_000_000, 3, 999), // SSS
        9 => write_num!(nanoseconds, 9, 999_999_999),     // SSSSSSSSS
        _ => unsafe { core::hint::unreachable_unchecked() }
    }

    if S::HAS_OFFSET {
        if offset.is_negative() {
            // go back one and overwrite +
            unsafe { *buf.as_mut_ptr().add(pos - 1) = b'-'; }
        }

        let (h, m, _) = offset.as_hms();

        write_num!(h.abs(), 2, 23); // HZ:
        write_num!(m.abs(), 2, 59); // MZ
    }

    TimestampStr(buf)
}

#[cfg(test)]
mod tests {
    use crate::ts_str::FullOffset;

    use super::*;

    #[test]
    fn test_offset() {
        let ts: PrimitiveDateTime = time::macros::datetime!(2014-4-12 4:00 PM);
        let o = UtcOffset::from_hms(-4, 30, 0).unwrap();

        let formatted = format_iso8601::<FullOffset>(ts, o);

        assert_eq!("2014-04-12T16:00:00.000-04:30", &*formatted);
    }
}
