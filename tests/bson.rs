#![cfg(feature = "bson")]

use iso8601_timestamp::Timestamp;

#[test]
fn test_bson() {
    let a = "{ \"$date\": \"2021-10-17T02:03:01+00:00\" }";
    let b = "{ \"$date\": { \"$numberLong\": \"1634436181000\" } }";

    let ta = serde_json::from_str::<Timestamp>(a).unwrap();
    let tb = serde_json::from_str::<Timestamp>(b).unwrap();

    assert_eq!(ta, tb);
}
