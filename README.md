ISO8601 Timestamp
=================

[![crates.io](https://img.shields.io/crates/v/iso8601-timestamp.svg)](https://crates.io/crates/iso8601-timestamp)
[![Documentation](https://docs.rs/iso8601-timestamp/badge.svg)](https://docs.rs/iso8601-timestamp)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/iso8601-timestamp.svg)](./LICENSE-Apache)

This crate provides high-performance formatting and parsing routines for ISO8601 timestamps, primarily focused on UTC values but with support for parsing (and automatically applying) UTC Offsets.

The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to a string when needed via Serde.

The [`Timestamp`] struct is only 12 bytes, while the formatted strings can be as large as 35 bytes, and care is taken to avoid heap allocations when formatting.

Example:
```rust
use iso8601_timestamp::Timestamp;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    name: String,
    ts: Timestamp, // only 12 bytes
    value: i32,
}
```
when serialized to JSON could result in:
```json
{
    "name": "some_event",
    "ts": "2021-10-17T02:03:01Z",
    "value": 42
}
```

When serializing to non-human-readable formats, such as binary formats, the `Timestamp` will be written as an `i64` representing milliseconds since the Unix Epoch. This way it only uses 8 bytes instead of 24.

Similarly, when deserializing, it supports either an ISO8601 string or an `i64` representing a unix timestamp in milliseconds.

## Cargo Features

* `std` (default)
    - Enables standard library features, such as getting the current time.

* `serde` (default)
    - Enables serde implementations for `Timestamp` and [`TimestampStr`]

* `rkyv_07`
    - Enables `rkyv` 0.7 archive support for `Timestamp`, serializing it as a 64-bit signed unix offset in milliseconds.

* `rkyv_08`
    - Enables `rkyv` 0.8 archive support for `Timestamp`, serializing it as a 64-bit signed unix offset in milliseconds.
    - NOTE: The archived representation for 0.8 is endian-agnostic, but will depend on how rkyv is configured. See rkyv's documentation for more information. Both systems will need to be configured identically.

* `verify`
    - Verifies numeric inputs when parsing and fails when non-numeric input is found.
    - When disabled, parsing ignores invalid input, possibly giving garbage timestamps.

* `pg`
    - Enables `ToSql`/`FromSql` implementations for `Timestamp` so it can be directly stored/fetched from a PostgreSQL database using `rust-postgres`

* `rusqlite`
    - Enables `ToSql`/`FromSql` implementations for `Timestamp` so it can be stored/fetched from an `rusqlite`/`sqlite3` database

* `diesel`/`diesel-pg`
    - Enables support for diesel `ToSql`/`FromSql` and `AsExpression`

* `schema`
    - Enables implementation for `JsonSchema` for generating a JSON schema on the fly using `schemars`.

* `bson`
    - Enables `visit_map` implementation to handle deserialising BSON (MongoDB) DateTime format, `{ $date: string }`.

* `rand`
    - Enables `rand` implementations, to generate random timestamps.

* `quickcheck`
    - Enables `quickcheck`'s `Arbitrary` implementation on `Timestamp`

* `worker`
    - Enables support for `now_utc()` in Cloudflare workers

* `js`
    - Enables support for `now_utc()` in WASM using `js-sys`

* `ramhorns`
    - Implements `Content` for `Timestamp`, formatting it as a regular ISO8601 timestamp.

* `fred`
    - Implements conversions between `Timestamp` and `RedisValue`/`RedisKey` to be used with `fred` Redis client.
    - Values are stored as milliseconds since the Unix Epoch, and keys are stored as ISO8601 strings.

* `borsh`
    - Implements `Borsh` (de)serialization for `Timestamp` using the `borsh` crate.
    - Timestamps are serialized as `i64` milliseconds since the Unix Epoch.