use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use quanta::Instant;
use quicklog::with_flush;
use quicklog_flush::noop_flusher::NoopFlusher;

macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, { quicklog::flush!() })
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {{
        quicklog::init!();

        $bencher.iter_custom(|iters| {
            let start = Instant::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = Instant::now() - start;

            $cleanup_f;

            end
        })
    }};
}

#[derive(Debug, Clone, Copy)]
struct Order {
    id: u64,
    price: f64,
    size: f64,
}

// Benchmark without trace feature (baseline)
fn bench_no_trace(b: &mut Bencher) {
    let order = black_box(Order {
        id: 12345,
        price: 100.5,
        size: 10.0,
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Order created: {:?}", order));
}

// Benchmark with trace feature but no active span
#[cfg(feature = "trace")]
fn bench_trace_no_span(b: &mut Bencher) {
    let order = black_box(Order {
        id: 12345,
        price: 100.5,
        size: 10.0,
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Order created: {:?}", order));
}

// Benchmark with trace feature and active span
#[cfg(feature = "trace")]
fn bench_trace_with_span(b: &mut Bencher) {
    use fastrace::prelude::*;

    let order = black_box(Order {
        id: 12345,
        price: 100.5,
        size: 10.0,
    });
    with_flush!(NoopFlusher);

    // Create a root span context
    let root = Span::root("test_span", SpanContext::random());
    let _guard = root.set_local_parent();

    loop_with_cleanup!(b, quicklog::info!("Order created: {:?}", order));
}

// Benchmark with primitives (no trace)
fn bench_primitives_no_trace(b: &mut Bencher) {
    let id = black_box(12345u64);
    let price = black_box(100.5f64);
    let size = black_box(10.0f64);
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Order: id={}, price={}, size={}", id, price, size));
}

// Benchmark with primitives (trace enabled, no span)
#[cfg(feature = "trace")]
fn bench_primitives_trace_no_span(b: &mut Bencher) {
    let id = black_box(12345u64);
    let price = black_box(100.5f64);
    let size = black_box(10.0f64);
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Order: id={}, price={}, size={}", id, price, size));
}

// Benchmark with primitives (trace enabled, with span)
#[cfg(feature = "trace")]
fn bench_primitives_trace_with_span(b: &mut Bencher) {
    use fastrace::prelude::*;

    let id = black_box(12345u64);
    let price = black_box(100.5f64);
    let size = black_box(10.0f64);
    with_flush!(NoopFlusher);

    let root = Span::root("test_span", SpanContext::random());
    let _guard = root.set_local_parent();

    loop_with_cleanup!(b, quicklog::info!("Order: id={}, price={}, size={}", id, price, size));
}

fn bench_trace_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("Trace Feature Overhead");

    // Baseline tests (no trace feature)
    group.bench_function("struct - no trace feature", bench_no_trace);
    group.bench_function("primitives - no trace feature", bench_primitives_no_trace);

    #[cfg(feature = "trace")]
    {
        // Trace feature enabled, no active span
        group.bench_function("struct - trace enabled, no span", bench_trace_no_span);
        group.bench_function("primitives - trace enabled, no span", bench_primitives_trace_no_span);

        // Trace feature enabled, with active span
        group.bench_function("struct - trace enabled, with span", bench_trace_with_span);
        group.bench_function("primitives - trace enabled, with span", bench_primitives_trace_with_span);
    }

    group.finish();
}

criterion_group!(benches, bench_trace_overhead);
criterion_main!(benches);
