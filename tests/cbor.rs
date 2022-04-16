use serde::{Deserialize, Serialize};

use iso8601_timestamp::Timestamp;

#[derive(Serialize, Deserialize)]
struct Nested {
    a: i32,
    t: Timestamp,
}

#[test]
fn test_cbor() {
    let mut buf = Vec::new();

    ciborium::ser::into_writer(
        &Nested {
            a: 42,
            t: Timestamp::now_utc(),
        },
        &mut buf,
    )
    .unwrap();

    let _now: Nested = ciborium::de::from_reader(&buf[..]).unwrap();
}
