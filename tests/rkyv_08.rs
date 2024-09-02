#![cfg(feature = "rkyv_08")]

use iso8601_timestamp::Timestamp;

use rkyv_08::{from_bytes, rancor::Error, to_bytes};

#[test]
fn test_rkyv() {
    #[cfg(not(feature = "std"))]
    let ts = Timestamp::from(datetime!(2024-09-01 12:32 PM).replace_millisecond(123).unwrap());

    #[cfg(feature = "std")]
    let ts = Timestamp::from(Timestamp::now_utc().replace_millisecond(123).unwrap());

    let buf = to_bytes::<Error>(&ts).unwrap();
    let de = from_bytes::<Timestamp, Error>(&buf).unwrap();

    assert_eq!(ts, de);
}
