use iso8061_timestamp::{formats::Full, Timestamp, TimestampStr};

#[inline(never)]
#[no_mangle]
pub fn format_iso8061(ts: Timestamp) -> TimestampStr<Full> {
    ts.format()
}

#[inline(never)]
#[no_mangle]
pub fn parse_iso8061(ts: &str) -> Option<Timestamp> {
    Timestamp::parse(ts)
}

fn main() {}
