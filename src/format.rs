use time::{PrimitiveDateTime, UtcOffset};

use crate::ts_str::{template, FormatString, IsValidFormat, TimestampStr};

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

use generic_array::typenum as t;

#[rustfmt::skip]
#[allow(unused_assignments)]
#[inline(always)]
pub fn do_format<F: t::Bit, O: t::Bit, P: t::Unsigned>(ts: PrimitiveDateTime, offset: UtcOffset) -> TimestampStr<FormatString<F, O, P>>
where
    FormatString<F, O, P>: IsValidFormat,
{
    let lookup = LOOKUP.as_ptr();

    // prefetch the table while datetime parts are being destructured
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    unsafe { _mm_prefetch::<_MM_HINT_T0>(lookup as _) }

    // decompose timestamp
    //let (year, month, day) = get_ymd(ts.date());
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, nanoseconds) = ts.as_hms_nano();

    let mut buf = template::<F, O, P>();

    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {unsafe {
            let mut value = $s;
            let mut len = $len;

            // tell the compiler that the max value is known
            assume!(value <= $max);

            let buf = buf.as_mut().as_mut_ptr().add(pos);

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

            if F::BOOL { pos += 1; }
        }};
    }

    write_num!(year as u16,     4, 9999);       // YYYY-
    write_num!(month as u8,     2, 12);         // MM-
    write_num!(day,             2, 31);         // DDT?
    if !F::BOOL { pos += 1; }                   // T
    write_num!(hour,            2, 59);         // HH:
    write_num!(minute,          2, 59);         // mm:
    write_num!(second,          2, 59);         // ss.?(if full)
    // if not full format and has subseconds, accept period.
    if !F::BOOL && P::USIZE > 0 { pos += 1; }   // .

    // also accepts +- if Full
    match P::USIZE {
        0 => {}
        1 => write_num!(nanoseconds / 100000000, 1, 9), // S
        2 => write_num!(nanoseconds / 10000000, 2, 99), // SS
        3 => write_num!(nanoseconds / 1000000, 3, 999), // SSS
        4 => write_num!(nanoseconds / 100000, 4, 9999), // SSSS
        5 => write_num!(nanoseconds / 10000, 5, 99999), // SSSSS
        6 => write_num!(nanoseconds / 1000, 6, 999999), // SSSSSS
        7 => write_num!(nanoseconds / 100, 7, 9999999), // SSSSSSS
        8 => write_num!(nanoseconds / 10, 8, 99999999), // SSSSSSSS
        9 => write_num!(nanoseconds / 1, 9, 999999999), // SSSSSSSSS
        _ => unsafe { std::hint::unreachable_unchecked() }
    }

    if O::BOOL {
        if !F::BOOL { pos += 1; } // +-

        if offset.is_negative() {
            // go back one and overwrite +
            unsafe { *buf.as_mut().as_mut_ptr().add(pos - 1) = b'-'; }
        }

        let (h, m, _) = offset.as_hms();

        write_num!(h.abs(), 2, 23); // HZ
        if !F::BOOL { pos += 1; }   // :
        write_num!(m.abs(), 2, 59); // MZ
    }

    TimestampStr(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template() {
        fn as_str<'a>(x: &'a [u8]) -> &'a str {
            std::str::from_utf8(x).unwrap()
        }

        macro_rules! g {
            ($($f:ty, $o:ty, $p:ty;)*) => {$(
                println!("{}", as_str(&template::<$f, $o, $p>()));
            )*}
        }

        g! {
            t::True, t::True, t::U0;
            t::True, t::True, t::U1;
            t::True, t::True, t::U2;
            t::True, t::True, t::U3;
            t::True, t::True, t::U4;
            t::True, t::True, t::U5;
            t::True, t::True, t::U6;
            t::True, t::True, t::U7;
            t::True, t::True, t::U8;
            t::True, t::True, t::U9;
            t::True, t::False, t::U0;
            t::True, t::False, t::U1;
            t::True, t::False, t::U2;
            t::True, t::False, t::U3;
            t::True, t::False, t::U4;
            t::True, t::False, t::U5;
            t::True, t::False, t::U6;
            t::True, t::False, t::U7;
            t::True, t::False, t::U8;
            t::True, t::False, t::U9;
            t::False, t::True, t::U0;
            t::False, t::True, t::U1;
            t::False, t::True, t::U2;
            t::False, t::True, t::U3;
            t::False, t::True, t::U4;
            t::False, t::True, t::U5;
            t::False, t::True, t::U6;
            t::False, t::True, t::U7;
            t::False, t::True, t::U8;
            t::False, t::True, t::U9;
            t::False, t::False, t::U0;
            t::False, t::False, t::U1;
            t::False, t::False, t::U2;
            t::False, t::False, t::U3;
            t::False, t::False, t::U4;
            t::False, t::False, t::U5;
            t::False, t::False, t::U6;
            t::False, t::False, t::U7;
            t::False, t::False, t::U8;
            t::False, t::False, t::U9;
        }
    }
}
