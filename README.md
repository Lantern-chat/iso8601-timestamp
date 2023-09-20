ISO8601 Timestamp
=================

This crate provides high-performance formatting and parsing routines for ISO8601 timestamps, primarily focused on UTC values but with support for parsing (and automatically applying) UTC Offsets.

The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to a string when needed via Serde.

The `Timestamp` struct is only 12 bytes, while the formatted strings can be as large as 35 bytes, and care is taken to avoid heap allocations when formatting.

Example:
```rust
use serde::{Serialize, Deserialize};
use smol_str::SmolStr; // stack-allocation for small strings
use iso8601_timestamp::Timestamp;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    name: SmolStr,
    ts: Timestamp,
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

* `lookup` (default)
    - Enables use of a 200-byte lookup table during formatting. Slightly faster with a hot cache. Disabling saves 200 bytes at a ~20% slowdown.

* `serde` (default)
    - Enables serde implementations for `Timestamp` and `TimestampStr`

* `verify`
    - Verifies numeric inputs when parsing and fails when non-numeric input is found.
    - When disabled, parsing ignores invalid input, possibly giving garbage timestamps.

* `nightly`
    - Enables nightly-specific optimizations, but without it will fallback to workarounds to enable the same optimizations.

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
    - Implements `Content` for `Timestamp`, formatting it as a regular ISO8601 timestamp. Note that `ramhorns` is GPLv3.