use core::marker::PhantomData;
use core::ops::{Add, Mul};
use generic_array::{typenum as t, ArrayLength, GenericArray};

mod sealed {
    pub trait Sealed {}
}

// convert boolean/bit to integer
type I<BOOL> = t::UInt<t::UTerm, BOOL>;

// 4 bytes for full-formatting (--::)
type F4<F> = t::Prod<I<F>, t::U4>;
// 5 bytes for offset (00:00)
type O5<O> = t::Prod<I<O>, t::U5>;
// '+' + 4Y + 2M + 2D + T + 2H + 2m + 2s + Z
type P17<P> = t::Sum<P, t::U17>;
type P18<P> = t::Sum<P17<P>, I<t::Gr<P, t::U0>>>; // accounts for . that's only present when P>0
type F4O5<F, O> = t::Sum<F4<F>, O5<O>>;

type StrLen<F, O, P> = t::Sum<P18<P>, F4O5<F, O>>;

#[doc(hidden)]
pub struct FormatString<F, O, P>(PhantomData<(F, O, P)>);

impl<F, O, P> sealed::Sealed for FormatString<F, O, P> {}

#[doc(hidden)]
pub trait IsValidFormat: sealed::Sealed {
    type Length: ArrayLength;
    type Storage: AsRef<[u8]> + AsMut<[u8]> + Clone + Copy + Default;
}

impl<F, O, P> IsValidFormat for FormatString<F, O, P>
where
    F: t::Bit,
    I<F>: Mul<t::U4>,
    O: t::Bit,
    I<O>: Mul<t::U5>,
    P: t::Unsigned + Add<t::U17> + t::IsLessOrEqual<t::U9, Output = t::True> + t::IsGreater<t::U0>,
    F4<F>: Add<O5<O>>,
    P17<P>: Add<I<t::Gr<P, t::U0>>>,
    P18<P>: Add<F4O5<F, O>>,
    StrLen<F, O, P>: ArrayLength,

    <StrLen<F, O, P> as ArrayLength>::ArrayType<u8>: Copy,
{
    type Length = StrLen<F, O, P>;
    type Storage = GenericArray<u8, Self::Length>;
}

#[allow(unused_assignments)]
#[inline(always)]
#[rustfmt::skip]
pub fn template<F: t::Bit, O: t::Bit, P: t::Unsigned>() -> <FormatString<F, O, P> as IsValidFormat>::Storage
where
    FormatString<F, O, P>: IsValidFormat,
{
    let mut value: <FormatString<F, O, P> as IsValidFormat>::Storage = Default::default();

    macro_rules! w {
        ($x:literal) => {value.as_mut().copy_from_slice($x)};
    }

    match (F::BOOL, O::BOOL, P::USIZE) {
        (true,  true,  0) => w!(b"+0000-00-00T00:00:00+00:00"),
        (true,  true,  1) => w!(b"+0000-00-00T00:00:00.0+00:00"),
        (true,  true,  2) => w!(b"+0000-00-00T00:00:00.00+00:00"),
        (true,  true,  3) => w!(b"+0000-00-00T00:00:00.000+00:00"),
        (true,  true,  4) => w!(b"+0000-00-00T00:00:00.0000+00:00"),
        (true,  true,  5) => w!(b"+0000-00-00T00:00:00.00000+00:00"),
        (true,  true,  6) => w!(b"+0000-00-00T00:00:00.000000+00:00"),
        (true,  true,  7) => w!(b"+0000-00-00T00:00:00.0000000+00:00"),
        (true,  true,  8) => w!(b"+0000-00-00T00:00:00.00000000+00:00"),
        (true,  true,  9) => w!(b"+0000-00-00T00:00:00.000000000+00:00"),
        (true,  false, 0) => w!(b"+0000-00-00T00:00:00Z"),
        (true,  false, 1) => w!(b"+0000-00-00T00:00:00.0Z"),
        (true,  false, 2) => w!(b"+0000-00-00T00:00:00.00Z"),
        (true,  false, 3) => w!(b"+0000-00-00T00:00:00.000Z"),
        (true,  false, 4) => w!(b"+0000-00-00T00:00:00.0000Z"),
        (true,  false, 5) => w!(b"+0000-00-00T00:00:00.00000Z"),
        (true,  false, 6) => w!(b"+0000-00-00T00:00:00.000000Z"),
        (true,  false, 7) => w!(b"+0000-00-00T00:00:00.0000000Z"),
        (true,  false, 8) => w!(b"+0000-00-00T00:00:00.00000000Z"),
        (true,  false, 9) => w!(b"+0000-00-00T00:00:00.000000000Z"),
        (false, true,  0) => w!(b"+00000000T000000+00:00"),
        (false, true,  1) => w!(b"+00000000T000000.0+00:00"),
        (false, true,  2) => w!(b"+00000000T000000.00+00:00"),
        (false, true,  3) => w!(b"+00000000T000000.000+00:00"),
        (false, true,  4) => w!(b"+00000000T000000.0000+00:00"),
        (false, true,  5) => w!(b"+00000000T000000.00000+00:00"),
        (false, true,  6) => w!(b"+00000000T000000.000000+00:00"),
        (false, true,  7) => w!(b"+00000000T000000.0000000+00:00"),
        (false, true,  8) => w!(b"+00000000T000000.00000000+00:00"),
        (false, true,  9) => w!(b"+00000000T000000.000000000+00:00"),
        (false, false, 0) => w!(b"+00000000T000000Z"),
        (false, false, 1) => w!(b"+00000000T000000.0Z"),
        (false, false, 2) => w!(b"+00000000T000000.00Z"),
        (false, false, 3) => w!(b"+00000000T000000.000Z"),
        (false, false, 4) => w!(b"+00000000T000000.0000Z"),
        (false, false, 5) => w!(b"+00000000T000000.00000Z"),
        (false, false, 6) => w!(b"+00000000T000000.000000Z"),
        (false, false, 7) => w!(b"+00000000T000000.0000000Z"),
        (false, false, 8) => w!(b"+00000000T000000.00000000Z"),
        (false, false, 9) => w!(b"+00000000T000000.000000000Z"),
        _ => unsafe { core::hint::unreachable_unchecked() },
    }

    value
}

/// Fixed-size inline string storage that exactly fits the formatted timestamp
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TimestampStr<S: IsValidFormat>(pub(crate) S::Storage);

impl<S: IsValidFormat> AsRef<str> for TimestampStr<S> {
    #[inline]
    fn as_ref(&self) -> &str {
        unsafe {
            // skip + sign if positive
            let bytes = self.0.as_ref();
            let is_positive = *bytes.get_unchecked(0) == b'+';
            core::str::from_utf8_unchecked(bytes.get_unchecked(is_positive as usize..))
        }
    }
}

use core::borrow::Borrow;

impl<S: IsValidFormat> Borrow<str> for TimestampStr<S> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

use core::ops::Deref;

impl<S: IsValidFormat> Deref for TimestampStr<S> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<S: IsValidFormat> PartialEq for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<S: IsValidFormat> PartialEq<str> for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<S: IsValidFormat> PartialEq<&str> for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl<S: IsValidFormat> PartialEq<TimestampStr<S>> for str {
    #[inline]
    fn eq(&self, other: &TimestampStr<S>) -> bool {
        self == other.as_ref()
    }
}

impl<S: IsValidFormat> PartialEq<TimestampStr<S>> for &str {
    #[inline]
    fn eq(&self, other: &TimestampStr<S>) -> bool {
        *self == other.as_ref()
    }
}

use core::fmt;

impl<S: IsValidFormat> fmt::Debug for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_ref(), f)
    }
}

impl<S: IsValidFormat> fmt::Display for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::ser::{Serialize, Serializer};

    use super::{IsValidFormat, TimestampStr};

    impl<STORAGE: IsValidFormat> Serialize for TimestampStr<STORAGE> {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self)
        }
    }
}
