use quicklog::{flush, info, init, with_flush};
use quicklog_flush::stdout_flusher::StdoutFlusher;

#[cfg(feature = "trace")]
use fastrace::prelude::*;
#[cfg(feature = "trace")]
use fastrace::collector::{Config, Reporter, SpanRecord};

#[cfg(feature = "trace")]
struct NoopReporter;

#[cfg(feature = "trace")]
impl Reporter for NoopReporter {
    fn report(&mut self, _spans: &[SpanRecord]) {}
}

fn main() {
    // Initialize quicklog
    init!();
    with_flush!(StdoutFlusher);

    #[cfg(feature = "trace")]
    {
        // Set up fastrace reporter (required for span context to work)
        fastrace::set_reporter(NoopReporter, Config::default());
    }

    // Test 1: Log without any trace context
    info!("Test 1: Logging without trace context");
    flush!();

    #[cfg(feature = "trace")]
    {
        // Test 2: Log with trace context
        let ctx = SpanContext::random();
        println!("Created trace context with trace_id: {:032x}", ctx.trace_id.0);
        let root = Span::root("example_operation", ctx);
        let _guard = root.set_local_parent();

        info!("Test 2: Logging with trace context");
        flush!();

        // Test 3: Multiple logs with same trace context
        info!("Test 3a: First log in traced operation");
        info!("Test 3b: Second log in traced operation");
        flush!();

        drop(_guard);

        // Test 4: Log after trace context is dropped
        info!("Test 4: Logging after trace context dropped");
        flush!();

        // Test 5: Nested spans
        let root = Span::root("outer_operation", SpanContext::random());
        let _guard = root.set_local_parent();

        info!("Test 5a: Outer operation");

        {
            let inner = Span::enter_with_local_parent("inner_operation");
            let _inner_guard = inner.set_local_parent();
            info!("Test 5b: Inner operation (note: same trace_id as outer)");
        }

        info!("Test 5c: Back to outer operation");
        flush!();
    }

    #[cfg(not(feature = "trace"))]
    {
        println!("\nNote: Trace feature is not enabled. Run with: cargo run --example trace_example --features trace");
    }
}
