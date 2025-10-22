use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use quicklog::serialize::Serialize;
use quicklog::{SerializeSelective, FixedSizeSerialize};
use std::fmt::{Debug, Display};

// Test structs for benchmarking

#[derive(Debug, SerializeSelective)]
pub struct Order {
    #[serialize]
    pub oid: u64,
    #[serialize]
    pub cloid: Option<u64>,
    #[serialize]
    pub external_name: u32,
    #[serialize]
    pub side: u8,
    #[serialize]
    pub price: Option<f64>,
    #[serialize]
    pub size: f64,
    #[serialize]
    pub time: u64,
    #[serialize]
    pub time_received: u64,

    // Not serialized (8 additional fields)
    pub order_type: u8,
    pub reduce_only: bool,
    pub time_in_force: u8,
    pub post_only: bool,
    pub status: u8,
    pub filled_size: f64,
    pub remaining_size: f64,
    pub avg_fill_price: Option<f64>,
}

#[derive(Debug, SerializeSelective)]
pub struct Position {
    #[serialize]
    pub position_id: u64,
    #[serialize]
    pub symbol: u32,
    #[serialize]
    pub size: f64,
    #[serialize]
    pub avg_price: f64,
    #[serialize]
    pub unrealized_pnl_cents: i64,
    #[serialize]
    pub realized_pnl: f64,
    // Not serialized fields
    pub last_updated: u64,
    pub margin_used: f64,
    pub maintenance_margin: f64,
}

// Custom types implementing FixedSizeSerialize for testing trait-based approach
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomId(u64);

impl FixedSizeSerialize<8> for CustomId {
    fn to_le_bytes(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
}

impl Display for CustomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomId({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CustomSide {
    Buy = 0,
    Sell = 1,
}

impl FixedSizeSerialize<1> for CustomSide {
    fn to_le_bytes(&self) -> [u8; 1] {
        [*self as u8]
    }

    fn from_le_bytes(bytes: [u8; 1]) -> Self {
        match bytes[0] {
            0 => CustomSide::Buy,
            1 => CustomSide::Sell,
            _ => panic!("Invalid CustomSide: {}", bytes[0]),
        }
    }
}

impl Display for CustomSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomSide::Buy => write!(f, "Buy"),
            CustomSide::Sell => write!(f, "Sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomPrice(f64);

impl FixedSizeSerialize<8> for CustomPrice {
    fn to_le_bytes(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self(f64::from_le_bytes(bytes))
    }
}

impl Display for CustomPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomPrice({})", self.0)
    }
}

// Custom Order struct using FixedSizeSerialize types
#[derive(Debug, SerializeSelective)]
pub struct CustomOrder {
    #[serialize]
    pub oid: CustomId,
    #[serialize]
    pub cloid: Option<CustomId>,
    #[serialize]
    pub side: CustomSide,
    #[serialize]
    pub price: Option<CustomPrice>,
    #[serialize]
    pub size: f64,
    #[serialize]
    pub time: u64,
    #[serialize]
    pub time_received: u64,

    // Not serialized
    pub order_type: u8,
    pub status: u8,
    pub filled_size: f64,
}

impl Display for CustomOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oid={} cloid={:?} side={} price={:?} size={} time={} time_received={}",
               self.oid, self.cloid, self.side, self.price, self.size, self.time, self.time_received)
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oid={} cloid={:?} external_name={} side={} price={:?} size={} time={} time_received={}",
               self.oid, self.cloid, self.external_name, self.side, self.price, self.size, self.time, self.time_received)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "position_id={} symbol={} size={} avg_price={} unrealized_pnl_cents={} realized_pnl={}",
            self.position_id,
            self.symbol,
            self.size,
            self.avg_price,
            self.unrealized_pnl_cents,
            self.realized_pnl
        )
    }
}

fn create_order() -> Order {
    Order {
        oid: 12345678,
        cloid: Some(87654321),
        external_name: 1001,
        side: 0,
        price: Some(45000.50),
        size: 2.5,
        time: 1642781234,
        time_received: 1642781235,
        order_type: 1,
        reduce_only: false,
        time_in_force: 0,
        post_only: false,
        status: 0,
        filled_size: 0.0,
        remaining_size: 2.5,
        avg_fill_price: None,
    }
}

fn create_position() -> Position {
    Position {
        position_id: 98765,
        symbol: 1001,
        size: 1.8,
        avg_price: 44500.0,
        unrealized_pnl_cents: 90000,
        realized_pnl: 150.0,
        last_updated: 1642781300,
        margin_used: 8900.0,
        maintenance_margin: 4450.0,
    }
}

fn create_custom_order() -> CustomOrder {
    CustomOrder {
        oid: CustomId(12345678),
        cloid: Some(CustomId(87654321)),
        side: CustomSide::Buy,
        price: Some(CustomPrice(45000.50)),
        size: 2.5,
        time: 1642781234,
        time_received: 1642781235,
        order_type: 1,
        status: 0,
        filled_size: 0.0,
    }
}

// Benchmarking functions

fn bench_order_selective_serialize_encode_only(c: &mut Criterion) {
    let order = create_order();
    let mut buf = [0u8; 128];

    c.bench_function("order_selective_serialize_encode_only", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(order.encode(buffer));
            black_box(store);
        });
    });
}

fn bench_order_selective_serialize_full_cycle(c: &mut Criterion) {
    let order = create_order();
    let mut buf = [0u8; 128];

    c.bench_function("order_selective_serialize_full_cycle", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(order.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });
}

fn bench_order_debug_format(c: &mut Criterion) {
    let order = create_order();

    c.bench_function("order_debug_format", |b| {
        b.iter(|| {
            let result = black_box(format!("{:?}", order));
            black_box(result);
        });
    });
}

fn bench_order_display_format(c: &mut Criterion) {
    let order = create_order();

    c.bench_function("order_display_format", |b| {
        b.iter(|| {
            let result = black_box(format!("{}", order));
            black_box(result);
        });
    });
}

fn bench_order_manual_format(c: &mut Criterion) {
    let order = create_order();

    c.bench_function("order_manual_format", |b| {
        b.iter(|| {
            let result = black_box(format!(
                "oid={} cloid={:?} external_name={} side={} price={:?} size={} time={} time_received={}",
                order.oid, order.cloid, order.external_name, order.side,
                order.price, order.size, order.time, order.time_received
            ));
            black_box(result);
        });
    });
}

fn bench_position_declarative_macro(c: &mut Criterion) {
    let position = create_position();
    let mut buf = [0u8; 128];

    c.bench_function("position_declarative_macro_full_cycle", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(position.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });
}

fn bench_buffer_size_calculation(c: &mut Criterion) {
    let order = create_order();

    c.bench_function("buffer_size_calculation", |b| {
        b.iter(|| {
            let size = black_box(order.buffer_size_required());
            black_box(size);
        });
    });
}

fn bench_throughput_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_comparison");

    // Set throughput measurement
    group.throughput(Throughput::Elements(1));

    let order = create_order();
    let mut buf = [0u8; 128];

    // Selective serialization
    group.bench_function("selective_serialize", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(order.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });

    // Debug formatting
    group.bench_function("debug_format", |b| {
        b.iter(|| {
            let result = black_box(format!("{:?}", order));
            black_box(result);
        });
    });

    // Display formatting
    group.bench_function("display_format", |b| {
        b.iter(|| {
            let result = black_box(format!("{}", order));
            black_box(result);
        });
    });

    // Manual format
    group.bench_function("manual_format", |b| {
        b.iter(|| {
            let result = black_box(format!(
                "oid={} cloid={:?} external_name={} side={} price={:?} size={} time={} time_received={}",
                order.oid, order.cloid, order.external_name, order.side,
                order.price, order.size, order.time, order.time_received
            ));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");

    let orders: Vec<Order> = (0..1000)
        .map(|i| {
            let mut order = create_order();
            order.oid = i;
            order
        })
        .collect();

    let mut buf = [0u8; 128000]; // Large buffer for batch processing

    group.throughput(Throughput::Elements(orders.len() as u64));

    // Batch selective serialization
    group.bench_function("batch_selective_serialize", |b| {
        b.iter(|| {
            let mut buffer = black_box(&mut buf[..]);
            let mut results = Vec::with_capacity(orders.len());

            for order in &orders {
                let (store, remaining) = order.encode(buffer);
                results.push(store.as_string());
                buffer = remaining;
            }

            black_box(results);
        });
    });

    // Batch debug formatting
    group.bench_function("batch_debug_format", |b| {
        b.iter(|| {
            let results: Vec<String> = orders.iter().map(|order| format!("{:?}", order)).collect();
            black_box(results);
        });
    });

    // Batch display formatting
    group.bench_function("batch_display_format", |b| {
        b.iter(|| {
            let results: Vec<String> = orders.iter().map(|order| format!("{}", order)).collect();
            black_box(results);
        });
    });

    group.finish();
}

fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    let order = create_order();

    // Compare buffer sizes required
    group.bench_function("selective_serialize_buffer_size", |b| {
        b.iter(|| {
            let size = black_box(order.buffer_size_required());
            black_box(size);
        });
    });

    // Compare actual memory usage patterns
    let orders: Vec<Order> = (0..100)
        .map(|i| {
            let mut order = create_order();
            order.oid = i;
            order
        })
        .collect();

    group.bench_function("selective_serialize_memory_pattern", |b| {
        b.iter(|| {
            let mut buf = vec![0u8; orders.len() * 64]; // Pre-calculated optimal size
            let mut offset = 0;

            for order in &orders {
                let required_size = order.buffer_size_required();
                let (store, _) = order.encode(&mut buf[offset..offset + required_size]);
                let _result = store.as_string();
                offset += required_size;
            }

            black_box(buf);
        });
    });

    group.bench_function("string_format_memory_pattern", |b| {
        b.iter(|| {
            let results: Vec<String> = orders.iter().map(|order| format!("{}", order)).collect();
            black_box(results);
        });
    });

    group.finish();
}

fn bench_high_frequency_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_frequency_simulation");
    group.sample_size(1000);

    // Simulate high-frequency trading scenario
    let order = create_order();
    let mut buf = [0u8; 128];

    group.bench_function("hft_selective_serialize", |b| {
        b.iter(|| {
            // Simulate the hot path in HFT systems
            let buffer = black_box(&mut buf);

            // Step 1: Calculate buffer size (compile-time optimizable)
            let _size = order.buffer_size_required();

            // Step 2: Encode (zero allocation)
            let (store, _) = order.encode(buffer);

            // Step 3: Defer string formatting (happens at flush time)
            black_box(store);

            // Note: as_string() would typically be called later during flush
        });
    });

    group.bench_function("hft_immediate_format", |b| {
        b.iter(|| {
            // Simulate immediate formatting (traditional approach)
            let result = format!(
                "oid={} cloid={:?} external_name={} side={} price={:?} size={} time={} time_received={}",
                order.oid, order.cloid, order.external_name, order.side,
                order.price, order.size, order.time, order.time_received
            );
            black_box(result);
        });
    });

    group.finish();
}

fn bench_option_handling_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("option_handling");

    // Order with Some values
    let order_with_values = create_order();

    // Order with None values
    let mut order_with_nones = create_order();
    order_with_nones.cloid = None;
    order_with_nones.price = None;

    let mut buf = [0u8; 128];

    group.bench_function("selective_serialize_with_some_values", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(order_with_values.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });

    group.bench_function("selective_serialize_with_none_values", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(order_with_nones.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });

    group.bench_function("format_with_some_values", |b| {
        b.iter(|| {
            let result = black_box(format!("{}", order_with_values));
            black_box(result);
        });
    });

    group.bench_function("format_with_none_values", |b| {
        b.iter(|| {
            let result = black_box(format!("{}", order_with_nones));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_custom_types_selective_serialize(c: &mut Criterion) {
    let custom_order = create_custom_order();
    let mut buf = [0u8; 128];

    c.bench_function("custom_types_selective_serialize_full_cycle", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(custom_order.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });
}

fn bench_custom_types_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("custom_types_format_comparison");

    let custom_order = create_custom_order();
    let mut buf = [0u8; 128];

    // Selective serialization with custom types
    group.bench_function("custom_types_selective", |b| {
        b.iter(|| {
            let buffer = black_box(&mut buf);
            let (store, _) = black_box(custom_order.encode(buffer));
            let result = black_box(store.as_string());
            black_box(result);
        });
    });

    // Debug formatting with custom types
    group.bench_function("custom_types_debug", |b| {
        b.iter(|| {
            let result = black_box(format!("{:?}", custom_order));
            black_box(result);
        });
    });

    // Display formatting with custom types
    group.bench_function("custom_types_display", |b| {
        b.iter(|| {
            let result = black_box(format!("{}", custom_order));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fixed_size_serialize_trait_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("trait_overhead_comparison");

    let custom_id = CustomId(12345678);
    let primitive_id = 12345678u64;

    // Direct primitive to_le_bytes
    group.bench_function("primitive_to_le_bytes", |b| {
        b.iter(|| {
            let bytes = black_box(primitive_id.to_le_bytes());
            black_box(bytes);
        });
    });

    // FixedSizeSerialize trait method
    group.bench_function("trait_to_le_bytes", |b| {
        b.iter(|| {
            let bytes = black_box(custom_id.to_le_bytes());
            black_box(bytes);
        });
    });

    // Note: FixedSizeSerialize is not dyn-compatible due to const generics and associated const
    // This is actually a feature - it forces compile-time dispatch for maximum performance

    group.finish();
}

fn bench_buffer_size_calculation_trait_vs_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_size_calculation");

    let order = create_order();
    let custom_order = create_custom_order();

    // Original approach (primitives only)
    group.bench_function("primitives_buffer_size", |b| {
        b.iter(|| {
            let size = black_box(order.buffer_size_required());
            black_box(size);
        });
    });

    // New approach (FixedSizeSerialize trait)
    group.bench_function("trait_buffer_size", |b| {
        b.iter(|| {
            let size = black_box(custom_order.buffer_size_required());
            black_box(size);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_order_selective_serialize_encode_only,
    bench_order_selective_serialize_full_cycle,
    bench_order_debug_format,
    bench_order_display_format,
    bench_order_manual_format,
    bench_position_declarative_macro,
    bench_buffer_size_calculation,
    bench_throughput_comparison,
    bench_batch_processing,
    bench_memory_efficiency,
    bench_high_frequency_simulation,
    bench_option_handling_overhead,
    bench_custom_types_selective_serialize,
    bench_custom_types_format_comparison,
    bench_fixed_size_serialize_trait_overhead,
    bench_buffer_size_calculation_trait_vs_primitive
);

criterion_main!(benches);
