# Changelog

## [Unreleased]

## [0.4.0] - 2025-11-19

- Parser improvements (fractional minutes, hour-only offsets, date-only Z suffix)
- Add `utoipa` support
- Add `From<worker::Date>`
- Improved `no_std` support in BSON
- Update crates

## [0.3.3] - 2025-09-23

- Switch to `serde_core`
- Optimize BSON handling

## [0.3.2] - 2025-01-01

- Update deps
- Fix rusqlite in tests
- Fix broken clippy suggestion
- Update `fred` dep

## [0.3.1] - 2024-11-20

- Add `borsh` impls

## [0.3.0] - 2024-10-18

- Final 3.0 cleanup
- Improve rkyv compatibility with database backends
- Support parsing Julian days from rusqlite
- Fix rkyv impl
- Use core types where possible
- `no_std` rkyv tests
- rkyv upgrades and `impl ToSql for ArchivedTimestamp`

## [0.3.0-rc.1] - 2024-09-11

- Release candidate cleanup

## [0.3.0-beta.1] - 2024-09-11

- Try out CI workflows
- Fix tests

## [0.3.0-alpha.2] - 2024-09-01

- rkyv upgrades

## [0.3.0-alpha.1] - 2024-08-25

- Remove lookup table
- Optimize `is_leap_year`
- Make rkyv endian-agnostic
- Unify docs
- Cleanup and rkyv 0.8 support

## [0.2.17] - 2024-05-23

- Add `fred` Redis interop support
- Upgrade deps
- Update rusqlite
- Add more tests for parsing signed timestamps
- Add `MAX_LEN` associated const to `TimestampStr`
- Improve parsing of leading signs

## [0.2.16] - 2023-12-28

- Fix missing feature flag

## [0.2.15] - 2023-12-28

- Add badges

## [0.2.14] - 2023-12-27

- Improve rkyv parts
- Simplify `now_utc` codegen
- Add `From` to convert back to `SystemTime`

## [0.2.13] - 2023-12-05

- Improve rkyv support
- Fix [#6]

## [0.2.12] - 2023-11-24

- Update rusqlite
- rkyv improvements
- Cleanup and docs
- Add rkyv support

## [0.2.11] - 2023-09-20

- Support direct formatting in ramhorns
- Use `generic-array` 1.0
- Add SSE2 and AVX2 `to_calendar_date` routines
- Upgrade deps
- Test more formats
- Cleanup parsing code
- Document diesel feature
- Add Diesel `AsExpression` derive ([#5])

## [0.2.10] - 2023-06-19

- Add Diesel trait impl
- Upgrade deps
- Allow missing tz data
- WIP WASM support
- Further optimizations
- Use less unsafe code for formatting
- More work on parsing optimization

## [0.2.9] - 2023-01-23

- Include all features in docs.rs build
- Cleanup

## [0.2.8] - 2023-01-22

- API improvements and safety

## [0.2.7] - 2023-01-22

- Make using a lookup table optional
- Support BC dates and micro-optimize parsing
- Simplify const initializers
- Experiments to improve formatting speed
- Fix some `no_std` usage
- Further improve performance

## [0.2.6] - 2022-12-26

- Improve performance and flexibility

## [0.2.5] - 2022-12-26

- Benchmark the `iso8601` crate
- Allow parsing commas as decimal separator

## [0.2.4] - 2022-12-10

- Add `duration_since` and improve unix conversions
- Switch to AFL for fuzzing

## [0.2.3] - 2022-11-21

- Fix parse bug and improve sqlite3 compatibility

## [0.2.2] - 2022-11-21

- Add rusqlite support

## [0.2.1] - 2022-11-21

- Fixes and crude fuzz testing
- Mark `TimestampStr` as `Clone`/`Copy`

## [0.2.0] - 2022-06-22

- Formatting system rewrite
- Add `format_microseconds`

## [0.1.11] - 2022-06-21

- Add `format_microseconds`

## [0.1.10] - 2022-04-29

- Improve parse performance slightly by using time's own checks

## [0.1.9] - 2022-04-20

- Add `format` field to JSON Schema

## [0.1.8] - 2022-04-16

- Fix parsing min bug

## [0.1.7] - 2022-04-16

- Handle `$numberLong` variant of BSON dates ([#2])

## [0.1.6] - 2022-04-16

- Verify or clamp digits to avoid crash on garbage input
- Implement map visitor
- Add support for `schemars::JsonSchema`
- Cleanup/optimize/test parsing

## [0.1.5] - 2022-02-28

- (no changes beyond version bump)

## [0.1.4] - 2022-02-28

- Fix deserialization and add tests for CBOR
- Add unsigned deserialization

## [0.1.3] - 2022-02-05

- Fix non-x86 builds
- Remove ~40 instructions from formatting hot path

## [0.1.2] - 2022-01-19

- Add nanosecond precision formatting and simplify formatting code

## [0.1.1] - 2022-01-19

- Docs and compatibility fixes

## [0.1.0] - 2022-01-19

- Initial release
