#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    missing_docs,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::must_use_candidate,
    clippy::perf,
    clippy::complexity,
    clippy::suspicious
)]

use core::ops::{AddAssign, Deref, DerefMut, SubAssign};

#[cfg(feature = "std")]
use std::time::SystemTime;

pub extern crate time;

pub use time::{Duration, UtcOffset};
use time::{OffsetDateTime, PrimitiveDateTime};

pub use generic_array::typenum;
use typenum as t;

#[macro_use]
mod macros;

mod format;
mod impls;
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
            Ok(dur) => *Self::UNIX_EPOCH + dur,
            Err(err) => *Self::UNIX_EPOCH - err.duration(),
        })
    }
}

#[cfg(feature = "std")]
impl From<Timestamp> for SystemTime {
    fn from(ts: Timestamp) -> Self {
        SystemTime::UNIX_EPOCH + ts.duration_since(Timestamp::UNIX_EPOCH)
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
    ///
    /// # Panics
    /// This will panic if the System Time is before the Unix Epoch.
    #[inline]
    #[must_use]
    pub fn now_utc() -> Self {
        Timestamp(
            *Self::UNIX_EPOCH
                + SystemTime::UNIX_EPOCH
                    .elapsed()
                    .expect("SystemTime before UNIX_EPOCH"),
        )
    }
}

#[cfg(all(feature = "worker", target_arch = "wasm32", not(feature = "js")))]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    #[must_use]
    pub fn now_utc() -> Self {
        match Timestamp::UNIX_EPOCH
            .checked_add(Duration::milliseconds(worker::Date::now().as_millis() as i64))
        {
            Some(ts) => ts,
            None => unreachable!("Invalid Date::now() value"),
        }
    }
}

#[cfg(all(
    feature = "js",
    any(target_arch = "wasm32", target_arch = "wasm64"),
    not(feature = "worker")
))]
impl Timestamp {
    /// Get the current time, assuming UTC
    #[inline]
    #[must_use]
    pub fn now_utc() -> Self {
        match Timestamp::UNIX_EPOCH.checked_add(Duration::milliseconds(js_sys::Date::now() as i64)) {
            Some(ts) => ts,
            None => unreachable!("Invalid Date::now() value"),
        }
    }
}

/// Preconfigured formats
pub mod formats {
    use super::*;

    /// `2023-03-24T07:05:59.005Z`
    pub type FullMilliseconds = FormatString<t::True, t::False, t::U3>;
    /// `2023-03-24T07:05:59.005000Z`
    pub type FullMicroseconds = FormatString<t::True, t::False, t::U6>;
    /// `2023-03-24T07:05:59.005432101Z`
    pub type FullNanoseconds = FormatString<t::True, t::False, t::U9>;

    /// `2023-03-24T07:05:59.005+05:00`
    pub type FullMillisecondsOffset = FormatString<t::True, t::True, t::U3>;

    /// `20230324T070559.005Z`
    pub type ShortMilliseconds = FormatString<t::False, t::False, t::U3>;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_short_ms_length() {
        // ensure the short format could fit within a smolstr/compact_str
        assert_eq!(
            <<ShortMilliseconds as crate::ts_str::IsValidFormat>::Length as super::t::Unsigned>::USIZE,
            "+20230324T070559.005Z".len()
        );

        assert!("+20230324T070559.005Z".len() <= 23);
    }
}

/// Construct a [`Timestamp`] with a statically known value.
///
/// The resulting expression can be used in `const` or `static` declarations.
///
/// See [`time::macros::datetime`](time::macros) for more information.
///
/// The variation presented here does not support timezone offsets.
#[macro_export]
macro_rules! datetime {
    ($($tt:tt)*) => {
        $crate::Timestamp::from_primitive_datetime(time::macros::datetime!($($tt)*))
    };
}

impl Timestamp {
    /// Unix Epoch -- 1970-01-01 Midnight
    pub const UNIX_EPOCH: Self = datetime!(1970 - 01 - 01 00:00);

    /// Constructs a [`Timestamp`] from a [`PrimitiveDateTime`]
    #[inline(always)]
    #[must_use]
    pub const fn from_primitive_datetime(dt: PrimitiveDateTime) -> Self {
        Timestamp(dt)
    }

    /// Returns the amount of time elapsed from an earlier point in time.
    #[inline]
    #[must_use]
    pub fn duration_since(self, earlier: Self) -> Duration {
        self.0 - earlier.0
    }

    /// Formats the timestamp given the provided formatting parameters
    #[must_use]
    pub fn format_raw<F: t::Bit, O: t::Bit, P: t::Unsigned>(
        &self,
        offset: UtcOffset,
    ) -> TimestampStr<FormatString<F, O, P>>
    where
        FormatString<F, O, P>: IsValidFormat,
    {
        format::do_format(self.0, offset)
    }

    /// Formats a full timestamp without offset, using the given subsecond precision level.
    #[inline(always)]
    #[must_use]
    pub fn format_with_precision<P: t::Unsigned>(&self) -> TimestampStr<FormatString<t::True, t::False, P>>
    where
        FormatString<t::True, t::False, P>: IsValidFormat,
    {
        self.format_raw(UtcOffset::UTC)
    }

    /// Format timestamp to ISO8601 with full punctuation, to millisecond precision.
    #[inline(always)]
    #[must_use]
    pub fn format(&self) -> TimestampStr<formats::FullMilliseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 with extended precision to nanoseconds.
    #[inline(always)]
    #[must_use]
    pub fn format_nanoseconds(&self) -> TimestampStr<formats::FullNanoseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 with extended precision to microseconds.
    #[inline(always)]
    #[must_use]
    pub fn format_microseconds(&self) -> TimestampStr<formats::FullMicroseconds> {
        self.format_with_precision()
    }

    /// Format timestamp to ISO8601 without most punctuation, to millisecond precision.
    #[inline(always)]
    #[must_use]
    pub fn format_short(&self) -> TimestampStr<formats::ShortMilliseconds> {
        self.format_raw(UtcOffset::UTC)
    }

    /// Format timestamp to ISO8601 with arbitrary UTC offset. Any offset is formatted as `+HH:MM`,
    /// and no timezone conversions are done. It is interpreted literally.
    #[inline(always)]
    #[must_use]
    pub fn format_with_offset(&self, offset: UtcOffset) -> TimestampStr<formats::FullMillisecondsOffset> {
        self.format_raw(offset)
    }

    /// Formats a full timestamp with timezone offset, and the provided level of subsecond precision.
    #[inline(always)]
    #[must_use]
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
    #[inline(never)]
    #[must_use] // Avoid deoptimizing the general &str case when presented with a fixed-size string
    pub fn parse(ts: &str) -> Option<Self> {
        parse::parse_iso8601(ts.as_bytes()).map(Timestamp)
    }

    /// Convert to `time::OffsetDateTime` with the given offset.
    #[inline(always)]
    #[must_use]
    pub const fn assume_offset(self, offset: UtcOffset) -> time::OffsetDateTime {
        self.0.assume_offset(offset)
    }

    /// Computes `self + duration`, returning `None` if an overflow occurred.
    ///
    /// See [`PrimitiveDateTime::checked_add`] for more implementation details
    #[inline]
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub const fn saturating_add(self, duration: Duration) -> Self {
        Timestamp(self.0.saturating_add(duration))
    }

    /// Computes `self - duration`, saturating value on overflow.
    ///
    /// See [`PrimitiveDateTime::saturating_sub`] for more implementation details
    #[inline]
    #[must_use]
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
        ) -> Result<IsNull, Box<dyn core::error::Error + Sync + Send>>
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
        fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn core::error::Error + Sync + Send>> {
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

    use core::{error, fmt, str};

    impl error::Error for InvalidTimestamp {}
    impl fmt::Display for InvalidTimestamp {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Invalid ISO8601 Timestamp")
        }
    }

    impl FromSql for Timestamp {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            // https://www.sqlite.org/lang_datefunc.html
            match value {
                ValueRef::Text(bytes) => match str::from_utf8(bytes) {
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

                // according to the link above, dates stored as floats are the number of
                // fractional days since -4713-11-24 12:00:00, and 2440587.5 is the
                // number of days between -4713-11-24 12:00:00 and 1970-01-01 00:00:00
                ValueRef::Real(ts) => {
                    let ts = Duration::seconds_f64((ts - 2440587.5) * 86_400.0);

                    Timestamp::UNIX_EPOCH
                        .checked_add(ts)
                        .ok_or_else(|| FromSqlError::OutOfRange(ts.whole_seconds()))
                }

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

#[cfg(all(feature = "rkyv_07", feature = "rkyv_08"))]
compile_error!("Cannot enable both rkyv 0.7 and 0.8 features at the same time");

#[cfg(feature = "rkyv_08")]
pub use rkyv_08_impl::ArchivedTimestamp;

#[cfg(feature = "rkyv_08")]
mod rkyv_08_impl {
    use super::*;

    use rkyv_08::{
        bytecheck::CheckBytes,
        place::Place,
        rancor::{Fallible, Source},
        traits::NoUndef,
        Archive, Archived, Deserialize, Serialize,
    };

    /// `rkyv`-ed Timestamp as a 64-bit signed millisecond offset from the UNIX Epoch.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, rkyv_08::Portable)]
    #[rkyv(crate = rkyv_08)]
    #[repr(transparent)]
    pub struct ArchivedTimestamp(pub Archived<i64>);

    // SAFETY: ArchivedTimestamp is repr(transparent) over i64_le
    unsafe impl NoUndef for ArchivedTimestamp {}

    impl ArchivedTimestamp {
        /// Get the raw millisecond offset
        #[inline(always)]
        #[must_use]
        pub const fn get(self) -> i64 {
            self.0.to_native()
        }
    }

    impl From<ArchivedTimestamp> for Timestamp {
        fn from(value: ArchivedTimestamp) -> Self {
            Timestamp::UNIX_EPOCH
                .checked_add(Duration::milliseconds(value.get()))
                .unwrap_or(Timestamp::UNIX_EPOCH)
        }
    }

    impl Archive for Timestamp {
        type Archived = ArchivedTimestamp;
        type Resolver = ();

        fn resolve(&self, _resolver: Self::Resolver, out: Place<Self::Archived>) {
            out.write(ArchivedTimestamp(<Archived<i64>>::from_native(
                self.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds() as i64,
            )));
        }
    }

    impl<S: Fallible + ?Sized> Serialize<S> for Timestamp {
        #[inline(always)]
        fn serialize(&self, _serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            Ok(())
        }
    }

    impl<D: Fallible + ?Sized> Deserialize<Timestamp, D> for ArchivedTimestamp {
        #[inline]
        fn deserialize(&self, _deserializer: &mut D) -> Result<Timestamp, <D as Fallible>::Error> {
            Ok(Timestamp::from(*self))
        }
    }

    // SAFETY: ArchivedTimestamp is repr(transparent) over i64_le
    unsafe impl<C> CheckBytes<C> for ArchivedTimestamp
    where
        C: Fallible + ?Sized,
        <C as Fallible>::Error: Source,
    {
        #[inline(always)]
        unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<(), C::Error> {
            CheckBytes::<C>::check_bytes(value as *const Archived<i64>, context)
        }
    }

    #[cfg(feature = "pg")]
    const _: () = {
        use postgres_types::{accepts, to_sql_checked, IsNull, ToSql, Type};

        impl ToSql for ArchivedTimestamp {
            fn to_sql(
                &self,
                _ty: &Type,
                out: &mut bytes::BytesMut,
            ) -> Result<IsNull, Box<dyn core::error::Error + Sync + Send>> {
                const EPOCH_OFFSET: i64 = 946684800000000; // 2000-01-01T00:00:00Z

                // convert to microseconds
                let Some(ts) = self.0.to_native().checked_mul(1000) else {
                    return Err("Timestamp out of range".into());
                };

                // convert to postgres timestamp
                let Some(pts) = ts.checked_sub(EPOCH_OFFSET) else {
                    return Err("Timestamp out of range".into());
                };

                postgres_protocol::types::time_to_sql(pts, out);

                Ok(IsNull::No)
            }

            accepts!(TIMESTAMP, TIMESTAMPTZ);
            to_sql_checked!();
        }
    };
}

#[cfg(feature = "rkyv_07")]
pub use rkyv_07_impl::ArchivedTimestamp;

#[cfg(feature = "rkyv_07")]
mod rkyv_07_impl {
    use super::{Duration, Timestamp};

    use rend_04::LittleEndian;

    /// `rkyv`-ed Timestamp as a 64-bit signed millisecond offset from the UNIX Epoch.
    ///
    /// This value is Endian-agnostic, with zero overhead on little-endian archs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct ArchivedTimestamp(LittleEndian<i64>);

    impl ArchivedTimestamp {
        /// Get the raw millisecond offset
        #[inline(always)]
        #[must_use]
        pub const fn get(self) -> i64 {
            self.0.value()
        }
    }

    impl From<ArchivedTimestamp> for Timestamp {
        fn from(value: ArchivedTimestamp) -> Self {
            Timestamp::UNIX_EPOCH
                .checked_add(Duration::milliseconds(value.get()))
                .unwrap_or(Timestamp::UNIX_EPOCH)
        }
    }

    use rkyv_07::{Archive, Archived, CheckBytes, Deserialize, Fallible, Serialize};

    impl<C: ?Sized> CheckBytes<C> for ArchivedTimestamp {
        type Error = <LittleEndian<i64> as CheckBytes<C>>::Error;

        #[inline(always)]
        unsafe fn check_bytes<'a>(value: *const Self, _context: &mut C) -> Result<&'a Self, Self::Error> {
            Ok(&*value)
        }
    }

    impl Archive for Timestamp {
        type Archived = ArchivedTimestamp;
        type Resolver = ();

        unsafe fn resolve(&self, _pos: usize, _resolver: Self::Resolver, out: *mut Self::Archived) {
            out.write(ArchivedTimestamp(LittleEndian::<i64>::new(
                self.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds() as i64,
            )))
        }
    }

    impl<S> Serialize<S> for Timestamp
    where
        S: Fallible + ?Sized,
    {
        #[inline(always)]
        fn serialize(&self, _serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            Ok(())
        }
    }

    impl<D> Deserialize<Timestamp, D> for Archived<Timestamp>
    where
        D: Fallible + ?Sized,
    {
        #[inline]
        fn deserialize(&self, _deserializer: &mut D) -> Result<Timestamp, <D as Fallible>::Error> {
            Ok(Timestamp::from(*self))
        }
    }
}

#[cfg(feature = "fred")]
mod fred_impl {
    use fred::{
        error::{RedisError, RedisErrorKind},
        types::{Expiration, FromRedis, FromRedisKey, RedisKey, RedisValue},
    };

    use super::{Duration, Timestamp};

    impl From<Timestamp> for RedisValue {
        fn from(ts: Timestamp) -> Self {
            RedisValue::Integer(ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds() as i64)
        }
    }

    impl From<Timestamp> for RedisKey {
        fn from(ts: Timestamp) -> Self {
            RedisKey::from(&*ts.format())
        }
    }

    impl FromRedis for Timestamp {
        fn from_value(value: RedisValue) -> Result<Self, RedisError> {
            match value {
                RedisValue::String(ts) => Timestamp::parse(&ts)
                    .ok_or_else(|| RedisError::new(RedisErrorKind::Parse, "Invalid Timestamp format")),
                RedisValue::Bytes(ts) => match core::str::from_utf8(&ts) {
                    Ok(ts) => Timestamp::parse(ts)
                        .ok_or_else(|| RedisError::new(RedisErrorKind::Parse, "Invalid Timestamp format")),
                    Err(_) => Err(RedisError::new(RedisErrorKind::Parse, "Invalid UTF-8 Timestamp")),
                },
                RedisValue::Integer(ts) => Timestamp::UNIX_EPOCH
                    .checked_add(Duration::seconds(ts))
                    .ok_or_else(|| RedisError::new(RedisErrorKind::Parse, "Timestamp out of range")),
                _ => Err(RedisError::new(RedisErrorKind::Parse, "Invalid Timestamp type")),
            }
        }
    }

    impl FromRedisKey for Timestamp {
        fn from_key(value: RedisKey) -> Result<Self, RedisError> {
            let Ok(value) = core::str::from_utf8(value.as_bytes()) else {
                return Err(RedisError::new(RedisErrorKind::Parse, "Invalid UTF-8 Key"));
            };

            Timestamp::parse(value)
                .ok_or_else(|| RedisError::new(RedisErrorKind::Parse, "Invalid Timestamp format"))
        }
    }

    impl From<Timestamp> for Expiration {
        fn from(ts: Timestamp) -> Self {
            Expiration::PXAT(ts.duration_since(Timestamp::UNIX_EPOCH).whole_milliseconds() as i64)
        }
    }
}
