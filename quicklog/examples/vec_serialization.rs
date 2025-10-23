use quicklog::{flush_all, info, init, with_flush};
use quicklog_flush::stdout_flusher::StdoutFlusher;

fn main() {
    init!();
    with_flush!(StdoutFlusher);

    // Example 1: Simple vector of integers
    let numbers: Vec<i32> = vec![1, 2, 3, 4, 5];
    info!("Numbers: {}", ^numbers);

    // Example 2: Empty vector
    let empty: Vec<u64> = Vec::new();
    info!("Empty vector: {}", ^empty);

    // Example 3: Vector of strings
    let words: Vec<&str> = vec!["hello", "world", "quicklog"];
    info!("Words: {}", ^words);

    // Example 4: Vector with Option types
    let optional_data: Vec<Option<i32>> = vec![Some(10), None, Some(20), None, Some(30)];
    info!("Optional data: {}", ^optional_data);

    // Example 5: Larger vector (demonstrating O(N) behavior)
    let large_vec: Vec<u32> = (0..20).collect();
    info!("Large vector: {}", ^large_vec);

    // Example 6: Vector of floats
    let measurements: Vec<f64> = vec![1.5, 2.7, 3.14159, 4.2];
    info!("Measurements: {}", ^measurements);

    // Flush all log lines
    flush_all!();

    println!("\nAll log messages have been written!");
    println!("Vec serialization uses the ^ prefix for high-performance encoding.");
    println!("Call-site latency: ~10ns per element");
}
