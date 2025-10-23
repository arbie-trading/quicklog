use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use quanta::Instant;
use quicklog::{with_flush, SerializeSelective};
use quicklog_flush::noop_flusher::NoopFlusher;

macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {{
        quicklog::init!();
        with_flush!(NoopFlusher);

        $bencher.iter_custom(|iters| {
            let start = Instant::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = Instant::now() - start;

            quicklog::flush!();

            end
        })
    }};
}

// ============================================================================
// Benchmark 1: Vec<u32> (Copy type) - Clone vs Serialize
// ============================================================================

fn bench_vec_u32_10_clone(b: &mut Bencher) {
    let vec: Vec<u32> = black_box((0..10).collect());
    loop_with_cleanup!(b, quicklog::info!("data: {:?}", vec));
}

fn bench_vec_u32_10_serialize(b: &mut Bencher) {
    let vec: Vec<u32> = black_box((0..10).collect());
    loop_with_cleanup!(b, quicklog::info!("data: {}", ^vec));
}

fn bench_vec_u32_100_clone(b: &mut Bencher) {
    let vec: Vec<u32> = black_box((0..100).collect());
    loop_with_cleanup!(b, quicklog::info!("data: {:?}", vec));
}

fn bench_vec_u32_100_serialize(b: &mut Bencher) {
    let vec: Vec<u32> = black_box((0..100).collect());
    loop_with_cleanup!(b, quicklog::info!("data: {}", ^vec));
}

// ============================================================================
// Benchmark 2: Vec<String> (Heap-allocated type) - Clone vs Serialize
// ============================================================================

fn bench_vec_string_10_clone(b: &mut Bencher) {
    let vec: Vec<String> = black_box(
        vec!["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA", "META", "NVDA", "AMD", "INTC", "NFLX"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    loop_with_cleanup!(b, quicklog::info!("symbols: {:?}", vec));
}

fn bench_vec_string_10_serialize(b: &mut Bencher) {
    let vec: Vec<&str> = black_box(vec![
        "AAPL", "GOOGL", "MSFT", "AMZN", "TSLA", "META", "NVDA", "AMD", "INTC", "NFLX",
    ]);
    loop_with_cleanup!(b, quicklog::info!("symbols: {}", ^vec));
}

fn bench_vec_string_100_clone(b: &mut Bencher) {
    let vec: Vec<String> = black_box(
        (0..100)
            .map(|i| format!("SYMBOL_{:04}", i))
            .collect(),
    );
    loop_with_cleanup!(b, quicklog::info!("symbols: {:?}", vec));
}

fn bench_vec_string_100_serialize(b: &mut Bencher) {
    // Note: We use &str for serialization since String doesn't implement Serialize
    // This is actually MORE favorable for serialize, showing real-world usage
    let vec: Vec<&str> = black_box(vec![
        "SYMBOL_0000", "SYMBOL_0001", "SYMBOL_0002", "SYMBOL_0003", "SYMBOL_0004",
        "SYMBOL_0005", "SYMBOL_0006", "SYMBOL_0007", "SYMBOL_0008", "SYMBOL_0009",
        "SYMBOL_0010", "SYMBOL_0011", "SYMBOL_0012", "SYMBOL_0013", "SYMBOL_0014",
        "SYMBOL_0015", "SYMBOL_0016", "SYMBOL_0017", "SYMBOL_0018", "SYMBOL_0019",
        "SYMBOL_0020", "SYMBOL_0021", "SYMBOL_0022", "SYMBOL_0023", "SYMBOL_0024",
        "SYMBOL_0025", "SYMBOL_0026", "SYMBOL_0027", "SYMBOL_0028", "SYMBOL_0029",
        "SYMBOL_0030", "SYMBOL_0031", "SYMBOL_0032", "SYMBOL_0033", "SYMBOL_0034",
        "SYMBOL_0035", "SYMBOL_0036", "SYMBOL_0037", "SYMBOL_0038", "SYMBOL_0039",
        "SYMBOL_0040", "SYMBOL_0041", "SYMBOL_0042", "SYMBOL_0043", "SYMBOL_0044",
        "SYMBOL_0045", "SYMBOL_0046", "SYMBOL_0047", "SYMBOL_0048", "SYMBOL_0049",
        "SYMBOL_0050", "SYMBOL_0051", "SYMBOL_0052", "SYMBOL_0053", "SYMBOL_0054",
        "SYMBOL_0055", "SYMBOL_0056", "SYMBOL_0057", "SYMBOL_0058", "SYMBOL_0059",
        "SYMBOL_0060", "SYMBOL_0061", "SYMBOL_0062", "SYMBOL_0063", "SYMBOL_0064",
        "SYMBOL_0065", "SYMBOL_0066", "SYMBOL_0067", "SYMBOL_0068", "SYMBOL_0069",
        "SYMBOL_0070", "SYMBOL_0071", "SYMBOL_0072", "SYMBOL_0073", "SYMBOL_0074",
        "SYMBOL_0075", "SYMBOL_0076", "SYMBOL_0077", "SYMBOL_0078", "SYMBOL_0079",
        "SYMBOL_0080", "SYMBOL_0081", "SYMBOL_0082", "SYMBOL_0083", "SYMBOL_0084",
        "SYMBOL_0085", "SYMBOL_0086", "SYMBOL_0087", "SYMBOL_0088", "SYMBOL_0089",
        "SYMBOL_0090", "SYMBOL_0091", "SYMBOL_0092", "SYMBOL_0093", "SYMBOL_0094",
        "SYMBOL_0095", "SYMBOL_0096", "SYMBOL_0097", "SYMBOL_0098", "SYMBOL_0099",
    ]);
    loop_with_cleanup!(b, quicklog::info!("symbols: {}", ^vec));
}

// ============================================================================
// Benchmark 3: Vec<ComplexStruct> (Large struct) - Clone vs Serialize
// ============================================================================

// Use the same Order struct from selective_serialization_benchmark for consistency
#[derive(Clone, Debug, SerializeSelective)]
struct Order {
    #[serialize]
    oid: u64,
    #[serialize]
    cloid: Option<u64>,
    #[serialize]
    external_name: u32,
    #[serialize]
    side: u8,
    #[serialize]
    price: Option<f64>,
    #[serialize]
    size: f64,
    #[serialize]
    time: u64,
    #[serialize]
    time_received: u64,

    // Not serialized (8 additional fields that slow down cloning)
    order_type: u8,
    reduce_only: bool,
    time_in_force: u8,
    post_only: bool,
    status: u8,
    filled_size: f64,
    remaining_size: f64,
    avg_fill_price: Option<f64>,
}

impl Order {
    fn new(id: u64) -> Self {
        Self {
            oid: id,
            cloid: Some(id * 10),
            external_name: (id % 1000) as u32,
            side: (id % 2) as u8,
            price: Some(100.0 + (id as f64)),
            size: 10.0,
            time: 1234567890 + id,
            time_received: 1234567891 + id,
            order_type: 1,
            reduce_only: false,
            time_in_force: 0,
            post_only: false,
            status: 1,
            filled_size: 0.0,
            remaining_size: 10.0,
            avg_fill_price: None,
        }
    }
}

fn bench_vec_order_10_clone(b: &mut Bencher) {
    let vec: Vec<Order> = black_box((0..10).map(Order::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {:?}", vec));
}

fn bench_vec_order_10_serialize(b: &mut Bencher) {
    let vec: Vec<Order> = black_box((0..10).map(Order::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {}", ^vec));
}

fn bench_vec_order_100_clone(b: &mut Bencher) {
    let vec: Vec<Order> = black_box((0..100).map(Order::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {:?}", vec));
}

fn bench_vec_order_100_serialize(b: &mut Bencher) {
    let vec: Vec<Order> = black_box((0..100).map(Order::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {}", ^vec));
}

// ============================================================================
// Benchmark 4: Vec<SelectiveOrder> (Selective Serialization)
// ============================================================================

#[derive(Clone, Debug, SerializeSelective)]
struct SelectiveOrder {
    #[serialize] id: u64,
    #[serialize] price: f64,
    #[serialize] size: f64,
    #[serialize] timestamp: u64,

    // These fields NOT serialized (saves time)
    symbol: String,
    metadata: String,
    internal_notes: String,
    debug_data: Vec<u8>,
}

impl SelectiveOrder {
    fn new(id: u64) -> Self {
        Self {
            id,
            price: 100.0 + (id as f64),
            size: 10.0,
            timestamp: 1234567890 + id,
            symbol: format!("SYM{}", id),
            metadata: "Lorem ipsum dolor sit amet, consectetur adipiscing elit".to_string(),
            internal_notes: "Internal trading notes for order processing and reconciliation".to_string(),
            debug_data: vec![0u8; 100],
        }
    }
}

fn bench_vec_selective_order_10_clone(b: &mut Bencher) {
    let vec: Vec<SelectiveOrder> = black_box((0..10).map(SelectiveOrder::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {:?}", vec));
}

fn bench_vec_selective_order_10_serialize(b: &mut Bencher) {
    let vec: Vec<SelectiveOrder> = black_box((0..10).map(SelectiveOrder::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {}", ^vec));
}

fn bench_vec_selective_order_100_clone(b: &mut Bencher) {
    let vec: Vec<SelectiveOrder> = black_box((0..100).map(SelectiveOrder::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {:?}", vec));
}

fn bench_vec_selective_order_100_serialize(b: &mut Bencher) {
    let vec: Vec<SelectiveOrder> = black_box((0..100).map(SelectiveOrder::new).collect());
    loop_with_cleanup!(b, quicklog::info!("orders: {}", ^vec));
}

// ============================================================================
// Benchmark 5: Vec<f64> (Copy type, floating point)
// ============================================================================

fn bench_vec_f64_50_clone(b: &mut Bencher) {
    let vec: Vec<f64> = black_box((0..50).map(|i| i as f64 * 1.5).collect());
    loop_with_cleanup!(b, quicklog::info!("prices: {:?}", vec));
}

fn bench_vec_f64_50_serialize(b: &mut Bencher) {
    let vec: Vec<f64> = black_box((0..50).map(|i| i as f64 * 1.5).collect());
    loop_with_cleanup!(b, quicklog::info!("prices: {}", ^vec));
}

// ============================================================================
// Benchmark 6: Vec<Option<T>> (Nested type)
// ============================================================================

fn bench_vec_option_i32_20_clone(b: &mut Bencher) {
    let vec: Vec<Option<i32>> = black_box(
        (0..20).map(|i| if i % 3 == 0 { None } else { Some(i) }).collect()
    );
    loop_with_cleanup!(b, quicklog::info!("data: {:?}", vec));
}

fn bench_vec_option_i32_20_serialize(b: &mut Bencher) {
    let vec: Vec<Option<i32>> = black_box(
        (0..20).map(|i| if i % 3 == 0 { None } else { Some(i) }).collect()
    );
    loop_with_cleanup!(b, quicklog::info!("data: {}", ^vec));
}

// ============================================================================
// Criterion Configuration
// ============================================================================

fn bench_vec_primitives(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec<Primitives>");
    group.bench_function("Vec<u32> 10 elements (clone)", bench_vec_u32_10_clone);
    group.bench_function("Vec<u32> 10 elements (serialize)", bench_vec_u32_10_serialize);
    group.bench_function("Vec<u32> 100 elements (clone)", bench_vec_u32_100_clone);
    group.bench_function("Vec<u32> 100 elements (serialize)", bench_vec_u32_100_serialize);
    group.bench_function("Vec<f64> 50 elements (clone)", bench_vec_f64_50_clone);
    group.bench_function("Vec<f64> 50 elements (serialize)", bench_vec_f64_50_serialize);
    group.finish();
}

fn bench_vec_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec<String>");
    group.bench_function("Vec<String> 10 elements (clone)", bench_vec_string_10_clone);
    group.bench_function("Vec<String> 10 elements (serialize)", bench_vec_string_10_serialize);
    group.bench_function("Vec<String> 100 elements (clone)", bench_vec_string_100_clone);
    group.bench_function("Vec<String> 100 elements (serialize)", bench_vec_string_100_serialize);
    group.finish();
}

fn bench_vec_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec<ComplexStruct>");
    group.bench_function("Vec<Order> 10 elements (clone)", bench_vec_order_10_clone);
    group.bench_function("Vec<Order> 10 elements (serialize)", bench_vec_order_10_serialize);
    group.bench_function("Vec<Order> 100 elements (clone)", bench_vec_order_100_clone);
    group.bench_function("Vec<Order> 100 elements (serialize)", bench_vec_order_100_serialize);
    group.finish();
}

fn bench_vec_selective(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec<SelectiveSerialize>");
    group.bench_function("Vec<SelectiveOrder> 10 (clone)", bench_vec_selective_order_10_clone);
    group.bench_function("Vec<SelectiveOrder> 10 (serialize)", bench_vec_selective_order_10_serialize);
    group.bench_function("Vec<SelectiveOrder> 100 (clone)", bench_vec_selective_order_100_clone);
    group.bench_function("Vec<SelectiveOrder> 100 (serialize)", bench_vec_selective_order_100_serialize);
    group.finish();
}

fn bench_vec_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec<Option<T>>");
    group.bench_function("Vec<Option<i32>> 20 (clone)", bench_vec_option_i32_20_clone);
    group.bench_function("Vec<Option<i32>> 20 (serialize)", bench_vec_option_i32_20_serialize);
    group.finish();
}

criterion_group!(
    benches,
    bench_vec_primitives,
    bench_vec_strings,
    bench_vec_complex,
    bench_vec_selective,
    bench_vec_nested
);
criterion_main!(benches);
