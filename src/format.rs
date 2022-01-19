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

/*
use time::{Date, Month};

const fn make_day_table(leap: bool) -> [(Month, u8); 366] {
    let mut table = [(Month::January, 0); 366];

    let mut i = 1;
    while i < 366 {
        if let Ok(date) = Date::from_ordinal_date(if leap { 2020 } else { 2019 }, i) {
            let (_, month, day) = date.to_calendar_date();
            table[i as usize] = (month, day);
        }
        i += 1;
    }

    table
}

const ORDINAL_TABLE: [(Month, u8); 366] = make_day_table(false);
const ORDINAL_TABLE_L: [(Month, u8); 366] = make_day_table(true);

fn get_ymd(d: Date) -> (i32, Month, u8) {
    let year = d.year();

    let table = match time::util::is_leap_year(year) {
        true => &ORDINAL_TABLE_L,
        false => &ORDINAL_TABLE,
    };

    let ordinal = d.ordinal();

    let (month, day) = unsafe { *table.get_unchecked(ordinal as usize) };

    (year, month, day)
}
*/

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

#[cfg(target_arch = "x86")]
use core::arch::x86::{_mm_prefetch, _MM_HINT_T0};

#[rustfmt::skip]
#[allow(unused_assignments)]
#[inline(always)]
pub fn format_iso8061<S: TimestampStrStorage>(ts: PrimitiveDateTime, offset: UtcOffset) -> TimestampStr<S> {
    let lookup = LOOKUP.as_ptr();
    if cfg!(any(target_arch = "x86_64", target_arch = "x86")) {
        unsafe { _mm_prefetch::<_MM_HINT_T0>(lookup as _) }
    }

    // decompose timestamp
    //let (year, month, day) = get_ymd(ts.date());
    let (year, month, day) = ts.to_calendar_date();
    let (hour, minute, second, milliseconds) = ts.as_hms_milli();

    let mut buf = S::init();
    let mut pos = 0;

    macro_rules! write_num {
        ($s: expr, $len: expr, $max: expr) => {unsafe {
            let value = $s;

            // tell the compiler that the max value is known
            assume!(value <= $max);

            let buf = buf.as_mut_ptr().add(pos);

            match $len {
                2 => {
                    buf.copy_from_nonoverlapping(lookup.add(value as usize) as *const u8, 2);
                }
                3 => {
                    let ab = value / 10;
                    let c = value % 10;

                    buf.copy_from_nonoverlapping(lookup.add(ab as usize) as *const u8, 2);
                    *buf.add(2) = (*lookup.add(c as usize))[1];
                }
                4 => {
                    let value = value as u16;

                    let ab = value / 100;
                    let cd = value % 100;

                    buf.copy_from_nonoverlapping(lookup.add(ab as usize) as *const u8, 2);
                    buf.add(2).copy_from_nonoverlapping(lookup.add(cd as usize) as *const u8, 2);
                }
                _ => core::hint::unreachable_unchecked()
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
    write_num!(milliseconds,    3, 999);    // SSS

    if S::HAS_OFFSET && S::IS_FULL {
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

        let formatted = format_iso8061::<FullOffset>(ts, o);

        assert_eq!("2014-04-12T16:00:00.000-04:30", &*formatted);
    }
}
