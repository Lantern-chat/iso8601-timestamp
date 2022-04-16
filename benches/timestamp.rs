#![allow(deprecated)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use iso8601_timestamp::Timestamp;

fn criterion_benchmark(c: &mut Criterion) {
    let offset = time::UtcOffset::from_hms(-4, 30, 0).unwrap();

    c.bench_function("format_iso8601", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format());
    });

    c.bench_function("format_iso8601_short", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_short());
    });

    c.bench_function("format_iso8601_offset", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_with_offset(offset));
    });

    c.bench_function("format_iso8601_nanoseconds", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_nanoseconds());
    });

    c.bench_function("format_is8601_slow", |b| {
        let ts = black_box(Utc::now().naive_utc());

        b.iter(|| format_naivedatetime(ts));
    });

    c.bench_function("format_time", |b| {
        let ts = black_box(time::OffsetDateTime::now_utc());

        b.iter(|| ts.format(&time::format_description::well_known::Rfc3339).unwrap());
    });

    c.bench_function("parse_iso8601_custom", |b| {
        let ts = black_box(Timestamp::now_utc().format());

        b.iter(|| Timestamp::parse(&ts));
    });

    c.bench_function("parse_iso8601_chrono", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        type T = DateTime<chrono::FixedOffset>;

        b.iter(|| T::parse_from_rfc3339(&ts).unwrap());
    });

    c.bench_function("parse_iso8601_time", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        use time::{format_description::well_known::Rfc3339, OffsetDateTime};

        b.iter(|| OffsetDateTime::parse(ts, &Rfc3339).unwrap());
    });

    c.bench_function("to_unix_timestamp_ms", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.to_unix_timestamp_ms());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

pub fn format_naivedatetime(dt: NaiveDateTime) -> String {
    DateTime::<Utc>::from_utc(dt, Utc).to_rfc3339_opts(SecondsFormat::Millis, true)
}
