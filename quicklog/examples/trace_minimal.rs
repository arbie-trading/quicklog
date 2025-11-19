// This example demonstrates that users don't need to add fastrace
// as a direct dependency - quicklog re-exports what's needed

use quicklog::{flush, info, init, with_flush};
use quicklog_flush::stdout_flusher::StdoutFlusher;

fn main() {
    init!();
    with_flush!(StdoutFlusher);

    // This works without fastrace in dependencies!
    info!("Log message without trace");
    flush!();

    #[cfg(feature = "trace")]
    {
        // Even when trace feature is enabled, we can use the re-exported types
        use quicklog::__FastraceSpanContext;
        use fastrace::prelude::*;  // This is only in the example
        use fastrace::collector::{Config, Reporter, SpanRecord};

        struct NoopReporter;
        impl Reporter for NoopReporter {
            fn report(&mut self, _spans: &[SpanRecord]) {}
        }

        fastrace::set_reporter(NoopReporter, Config::default());

        let ctx = __FastraceSpanContext::random();
        let root = Span::root("test", ctx);
        let _guard = root.set_local_parent();

        info!("Log message with trace context");
        flush!();
    }

    println!("\nNote: This example doesn't require fastrace as a dependency!");
    println!("The trace feature is enabled in quicklog, which re-exports the necessary types.");
}
