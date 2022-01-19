ISO8061 Timestamp
=================

This crate provides high-performance formatting and parsing routines for ISO8061 timestamps, primarily focused on UTC values but with support for parsing (and automatically applying) UTC Offsets.

The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to a string when needed via Serde.

The `Timestamp` struct is only 12 bytes, while the formatted strings can be as large as 29 bytes, and care is taken to avoid heap allocations when formatting.

Example:
```rust
use serde::{Serialize, Deserialize};
use smol_str::SmolStr; // stack-allocation for small strings
use iso8061_timestamp::Timestamp;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    name: SmolStr,
    ts: Timestamp,
    value: i32,
}
```
when formatted to JSON could result in:
```json
{
    "name": "some_event",
    "ts": "2021-10-17T02:03:01Z",
    "value": 42,
}
```

When serializing to non-human-readable formats, such as binary formats, the `Timestamp` will be written as an `i64` representing milliseconds since the Unix Epoch. This way it only uses 8 bytes instead of 24.