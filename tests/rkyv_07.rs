#![cfg(feature = "rkyv_07")]

use iso8601_timestamp::{datetime, Timestamp};

use rkyv_07::{
    check_archived_root,
    //Archive, Deserialize, Serialize,
    ser::{serializers::AllocSerializer, Serializer},
    Deserialize,
    Infallible,
};

#[test]
fn test_rkyv() {
    #[cfg(not(feature = "std"))]
    let ts = Timestamp::from(datetime!(2024-09-01 12:32 PM).replace_millisecond(123).unwrap());

    #[cfg(feature = "std")]
    let ts = Timestamp::from(Timestamp::now_utc().replace_millisecond(123).unwrap());

    let mut ser = AllocSerializer::<1024>::default();
    ser.serialize_value(&ts).unwrap();

    let buf = ser.into_serializer().into_inner();
    let archived_value = check_archived_root::<Timestamp>(&buf).unwrap();

    println!("{:?}", archived_value);

    let de = archived_value.deserialize(&mut Infallible).unwrap();

    assert_eq!(ts, de);
}
