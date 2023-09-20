//! ISO8601 Timestamp
//!
//! This crate provides high-performance formatting and parsing routines for ISO8601 timestamps, primarily focused on UTC values but with support
//! for parsing (and automatically applying) UTC Offsets.
//!
//! The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to
//! a string when needed via Serde.
//!
//! The [Timestamp] struct is only 12 bytes, while the formatted strings can be as large as 35 bytes,
//! and care is taken to avoid heap allocations when formatting.
//!
//! Example:
//! ```rust,ignore
//! use serde::{Serialize, Deserialize};
//! use smol_str::SmolStr; // stack-allocation for small strings
//! use iso8601_timestamp::Timestamp;
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
//! Similarly, when deserializing, it supports either an ISO8601 string or an `i64` representing a unix timestamp in milliseconds.
//!
//! ## Cargo Features
//!
//! * `std` (default)
//!     - Enables standard library features, such as getting the current time.
//!
//! * `lookup` (default)
//!     - Enables use of a 200-byte lookup table during formatting. Slightly faster with a hot cache. Disabling saves 200 bytes at a ~20% slowdown.
//!
//! * `serde` (default)
//!     - Enables serde implementations for `Timestamp` and `TimestampStr`
//!
//! * `verify`
//!     - Verifies numeric inputs when parsing and fails when non-numeric input is found.
//!     - When disabled, parsing ignores invalid input, possibly giving garbage timestamps.
//!
//! * `nightly`
//!     - Enables nightly-specific optimizations, but without it will fallback to workarounds to enable the same optimizations.
//!
//! * `pg`
//!     - Enables `ToSql`/`FromSql` implementations for `Timestamp` so it can be directly stored/fetched from a PostgreSQL database using `rust-postgres`
//!
//! * `rusqlite`
//!     - Enables `ToSql`/`FromSql` implementations for `Timestamp` so it can be stored/fetched from an `rusqlite`/`sqlite3` database
//!
//! * `diesel`/`diesel-pg`
//!     - Enables support for diesel `ToSql`/`FromSql` and `AsExpression`
//!
//! * `schema`
//!     - Enables implementation for `JsonSchema` for generating a JSON schema on the fly using `schemars`.
//!
//! * `bson`
//!     - Enables `visit_map` implementation to handle deserialising BSON (MongoDB) DateTime format, `{ $date: string }`.
//!
//! * `rand`
//!     - Enables `rand` implementations, to generate random timestamps.
//!
//! * `quickcheck`
//!     - Enables `quickcheck`'s `Arbitrary` implementation on `Timestamp`
//!
//! * `worker`
//!     - Enables support for `now_utc()` in Cloudflare workers
//!
//! * `js`
//!     - Enables support for `now_utc()` in WASM using `js-sys`
//!
//! * `ramhorns`
//!     - Implements `Content` for `Timestamp`, formatting it as a regular ISO8601 timestamp. Note that `ramhorns` is GPLv3.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

use core::ops::{AddAssign, Deref, DerefMut, SubAssign};

#[cfg(feature = "std")]
use std::time::SystemTime;

pub use time::{Duration, UtcOffset};
use time::{OffsetDateTime, PrimitiveDateTime};

pub use generic_array::typenum;
use typenum as t;

#[macro_use]
mod macros;

mod format;
mod parse;
mod ts_str;

use ts_str::IsValidFormat;
pub use ts_str::{FormatString, TimestampStr};

/// UTC Timestamp with nanosecond precision, millisecond-precision when serialized to serde (JSON).
///
/// A `Deref`/`DerefMut` implementation is provided to gain access to the inner `PrimitiveDateTime` object.
#[cfg_attr(feature = "diesel", derive(diesel::AsExpression, diesel::FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Timestamp))]
#[cfg_attr(feature = "diesel-pg", diesel(sql_type = diesel::sql_types::Timestamptz))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Timestamp(PrimitiveDateTime);

use core::fmt;

impl fmt::Debug for Timestamp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Timestamp")
            .field(&self.format_nanoseconds())
            .finish()
    }
}

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

// SystemTime::now() is not implemented on wasm32
#[cfg(all(feature = "std", not(any(target_arch = "wasm64", target_arch = "wasm32"))))]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    pub fn now_utc() -> Self {
        SystemTime::now().into()
    }
}

#[cfg(all(feature = "worker", target_arch = "wasm32", not(feature = "js")))]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    pub fn now_utc() -> Self {
        match Timestamp::UNIX_EPOCH
            .checked_add(Duration::milliseconds(worker::Date::now().as_millis() as i64))
        {
            Some(ts) => ts,
            None => unreachable!("Invalid Date::now() value"),
        }
    }
}

#[cfg(all(feature = "js", any(target_arch = "wasm32", target_arch = "wasm64")))]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    pub fn now_utc() -> Self {
        match Timestamp::UNIX_EPOCH.checked_add(Duration::milliseconds(js_sys::Date::now() as i64)) {
            Some(ts) => ts,
            None => unreachable!("Invalid Date::now() value"),
        }
    }
}

pub mod formats {
    use super::*;

    pub type FullMilliseconds = FormatString<t::True, t::False, t::U3>;
    pub type FullMicroseconds = FormatString<t::True, t::False, t::U6>;
    pub type FullNanoseconds = FormatString<t::True, t::False, t::U9>;

    pub type FullMillisecondsOffset = FormatString<t::True, t::True, t::U3>;

    pub type ShortMilliseconds = FormatString<t::False, t::False, t::U3>;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_short_ms_length() {
        // ensure the short format could fit within a smolstr/compact_str
        assert!(
            <<ShortMilliseconds as crate::ts_str::IsValidFormat>::Length as super::t::Unsigned>::USIZE <= 22
        );
    }
}

impl Timestamp {
    const PRIMITIVE_UNIX_EPOCH: PrimitiveDateTime = time::macros::datetime!(1970 - 01 - 01 00:00);

    /// Unix Epoch -- 1970-01-01 Midnight
    pub const UNIX_EPOCH: Self = Timestamp(Self::PRIMITIVE_UNIX_EPOCH);

    #[deprecated = "Use `Timestamp::UNIX_EPOCH.checked_add(Duration::seconds(seconds))`"]
    pub fn from_unix_timestamp(seconds: i64) -> Self {
        Self::UNIX_EPOCH + time::Duration::seconds(seconds)
    }

    #[deprecated = "Use `Timestamp::UNIX_EPOCH.checked_add(Duration::milliseconds(milliseconds))`"]
    pub fn from_unix_timestamp_ms(milliseconds: i64) -> Self {
        Self::UNIX_EPOCH + time::Duration::milliseconds(milliseconds)
    }

    #[deprecated = "Use `self.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds()`"]
    pub fn to_unix_timestamp_ms(self) -> i64 {
        const UNIX_EPOCH_JULIAN_DAY: i64 = time::macros::date!(1970 - 01 - 01).to_julian_day() as i64;

        let day = self.to_julian_day() as i64 - UNIX_EPOCH_JULIAN_DAY;
        let (hour, minute, second, ms) = self.as_hms_milli();

        let hours = day * 24 + hour as i64;
        let minutes = hours * 60 + minute as i64;
        let seconds = minutes * 60 + second as i64;
        let millis = seconds * 1000 + ms as i64;

        #[allow(clippy::let_and_return)]
        millis
    }

    /// Returns the amount of time elapsed from an earlier point in time.
    #[inline]
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.0 - earlier.0
    }

    pub fn format_raw<F: t::Bit, O: t::Bit, P: t::Unsigned>(
        &self,
        offset: UtcOffset,
    ) -> TimestampStr<FormatString<F, O, P>>
    where
        FormatString<F, O, P>: IsValidFormat,
    {
        format::do_format(self.0, offset)
    }

    #[inline(always)]
    pub fn format_with_precision<P: t::Unsigned>(&self) -> TimestampStr<FormatString<t::True, t::False, P>>
    where
        FormatString<t::True, t::False, P>: IsValidFormat,
    {
        self.format_raw(UtcOffset::UTC)
    }

    /// Format timestamp to ISO8601 with full punctuation, to millisecond precision.
    #[inline(always)]
    pub fn format(&self) -> TimestampStr<formats::FullMilliseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 with extended precision to nanoseconds.
    #[inline(always)]
    pub fn format_nanoseconds(&self) -> TimestampStr<formats::FullNanoseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 with extended precision to microseconds.
    #[inline(always)]
    pub fn format_microseconds(&self) -> TimestampStr<formats::FullMicroseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 without most punctuation, to millisecond precision.
    #[inline(always)]
    pub fn format_short(&self) -> TimestampStr<formats::ShortMilliseconds> {
        self.format_raw(UtcOffset::UTC)
    }

    /// Format timestamp to ISO8601 with arbitrary UTC offset. Any offset is formatted as `+HH:MM`,
    /// and no timezone conversions are done. It is interpreted literally.
    #[inline(always)]
    pub fn format_with_offset(&self, offset: UtcOffset) -> TimestampStr<formats::FullMillisecondsOffset> {
        self.format_raw(offset)
    }

    #[inline(always)]
    pub fn format_with_offset_and_precision<P: t::Unsigned>(
        &self,
        offset: UtcOffset,
    ) -> TimestampStr<FormatString<t::True, t::True, P>>
    where
        FormatString<t::True, t::True, P>: IsValidFormat,
    {
        self.format_raw(offset)
    }

    /// Parse to UTC timestamp from any ISO8601 string. Offsets are applied during parsing.
    #[inline(never)] // Avoid deoptimizing the general &str case when presented with a fixed-size string
    pub fn parse(ts: &str) -> Option<Self> {
        parse::parse_iso8601(ts.as_bytes()).map(Timestamp)
    }

    /// Convert to `time::OffsetDateTime` with the given offset.
    #[inline(always)]
    pub const fn assume_offset(self, offset: UtcOffset) -> time::OffsetDateTime {
        self.0.assume_offset(offset)
    }

    /// Computes `self + duration`, returning `None` if an overflow occurred.
    ///
    /// See [`PrimitiveDateTime::checked_add`] for more implementation details
    #[inline]
    pub const fn checked_add(self, duration: Duration) -> Option<Self> {
        match self.0.checked_add(duration) {
            Some(ts) => Some(Timestamp(ts)),
            None => None,
        }
    }

    /// Computes `self - duration`, returning `None` if an overflow occurred.
    ///
    /// See [`PrimitiveDateTime::checked_sub`] for more implementation details
    #[inline]
    pub const fn checked_sub(self, duration: Duration) -> Option<Self> {
        match self.0.checked_sub(duration) {
            Some(ts) => Some(Timestamp(ts)),
            None => None,
        }
    }

    /// Computes `self + duration`, saturating value on overflow.
    ///
    /// See [`PrimitiveDateTime::saturating_add`] for more implementation details
    #[inline]
    pub const fn saturating_add(self, duration: Duration) -> Self {
        Timestamp(self.0.saturating_add(duration))
    }

    /// Computes `self - duration`, saturating value on overflow.
    ///
    /// See [`PrimitiveDateTime::saturating_sub`] for more implementation details
    #[inline]
    pub const fn saturating_sub(self, duration: Duration) -> Self {
        Timestamp(self.0.saturating_sub(duration))
    }
}

impl Deref for Timestamp {
    type Target = PrimitiveDateTime;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timestamp {
    #[inline(always)]
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

impl<T> AddAssign<T> for Timestamp
where
    PrimitiveDateTime: AddAssign<T>,
{
    #[inline]
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs;
    }
}

impl<T> SubAssign<T> for Timestamp
where
    PrimitiveDateTime: SubAssign<T>,
{
    #[inline]
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs;
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::de::{Deserialize, Deserializer, Error, Visitor};
    use serde::ser::{Serialize, Serializer};

    #[cfg(feature = "bson")]
    use serde::de::MapAccess;

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
                (self.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds() as i64).serialize(serializer)
            }
        }
    }

    const OUT_OF_RANGE: &str = "Milliseconds out of range";

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
                    formatter.write_str("an ISO8601 Timestamp")
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

                    // Though in some cases if the year is < 1970 or > 9999, it will be:
                    // `{ $date: { $numberLong: string } }` where `$numberLong` is a signed integer (as a string)

                    #[derive(serde::Deserialize, Debug)]
                    #[serde(untagged)]
                    enum StringOrNumberLong {
                        Str(Timestamp),
                        Num {
                            #[serde(rename = "$numberLong")]
                            num: String,
                        },
                    }

                    // Fish out the first entry we can find.
                    let (key, v) = access
                        .next_entry::<String, StringOrNumberLong>()?
                        .ok_or_else(|| M::Error::custom("Map Is Empty"))?;

                    // Match `$date` and only date.
                    if key == "$date" {
                        match v {
                            StringOrNumberLong::Str(ts) => Ok(ts),
                            StringOrNumberLong::Num { num } => match num.parse::<i64>() {
                                Ok(ms) => Timestamp::UNIX_EPOCH
                                    .checked_add(time::Duration::milliseconds(ms))
                                    .ok_or_else(|| M::Error::custom(OUT_OF_RANGE)),
                                Err(_) => return Err(M::Error::custom("Invalid Number")),
                            },
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
                    Timestamp::UNIX_EPOCH
                        .checked_add(time::Duration::milliseconds(v))
                        .ok_or_else(|| E::custom(OUT_OF_RANGE))
                }

                #[inline]
                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let seconds = v / 1000;
                    let nanoseconds = (v % 1_000) * 1_000_000;

                    Timestamp::UNIX_EPOCH
                        .checked_add(time::Duration::new(seconds as i64, nanoseconds as i32))
                        .ok_or_else(|| E::custom(OUT_OF_RANGE))
                }
            }

            deserializer.deserialize_any(TsVisitor)
        }
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    #[cfg(feature = "diesel-pg")]
    use diesel::sql_types::Timestamptz as DbTimestamptz;
    use diesel::{
        backend::Backend,
        deserialize::{self, FromSql},
        serialize::{self, ToSql},
        sql_types::Timestamp as DbTimestamp,
    };
    use time::PrimitiveDateTime;

    use super::Timestamp;

    impl<DB> FromSql<DbTimestamp, DB> for Timestamp
    where
        DB: Backend,
        PrimitiveDateTime: FromSql<DbTimestamp, DB>,
    {
        fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            <PrimitiveDateTime as FromSql<DbTimestamp, DB>>::from_sql(bytes).map(Timestamp::from)
        }
    }

    #[cfg(feature = "diesel-pg")]
    impl<DB> FromSql<DbTimestamptz, DB> for Timestamp
    where
        DB: Backend,
        PrimitiveDateTime: FromSql<DbTimestamptz, DB>,
    {
        fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            <PrimitiveDateTime as FromSql<DbTimestamptz, DB>>::from_sql(bytes).map(Timestamp::from)
        }
    }

    impl<DB> ToSql<DbTimestamp, DB> for Timestamp
    where
        DB: Backend,
        PrimitiveDateTime: ToSql<DbTimestamp, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
            <PrimitiveDateTime as ToSql<DbTimestamp, DB>>::to_sql(self, out)
        }
    }

    #[cfg(feature = "diesel-pg")]
    impl<DB> ToSql<DbTimestamptz, DB> for Timestamp
    where
        DB: Backend,
        PrimitiveDateTime: ToSql<DbTimestamptz, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
            <PrimitiveDateTime as ToSql<DbTimestamptz, DB>>::to_sql(self, out)
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

#[cfg(feature = "rusqlite")]
mod rusqlite_impl {
    use super::{Duration, Timestamp};

    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};
    use rusqlite::Error;

    #[derive(Debug)]
    struct InvalidTimestamp;

    impl std::error::Error for InvalidTimestamp {}
    impl std::fmt::Display for InvalidTimestamp {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("Invalid ISO8601 Timestamp")
        }
    }

    impl FromSql for Timestamp {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            // https://www.sqlite.org/lang_datefunc.html
            match value {
                ValueRef::Text(bytes) => match core::str::from_utf8(bytes) {
                    Err(e) => Err(FromSqlError::Other(Error::Utf8Error(e).into())),
                    Ok(ts) => match Timestamp::parse(ts) {
                        Some(ts) => Ok(ts),
                        None => Err(FromSqlError::Other(InvalidTimestamp.into())),
                    },
                },
                // according to the link above, dates stored as integers are seconds since unix epoch
                ValueRef::Integer(ts) => Timestamp::UNIX_EPOCH
                    .checked_add(Duration::seconds(ts))
                    .ok_or_else(|| FromSqlError::OutOfRange(ts)),

                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl ToSql for Timestamp {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::Owned(Value::Text(self.format().to_owned())))
        }
    }
}

#[cfg(feature = "schema")]
mod schema_impl {
    use schemars::_serde_json::json;
    use schemars::schema::{InstanceType, Metadata, Schema, SchemaObject, SingleOrVec};
    use schemars::JsonSchema;

    use super::Timestamp;

    impl JsonSchema for Timestamp {
        fn schema_name() -> String {
            "ISO8601 Timestamp".to_owned()
        }

        fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
            Schema::Object(SchemaObject {
                metadata: Some(Box::new(Metadata {
                    description: Some("ISO8601 formatted timestamp".to_owned()),
                    examples: vec![json!("1970-01-01T00:00:00Z")],
                    ..Default::default()
                })),
                format: Some("date-time".to_owned()),
                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                ..Default::default()
            })
        }
    }
}

#[cfg(feature = "rand")]
mod rand_impl {
    use rand::distributions::{Distribution, Standard};
    use rand::Rng;

    use super::Timestamp;

    impl Distribution<Timestamp> for Standard {
        #[inline]
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Timestamp {
            Timestamp(rng.gen())
        }
    }
}

#[cfg(feature = "quickcheck")]
mod quickcheck_impl {
    extern crate alloc;

    use alloc::boxed::Box;
    use quickcheck::{Arbitrary, Gen};

    use super::Timestamp;

    impl Arbitrary for Timestamp {
        #[inline(always)]
        fn arbitrary(g: &mut Gen) -> Self {
            Timestamp(Arbitrary::arbitrary(g))
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(
                (self.date(), self.time())
                    .shrink()
                    .map(|(d, t)| Timestamp(time::PrimitiveDateTime::new(d, t))),
            )
        }
    }
}

#[cfg(feature = "ramhorns")]
mod ramhorns_impl {
    use super::{formats::FullMilliseconds, ts_str::IsValidFormat, Timestamp};

    use ramhorns::{encoding::Encoder, Content};

    impl Content for Timestamp {
        fn capacity_hint(&self, _tpl: &ramhorns::Template) -> usize {
            use generic_array::typenum::Unsigned;

            <FullMilliseconds as IsValidFormat>::Length::USIZE
        }

        fn render_escaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
            encoder.write_unescaped(&self.format())
        }
    }
}
