use iso8601_timestamp::{formats::*, Timestamp, TimestampStr};

#[inline(never)]
#[no_mangle]
pub fn format_iso8601(ts: Timestamp) -> TimestampStr<FullMilliseconds> {
    ts.format()
}

#[inline(never)]
#[no_mangle]
pub fn parse_iso8601(ts: &str) -> Option<Timestamp> {
    Timestamp::parse(ts)
}

fn main() {}
