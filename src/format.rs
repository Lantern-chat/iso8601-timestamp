use generic_array::typenum as t;
use time::{Date, PrimitiveDateTime, UtcOffset};

use crate::ts_str::{template, FormatString, IsValidFormat, TimestampStr};

#[cfg(all(feature = "lookup", not(target_arch = "wasm32")))]
static LOOKUP: [[u8; 2]; 100] = {
    let mut table = [[0; 2]; 100];

    let mut i: u8 = 0;
    while i < 100 {
        let (a, b) = (i / 10, i % 10);
        table[i as usize] = [a + b'0', b + b'0'];
        i += 1;
    }

    table
};

#[allow(unused_macros)]
macro_rules! import_intrinsics {
    (x86::{$($intr:ident),*}) => {
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::{$($intr),*};
        #[cfg(target_arch = "x86")]
        use core::arch::x86::{$($intr),*};
    };
}

#[inline(always)]
#[cfg(any(target_feature = "sse2", target_feature = "avx2"))]
pub const fn is_leap_year(year: i32) -> bool {
    // NOTE: Using bitwise operators is intentional
    (year % 4 == 0) & ((year % 25 != 0) | (year % 16 == 0))
}

#[inline(always)]
#[cfg(target_feature = "avx2")]
unsafe fn to_calendar_date_avx2(date: Date) -> (i32, u8, u8) {
    import_intrinsics!(x86::{
        _mm256_set1_epi16, _mm256_set_epi16, _mm256_setzero_si256,
        _mm256_cmpeq_epi16, _mm256_movemask_epi8, _mm256_subs_epu16
    });

    let year = date.year();

    #[rustfmt::skip]
    let mut days = match is_leap_year(year) {
        true => _mm256_set_epi16(i16::MAX, i16::MAX, i16::MAX, i16::MAX, 335, 305, 274, 244, 213, 182, 152, 121, 91, 60, 31, 0),
        false => _mm256_set_epi16(i16::MAX, i16::MAX, i16::MAX, i16::MAX, 334, 304, 273, 243, 212, 181, 151, 120, 90, 59, 31, 0),
    };

    days = _mm256_subs_epu16(_mm256_set1_epi16(date.ordinal() as i16), days);

    let mask = _mm256_movemask_epi8(_mm256_cmpeq_epi16(days, _mm256_setzero_si256()));
    let month = mask.trailing_zeros() / 2;
    let day = *core::mem::transmute::<_, [u16; 16]>(days).get_unchecked(month as usize - 1);

    (year, month as u8, day as u8)
}

#[inline(always)]
#[cfg(target_feature = "sse2")]
unsafe fn to_calendar_date_sse2(date: Date) -> (i32, u8, u8) {
    import_intrinsics!(x86::{
        _mm_cmpeq_epi16, _mm_movemask_epi8, _mm_set1_epi16,
        _mm_set_epi16, _mm_setzero_si128, _mm_subs_epu16
    });

    let year = date.year();

    #[rustfmt::skip]
    let (mut hd, mut ld) = match is_leap_year(year) {
        true => (
            _mm_set_epi16(i16::MAX, i16::MAX, i16::MAX, i16::MAX, 335, 305, 274, 244),
            _mm_set_epi16(213, 182, 152, 121, 91, 60, 31, 0)),
        false => (
            _mm_set_epi16(i16::MAX, i16::MAX, i16::MAX, i16::MAX, 334, 304, 273, 243),
            _mm_set_epi16(212, 181, 151, 120, 90, 59, 31, 0))
    };

    let ordinals = _mm_set1_epi16(date.ordinal() as i16);

    hd = _mm_subs_epu16(ordinals, hd);
    ld = _mm_subs_epu16(ordinals, ld);

    let z = _mm_setzero_si128();

    let hm = _mm_movemask_epi8(_mm_cmpeq_epi16(hd, z));
    let lm = _mm_movemask_epi8(_mm_cmpeq_epi16(ld, z));

    let mask = (hm << 16) | lm;
    let month = mask.trailing_zeros() / 2;

    let day = *core::mem::transmute::<_, [u16; 16]>([ld, hd]).get_unchecked(month as usize - 1);

    (year, month as u8, day as u8)
}

#[inline(always)]
#[allow(unreachable_code)]
fn to_calendar_date(date: Date) -> (i32, u8, u8) {
    #[cfg(target_feature = "avx2")]
    return unsafe { to_calendar_date_avx2(date) };

    #[cfg(target_feature = "sse2")]
    return unsafe { to_calendar_date_sse2(date) };

    let (y, m, d) = date.to_calendar_date();
    (y, m as u8, d)
}

#[rustfmt::skip]
#[allow(unused_assignments, clippy::identity_op)]
#[inline(always)]
pub fn do_format<F: t::Bit, O: t::Bit, P: t::Unsigned>(ts: PrimitiveDateTime, offset: UtcOffset) -> TimestampStr<FormatString<F, O, P>>
where
    FormatString<F, O, P>: IsValidFormat,
{
    // Prefetch the table while datetime parts are being destructured.
    // Might cause slightly worse microbenchmark performance,
    // but may save a couple nanoseconds in real applications.
    #[cfg(all(feature = "lookup", any(target_arch = "x86_64", target_arch = "x86")))]
    unsafe {
        import_intrinsics!(x86::{_mm_prefetch, _MM_HINT_T0});
        _mm_prefetch::<_MM_HINT_T0>(LOOKUP.as_ptr() as _);
    }

    // decompose timestamp
    let (mut year, month, day) = to_calendar_date(ts.date());
    let (hour, minute, second, nanoseconds) = ts.as_hms_nano();

    let mut template = template::<F, O, P>();
    let buf = template.as_mut();

    if unlikely!(year < 0) {
        year = -year; // formatting only accepts unsigned integers
        buf[0] = b'-';
    }

    let mut pos = 1;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {{
            let mut value = $s;
            let mut len = $len;
            let mut d1 = 0;

            // tell the compiler that the max value is known
            unsafe { assume!(value <= $max); }

            // get offset stuff out of the way, freeing dependency chain for next field
            let buf = &mut buf[pos..];
            pos += $len;
            if F::BOOL { pos += 1; }

            // process 2 digits per iteration, this loop will likely be unrolled
            while len >= 2 {
                // combine these so the compiler can optimize both operations
                (value, d1) = (value / 100, value % 100);

                #[cfg(all(feature = "lookup", not(target_arch = "wasm32")))]
                {
                    let e = LOOKUP[d1 as usize];
                    len -= 1; buf[len] = e[1];
                    len -= 1; buf[len] = e[0];
                }

                #[cfg(not(all(feature = "lookup", not(target_arch = "wasm32"))))]
                {
                    let (a, b) = (d1 / 10, d1 % 10);
                    len -= 1; buf[len] = (b as u8) | b'0';
                    len -= 1; buf[len] = (a as u8) | b'0';
                }
            }

            // handle remainder
            if len == 1 {
                buf[0] = (value as u8) + b'0';
            }
        }};
    }

    write_num!(year as u16,     4, 9999);       // YYYY-
    write_num!(month      ,     2, 12);         // MM-
    write_num!(day,             2, 31);         // DDT?
    if !F::BOOL { pos += 1; }                   // T
    write_num!(hour,            2, 59);         // HH:
    write_num!(minute,          2, 59);         // mm:
    write_num!(second,          2, 59);         // ss.?(if full)
    // if not full format and has subseconds, accept period.
    if !F::BOOL && P::USIZE > 0 { pos += 1; }   // .

    // also accepts +- if Full
    match P::USIZE {
        1 => write_num!(nanoseconds / 100000000, 1, 9), // S
        2 => write_num!(nanoseconds / 10000000, 2, 99), // SS
        3 => write_num!(nanoseconds / 1000000, 3, 999), // SSS
        4 => write_num!(nanoseconds / 100000, 4, 9999), // SSSS
        5 => write_num!(nanoseconds / 10000, 5, 99999), // SSSSS
        6 => write_num!(nanoseconds / 1000, 6, 999999), // SSSSSS
        7 => write_num!(nanoseconds / 100, 7, 9999999), // SSSSSSS
        8 => write_num!(nanoseconds / 10, 8, 99999999), // SSSSSSSS
        9 => write_num!(nanoseconds / 1, 9, 999999999), // SSSSSSSSS
        _ => {}
    }

    if O::BOOL {
        if !F::BOOL { pos += 1; } // +-

        if offset.is_negative() {
            // go back one and overwrite +
            buf[pos - 1] = b'-';
        }

        let (h, m, _) = offset.as_hms();

        write_num!(h.abs(), 2, 23); // HZ
        if !F::BOOL { pos += 1; }   // :
        write_num!(m.abs(), 2, 59); // MZ
    }

    TimestampStr(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_calendar_date() {
        for year in &[2004, 2005, 2006] {
            for ordinal in 0..367 {
                let Ok(date) = Date::from_ordinal_date(*year, ordinal) else {
                    continue;
                };

                let avx2 = unsafe { to_calendar_date_avx2(date) };
                let sse2 = unsafe { to_calendar_date_sse2(date) };
                let none = {
                    let (y, m, d) = date.to_calendar_date();
                    (y, m as u8, d)
                };

                assert_eq!(none, avx2);
                assert_eq!(none, sse2);
            }
        }
    }

    // #[test]
    // fn test_get_ymd() {
    //     let mut o = 0;
    //     while o <= 367 {
    //         if let Ok(date) = time::Date::from_ordinal_date(2004, o) {
    //             let (y, m, d) = date.to_calendar_date();
    //             assert_eq!((y, m as u8, d as u16), to_calendar_date(date));
    //         }

    //         if let Ok(date) = time::Date::from_ordinal_date(2005, o) {
    //             let (y, m, d) = date.to_calendar_date();
    //             assert_eq!((y, m as u8, d as u16), to_calendar_date(date));
    //         }

    //         o += 1;
    //     }
    // }

    #[test]
    fn test_template() {
        let now = crate::Timestamp::now_utc();

        fn as_str(x: &[u8]) -> &str {
            std::str::from_utf8(x).unwrap()
        }

        macro_rules! g {
            ($($f:ty, $o:ty, $p:ty;)*) => {$(
                println!("{} -> {}",
                    as_str(&template::<$f, $o, $p>()),
                    now.format_raw::<$f, $o, $p>(time::UtcOffset::from_hms(-6, 0, 0).unwrap())
                );
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
