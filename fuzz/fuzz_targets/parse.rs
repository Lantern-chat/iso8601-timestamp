#![no_main]

use libfuzzer_sys::fuzz_target;
use timestamp::Timestamp;

fuzz_target!(|data: &str| {
    Timestamp::parse(data);
});
