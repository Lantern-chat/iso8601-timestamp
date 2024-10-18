//! This example is used to examine generated assembly code via the command:
//! ```
//! cargo rustc --example ts_asm --release --  -C codegen-units=1 -C opt-level=3 --emit asm
//! ```

use iso8601_timestamp::{formats::*, Timestamp, TimestampStr};
use time::Month;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn format_iso8601(ts: Timestamp) -> TimestampStr<FullMilliseconds> {
    ts.format()
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn parse_iso8601(ts: &str) -> Option<Timestamp> {
    Timestamp::parse(ts)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn to_calendar_date(ts: Timestamp) -> (i32, Month, u8) {
    ts.to_calendar_date()
}

fn main() {}
