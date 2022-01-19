ISO8061 Timestamp
=================

This crate provides high-performance formatting and parsing routines for ISO8061 timestamps, primarily focused on UTC values but with support for parsing (and automatically applying) UTC Offsets.

The primary purpose of this is to keep the lightweight representation of timestamps within data structures, and only formatting it to a string when needed via Serde.

The `Timestamp` struct is only 12 bytes, while the formatted strings can be as large as 29 bytes, and care is taken to avoid heap allocations when formatting.