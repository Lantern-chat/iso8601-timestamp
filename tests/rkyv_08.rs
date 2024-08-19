#![cfg(feature = "rkyv_08")]

use iso8601_timestamp::Timestamp;

use rkyv_08::{from_bytes, rancor::Error, to_bytes};

#[test]
fn test_rkyv() {
    let ts = Timestamp::from(Timestamp::now_utc().replace_millisecond(123).unwrap());

    let buf = to_bytes::<Error>(&ts).unwrap();
    let de = from_bytes::<Timestamp, Error>(&buf).unwrap();

    assert_eq!(ts, de);
}
