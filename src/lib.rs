//! ISO8061 Timestamp
//!
//! This crate provides high-performance formatting and parsing routines for ISO8061 timestamps, primarily focused on UTC values but with support
//! for parsing (and automatically applying) UTC Offsets.
//!
//! The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to
//! a string when needed via Serde.
//!
//! The [Timestamp] struct is only 12 bytes, while the formatted strings can be as large as 29 bytes,
//! and care is taken to avoid heap allocations when formatting.
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

use core::ops::{Deref, DerefMut};
use core::time::Duration;

#[cfg(feature = "std")]
use std::time::SystemTime;

use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

#[macro_use]
mod macros;

mod format;
mod parse;
mod ts_str;

use ts_str::{Full, FullOffset, Short};

pub use ts_str::TimestampStr;

/// Timestamp formats
pub mod formats {
    pub use crate::ts_str::{Full, FullOffset, Short};
}

/// UTC Timestamp with nanosecond precision, millisecond-precision when serialized to serde (JSON).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Timestamp(PrimitiveDateTime);

use core::fmt;

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.format();

        f.debug_tuple("Timestamp").field(&ts).finish()
    }
}

#[cfg(feature = "std")]
impl From<SystemTime> for Timestamp {
    fn from(ts: SystemTime) -> Self {
        Timestamp(match ts.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => Self::PRIMITIVE_UNIX_EPOCH + dur,
            Err(err) => Self::PRIMITIVE_UNIX_EPOCH - err.duration(),
        })
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(ts: OffsetDateTime) -> Self {
        let utc_datetime = ts.to_offset(UtcOffset::UTC);
        let date = utc_datetime.date();
        let time = utc_datetime.time();
        Timestamp(PrimitiveDateTime::new(date, time))
    }
}

impl From<PrimitiveDateTime> for Timestamp {
    #[inline]
    fn from(ts: PrimitiveDateTime) -> Self {
        Timestamp(ts)
    }
}

#[cfg(feature = "std")]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    pub fn now_utc() -> Self {
        SystemTime::now().into()
    }
}

impl Timestamp {
    const PRIMITIVE_UNIX_EPOCH: PrimitiveDateTime = time::macros::datetime!(1970 - 01 - 01 00:00);

    pub const UNIX_EPOCH: Self = Timestamp(Self::PRIMITIVE_UNIX_EPOCH);

    pub fn from_unix_timestamp(seconds: i64) -> Self {
        if seconds < 0 {
            Self::UNIX_EPOCH - Duration::from_secs(-seconds as u64)
        } else {
            Self::UNIX_EPOCH + Duration::from_secs(seconds as u64)
        }
    }

    pub fn from_unix_timestamp_ms(milliseconds: i64) -> Self {
        if milliseconds < 0 {
            Self::UNIX_EPOCH - Duration::from_millis(-milliseconds as u64)
        } else {
            Self::UNIX_EPOCH + Duration::from_millis(milliseconds as u64)
        }
    }

    pub fn to_unix_timestamp_ms(self) -> i64 {
        const UNIX_EPOCH_JULIAN_DAY: i64 = time::macros::date!(1970 - 01 - 01).to_julian_day() as i64;

        let day = self.to_julian_day() as i64 - UNIX_EPOCH_JULIAN_DAY;
        let (hour, minute, second, ms) = self.as_hms_milli();

        let hours = day * 24 + hour as i64;
        let minutes = hours * 60 + minute as i64;
        let seconds = minutes * 60 + second as i64;
        let millis = seconds * 1000 + ms as i64;

        millis
    }

    /// Format timestamp to ISO8061 with full punctuation
    pub fn format(&self) -> TimestampStr<Full> {
        format::format_iso8061(self.0, UtcOffset::UTC)
    }

    /// Format timestamp to ISO8061 without most punctuation
    pub fn format_short(&self) -> TimestampStr<Short> {
        format::format_iso8061(self.0, UtcOffset::UTC)
    }

    /// Format timestamp to ISO8061 with arbitrary UTC offset. Any offset is formatted as `+HH:MM`,
    /// and no timezone conversions are done. It is interpreted literally.
    pub fn format_with_offset(&self, offset: UtcOffset) -> TimestampStr<FullOffset> {
        format::format_iso8061(self.0, offset)
    }

    /// Parse to UTC timestamp from any ISO8061 string. Offsets are applied during parsing.
    #[inline]
    pub fn parse(ts: &str) -> Option<Self> {
        parse::parse_iso8061(ts).map(Timestamp)
    }

    /// Convert to `time::OffsetDateTime` with the given offset.
    pub const fn assume_offset(self, offset: UtcOffset) -> time::OffsetDateTime {
        self.0.assume_offset(offset)
    }
}

impl Deref for Timestamp {
    type Target = PrimitiveDateTime;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timestamp {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

use core::ops::{Add, Sub};

impl<T> Add<T> for Timestamp
where
    PrimitiveDateTime: Add<T, Output = PrimitiveDateTime>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: T) -> Self::Output {
        Timestamp(self.0 + rhs)
    }
}

impl<T> Sub<T> for Timestamp
where
    PrimitiveDateTime: Sub<T, Output = PrimitiveDateTime>,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: T) -> Self::Output {
        Timestamp(self.0 - rhs)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::de::{Deserialize, Deserializer, Error, Visitor};
    use serde::ser::{Serialize, Serializer};

    use super::Timestamp;

    impl Serialize for Timestamp {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                self.format().serialize(serializer)
            } else {
                self.to_unix_timestamp_ms().serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for Timestamp {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            use core::fmt;

            struct TsVisitor;

            impl<'de> Visitor<'de> for TsVisitor {
                type Value = Timestamp;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("an ISO8061 Timestamp")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    match Timestamp::parse(v) {
                        Some(ts) => Ok(ts),
                        None => Err(E::custom("Invalid Format")),
                    }
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Timestamp::from_unix_timestamp_ms(v))
                }
            }

            deserializer.deserialize_str(TsVisitor)
        }
    }
}

#[cfg(feature = "pg")]
mod pg_impl {
    use postgres_types::{accepts, to_sql_checked, FromSql, IsNull, ToSql, Type};
    use time::PrimitiveDateTime;

    use super::Timestamp;

    impl ToSql for Timestamp {
        #[inline]
        fn to_sql(
            &self,
            ty: &Type,
            out: &mut bytes::BytesMut,
        ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
        where
            Self: Sized,
        {
            self.0.to_sql(ty, out)
        }

        accepts!(TIMESTAMP, TIMESTAMPTZ);
        to_sql_checked!();
    }

    impl<'a> FromSql<'a> for Timestamp {
        #[inline]
        fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
            PrimitiveDateTime::from_sql(ty, raw).map(Timestamp)
        }

        accepts!(TIMESTAMP, TIMESTAMPTZ);
    }
}
