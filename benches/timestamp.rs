#![allow(deprecated)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use iso8601_timestamp::{Timestamp, UtcOffset};

fn criterion_benchmark(c: &mut Criterion) {
    let offset = black_box(time::UtcOffset::from_hms(-4, 30, 0).unwrap());

    let mut format_group = c.benchmark_group("format");
    format_group.bench_function("iso8601", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format());
    });

    format_group.bench_function("iso8601_short", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_short());
    });

    format_group.bench_function("iso8601_offset", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_with_offset(offset));
    });

    format_group.bench_function("iso8601_nanoseconds", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_nanoseconds());
    });

    format_group.bench_function("is8601_slow", |b| {
        let ts = black_box(Utc::now().naive_utc());

        b.iter(|| format_naivedatetime(ts));
    });

    format_group.bench_function("time", |b| {
        let ts = black_box(time::OffsetDateTime::now_utc());

        b.iter(|| ts.format(&time::format_description::well_known::Rfc3339).unwrap());
    });

    format_group.finish();

    let mut parse_group = c.benchmark_group("parse");

    parse_group.bench_function("iso8601_custom", |b| {
        let ts = black_box(Timestamp::now_utc().format());
        let ts = black_box(ts.as_ref());

        b.iter(|| Timestamp::parse(ts));
    });

    parse_group.bench_function("iso8601_custom_cst", |b| {
        let ts = black_box(Timestamp::now_utc().format_with_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()));
        let ts = black_box(ts.as_ref());

        b.iter(|| Timestamp::parse(ts));
    });

    parse_group.bench_function("iso8601_chrono", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        type T = DateTime<chrono::FixedOffset>;

        b.iter(|| T::parse_from_rfc3339(ts).unwrap());
    });

    parse_group.bench_function("iso8601_time", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        use time::{format_description::well_known::Rfc3339, OffsetDateTime};

        b.iter(|| OffsetDateTime::parse(ts, &Rfc3339).unwrap());
    });

    parse_group.bench_function("iso8601_time_cst", |b| {
        let ts = black_box(Timestamp::now_utc().format_with_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()));
        let ts = black_box(ts.as_ref());

        use time::{format_description::well_known::Rfc3339, OffsetDateTime};

        b.iter(|| OffsetDateTime::parse(ts, &Rfc3339).unwrap());
    });

    parse_group.bench_function("iso8601_other", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        b.iter(|| iso8601::datetime(ts).unwrap());
    });

    parse_group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

pub fn format_naivedatetime(dt: NaiveDateTime) -> String {
    DateTime::<Utc>::from_utc(dt, Utc).to_rfc3339_opts(SecondsFormat::Millis, true)
}
