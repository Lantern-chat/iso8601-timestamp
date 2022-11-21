use iso8601_timestamp::Timestamp;

#[test]
fn test_bad_inputs() {
    Timestamp::parse("9999\u{1}\u{2}\u{12}UT\u{1}92-+?!\\\0");
    Timestamp::parse("9999\u{1}\u{2}\u{12};T\u{1}50-+#333");
}
