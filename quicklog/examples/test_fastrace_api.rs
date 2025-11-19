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
    #[cfg(feature = "trace")]
    {
        println!("Testing fastrace API...");

        // Set up a reporter (even if it's just a noop)
        fastrace::set_reporter(NoopReporter, Config::default());

        // Test 1: Check current_local_parent without any span
        let ctx1 = SpanContext::current_local_parent();
        println!("Context before setting span: {:?}", ctx1);

        // Test 2: Set a span and check again with set_local_parent
        let ctx = SpanContext::random();
        println!("\nTest with set_local_parent:");
        println!("Created context - trace_id.0 = {:032x}", ctx.trace_id.0);

        let root = Span::root("test", ctx);
        let _guard = root.set_local_parent();

        let ctx2 = SpanContext::current_local_parent();
        println!("Context after set_local_parent: {:?}", ctx2);

        drop(_guard);

        // Test 3: Following the exact pattern from fastrace example
        println!("\nTest following fastrace example pattern:");

        let ctx3 = SpanContext::random();
        println!("Created context - trace_id.0 = {:032x}", ctx3.trace_id.0);

        let root2 = Span::root("test2", ctx3);
        let _root_guard = root2.set_local_parent();

        // Must enter a LocalSpan for current_local_parent to work!
        let _local_span = LocalSpan::enter_with_local_parent("local_test");

        let ctx4 = SpanContext::current_local_parent();
        println!("Context after creating LocalSpan: {:?}", ctx4);

        if let Some(c) = ctx4 {
            println!("SUCCESS! Trace ID from context: {:032x}", c.trace_id.0);
        }
    }

    #[cfg(not(feature = "trace"))]
    {
        println!("Trace feature not enabled");
    }
}
