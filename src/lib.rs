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
//! Example:
//! ```rust,ignore
//! use serde::{Serialize, Deserialize};
//! use smol_str::SmolStr; // stack-allocation for small strings
//! use iso8061_timestamp::Timestamp;
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct Event {
//!     name: SmolStr,
//!     ts: Timestamp,
//!     value: i32,
//! }
//! ```
//! when serialized to JSON could result in:
//! ```json
//! {
//!     "name": "some_event",
//!     "ts": "2021-10-17T02:03:01Z",
//!     "value": 42
//! }
//! ```
//!
//! When serializing to non-human-readable formats, such as binary formats, the `Timestamp` will be written
//! as an `i64` representing milliseconds since the Unix Epoch. This way it only uses 8 bytes instead of 24.
//!
//! Similarly, when deserializing, it supports either an ISO8061 string or an `i64` representing a unix timestamp in milliseconds.
//!
//! ## Features
//!
//! * `std` (default)
//!     - Enables standard library features, such as getting the current time.
//!
//! * `serde` (default)
//!     - Enables serde implementations for `Timestamp` and `TimestampStr`
//!
//! * `nightly`
//!     - Enables nightly-specific optimizations, but without it will fallback to workarounds to enable the same optimizations.
//!
//! * `pg`
//!     - Enables `ToSql`/`FromSql` implementations for `Timestamp` so it can be directly stored/fetched from a PostgreSQL database using `rust-postgres`

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

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

use ts_str::{Full, FullNanoseconds, FullOffset, Short};

pub use ts_str::TimestampStr;

/// Timestamp formats
pub mod formats {
    pub use crate::ts_str::{Full, FullNanoseconds, FullOffset, Short};
}

/// UTC Timestamp with nanosecond precision, millisecond-precision when serialized to serde (JSON).
///
/// A `Deref`/`DerefMut` implementation is provided to gain access to the inner `PrimitiveDateTime` object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Timestamp(PrimitiveDateTime);

use core::fmt;

impl fmt::Display for Timestamp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
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

    /// Unix Epoch -- 1970-01-01 Midnight
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

    /// Format timestamp to ISO8061 with full punctuation, see [Full](formats::Full) for more information.
    pub fn format(&self) -> TimestampStr<Full> {
        format::format_iso8061(self.0, UtcOffset::UTC)
    }

    /// Format timestamp to ISO8061 without most punctuation, see [Short](formats::Short) for more information.
    pub fn format_short(&self) -> TimestampStr<Short> {
        format::format_iso8061(self.0, UtcOffset::UTC)
    }

    /// Format timestamp to ISO8061 with arbitrary UTC offset. Any offset is formatted as `+HH:MM`,
    /// and no timezone conversions are done. It is interpreted literally.
    ///
    /// See [FullOffset](formats::FullOffset) for more information.
    pub fn format_with_offset(&self, offset: UtcOffset) -> TimestampStr<FullOffset> {
        format::format_iso8061(self.0, offset)
    }

    /// Format timestamp to ISO8061 with extended precision to nanoseconds, see [FullNanoseconds](formats::FullNanoseconds) for more information.
    pub fn format_nanoseconds(&self) -> TimestampStr<FullNanoseconds> {
        format::format_iso8061(self.0, UtcOffset::UTC)
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
    #[cfg(feature = "bson")]
    use serde::de::MapAccess;
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

                #[cfg(feature = "bson")]
                fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
                where
                    M: MapAccess<'de>,
                {
                    // In the MongoDB database, or generally with BSON, dates
                    // are serialised into `{ $date: string }` where `$date`
                    // is what we actually want.

                    // Fish out the first entry we can find.
                    let (key, v) = access.next_entry::<String, String>()
                        .map_err(|_| M::Error::custom("Map Is Empty"))?
                        .ok_or_else(|| M::Error::custom("Invalid Map"))?;

                    // Match `$date` and only date.
                    if key == "$date" {
                        // Continue as normal with the given value.
                        match Timestamp::parse(&v) {
                            Some(ts) => Ok(ts),
                            None => Err(M::Error::custom("Invalid Format")),
                        }
                    } else {
                        // We don't expect anything else in the map in any case,
                        // but throw an error if we do encounter anything weird.
                        Err(M::Error::custom("Expected only key `$date` in map"))
                    }
                }

                #[inline]
                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Timestamp::from_unix_timestamp_ms(v))
                }

                #[inline]
                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Timestamp::UNIX_EPOCH + std::time::Duration::from_millis(v))
                }
            }

            deserializer.deserialize_any(TsVisitor)
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
