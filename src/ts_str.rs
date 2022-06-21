use generic_array::{ArrayLength, GenericArray};

mod sealed {
    pub trait Sealed {}
}

#[doc(hidden)]
pub trait TimestampStrStorage: sealed::Sealed {
    type Length: ArrayLength<u8>;

    fn init() -> GenericArray<u8, Self::Length>;

    const IS_FULL: bool;
    const HAS_OFFSET: bool;
    const PRECISION: usize;
}

/// Shorthand format without punctuation, (`YYYYMMDDTHHmmss.SSSZ`)
pub struct Short;
/// Full ISO8601 format without offset, (`YYYY-MM-DDTHH:mm:ss.SSSZ`) with character literal `Z` meaning UTC
pub struct Full;
/// Full ISO8601 format with hour/minute timezone offset, (`YYYY-MM-DDTHH:mm:ss.SSS+HZ:MZ`) with offset at end
pub struct FullOffset;
/// Full ISO8601 format without offset, but to nanosecond precision, (`YYYY-MM-DDTHH:mm:ss.SSSSSSSSSZ`)
pub struct FullNanoseconds;
/// Full ISO8601 format without offset, but to microsecond precision, (`YYYY-MM-DDTHH:mm:ss.SSSSSSZ`)
pub struct FullMicroseconds;

impl sealed::Sealed for Short {}
impl sealed::Sealed for Full {}
impl sealed::Sealed for FullOffset {}
impl sealed::Sealed for FullNanoseconds {}
impl sealed::Sealed for FullMicroseconds {}

impl TimestampStrStorage for Short {
    type Length = generic_array::typenum::consts::U20;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYYMMDDTHHmmss.SSSZ")
        GenericArray::from(*b"00000000T000000.000Z")
    }

    const IS_FULL: bool = false;
    const HAS_OFFSET: bool = false;
    const PRECISION: usize = 3;
}

impl TimestampStrStorage for Full {
    type Length = generic_array::typenum::consts::U24;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYY-MM-DDTHH:mm:ss.SSSZ")
        GenericArray::from(*b"0000-00-00T00:00:00.000Z")
    }

    const IS_FULL: bool = true;
    const HAS_OFFSET: bool = false;
    const PRECISION: usize = 3;
}

impl TimestampStrStorage for FullOffset {
    type Length = generic_array::typenum::consts::U29;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYY-MM-DDTHH:mm:ss.SSS+HH:MM")
        GenericArray::from(*b"0000-00-00T00:00:00.000+00:00")
    }

    const IS_FULL: bool = true;
    const HAS_OFFSET: bool = true;
    const PRECISION: usize = 3;
}

impl TimestampStrStorage for FullNanoseconds {
    type Length = generic_array::typenum::consts::U30;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYY-MM-DDTHH:mm:ss.SSSSSSSSSZ")
        GenericArray::from(*b"0000-00-00T00:00:00.000000000Z")
    }

    const IS_FULL: bool = true;
    const HAS_OFFSET: bool = false;
    const PRECISION: usize = 9;
}

impl TimestampStrStorage for FullMicroseconds {
    type Length = generic_array::typenum::consts::U27;

    #[inline(always)]
    fn init() -> GenericArray<u8, Self::Length> {
        //nericArray::from(*b"YYYY-MM-DDTHH:mm:ss.SSSSSSZ")
        GenericArray::from(*b"0000-00-00T00:00:00.000000Z")
    }

    const IS_FULL: bool = true;
    const HAS_OFFSET: bool = false;
    const PRECISION: usize = 6;
}

/// Fixed-size inline string storage that exactly fits the formatted timestamp
pub struct TimestampStr<S: TimestampStrStorage>(pub(crate) GenericArray<u8, S::Length>);

impl<S: TimestampStrStorage> AsRef<str> for TimestampStr<S> {
    #[inline]
    fn as_ref(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.0) }
    }
}

use core::borrow::Borrow;

impl<S: TimestampStrStorage> Borrow<str> for TimestampStr<S> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

use core::ops::Deref;

impl<S: TimestampStrStorage> Deref for TimestampStr<S> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<S: TimestampStrStorage> PartialEq for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<S: TimestampStrStorage> PartialEq<str> for TimestampStr<S> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<S: TimestampStrStorage> PartialEq<TimestampStr<S>> for str {
    #[inline]
    fn eq(&self, other: &TimestampStr<S>) -> bool {
        self == other.as_ref()
    }
}

use core::fmt;

impl<S: TimestampStrStorage> fmt::Debug for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_ref(), f)
    }
}

impl<S: TimestampStrStorage> fmt::Display for TimestampStr<S> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::ser::{Serialize, Serializer};

    use super::{TimestampStr, TimestampStrStorage};

    impl<STORAGE: TimestampStrStorage> Serialize for TimestampStr<STORAGE> {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&*self)
        }
    }
}
