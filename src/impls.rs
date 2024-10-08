use time::{Date, Month};

use super::Timestamp;

#[inline(always)]
#[allow(dead_code)]
const fn is_leap_year(y: i32) -> bool {
    //(y % 4 == 0) & ((y % 25 != 0) | (y % 16 == 0)) // old version

    // ternary compiles to cmov
    y & (if y % 25 == 0 { 15 } else { 3 }) == 0
}

#[inline(always)]
#[cfg(target_feature = "avx2")]
unsafe fn to_calendar_date_avx2(date: Date) -> (i32, Month, u8) {
    import_intrinsics!(x86::{
        _mm256_set1_epi16, _mm256_set_epi16, _mm256_setzero_si256,
        _mm256_cmpeq_epi16, _mm256_movemask_epi8, _mm256_subs_epu16, __m256i
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
    let day = *core::mem::transmute::<__m256i, [u16; 16]>(days).get_unchecked(month as usize - 1);

    (year, core::mem::transmute::<u8, Month>(month as u8), day as u8)
}

#[inline(always)]
#[cfg(target_feature = "sse2")]
unsafe fn to_calendar_date_sse2(date: Date) -> (i32, Month, u8) {
    import_intrinsics!(x86::{
        _mm_cmpeq_epi16, _mm_movemask_epi8, _mm_set1_epi16,
        _mm_set_epi16, _mm_setzero_si128, _mm_subs_epu16, __m128i
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

    let day = *core::mem::transmute::<[__m128i; 2], [u16; 16]>([ld, hd]).get_unchecked(month as usize - 1);

    (year, core::mem::transmute::<u8, Month>(month as u8), day as u8)
}

#[inline(always)]
#[allow(unreachable_code)]
pub(crate) fn to_calendar_date(date: Date) -> (i32, Month, u8) {
    #[cfg(target_feature = "avx2")]
    // SAFETY: Checked for AVX2 support
    return unsafe { to_calendar_date_avx2(date) };

    #[cfg(target_feature = "sse2")]
    // SAFETY: Checked for SSE2 support
    return unsafe { to_calendar_date_sse2(date) };

    date.to_calendar_date()
}

impl Timestamp {
    /// Get the year, month, and day.
    ///
    /// Like [`PrimitiveDateTime::to_calendar_date`](time::PrimitiveDateTime::to_calendar_date), but optimized for SSE2/AVX2 when available.
    ///
    /// ```rust
    /// # use time::{Month, macros::datetime};
    /// # use iso8601_timestamp::Timestamp;
    /// assert_eq!(
    ///     Timestamp::from(datetime!(2019-01-01 0:00)).to_calendar_date(),
    ///     (2019, Month::January, 1)
    /// );
    /// ```
    #[inline(always)]
    #[must_use]
    pub fn to_calendar_date(&self) -> (i32, Month, u8) {
        to_calendar_date(self.date())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_to_calendar_date() {
        for year in &[2004, 2005, 2006] {
            for ordinal in 0..367 {
                let Ok(date) = Date::from_ordinal_date(*year, ordinal) else {
                    continue;
                };

                // SAFETY: Only tested on x86_64 with AVX2 and SSE2
                let (avx2, sse2, none) = unsafe {
                    (
                        to_calendar_date_avx2(date),
                        to_calendar_date_sse2(date),
                        date.to_calendar_date(),
                    )
                };

                assert_eq!(none, avx2);
                assert_eq!(none, sse2);
            }
        }
    }
}
