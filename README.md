# quicklog

Ultra-fast single-threaded logging framework with **selective field serialization**, **generic type support**, and **optimized collection logging**. Achieves **111x performance improvement** over Debug formatting for complex structs, **2-85× faster than cloning** for `Vec<T>`, and almost 200x faster than `tracing` and `delog` for large structs.

Supports standard logging macros like `trace!`, `debug!`, `info!`, `warn!` and `error!`. Fully supports generic types with zero runtime overhead.

Flushing is deferred until `flush!()` macro is called.

## Objectives

- Deferred formatting
- Deferred I/O
- Low call site latency

While `tracing` is a popular library for event tracing and are really good at what they do, `quicklog` is optimized for low callsite latency, paying the cost of formatting and I/O on a separate thread, away from the hot path.

## Installation

#### Install using Cargo

```bash
cargo add quicklog
```

#### Add to Cargo.toml

```toml
# Cargo.toml
[dependencies]
quicklog = "0.2"
# ...
```

## Usage

### Quick Start

```rust
use quicklog::{info, init, flush};

fn main() {
    // Initialize required resources
    init!();

    // Simple logging
    info!("hello world");

    // Logging primitives (no prefix is fastest)
    let price = 100.5;
    info!("Price: {}", price);  // ~1-2ns

    // Logging structs (use ^ prefix for best performance)
    let order = Order { id: 123, size: 10.0 };
    info!("Order: {}", ^order);  // ~5-10ns with SerializeSelective

    // Flush all log messages
    flush!();
}
```

### Logging Syntax and Performance

Quicklog provides multiple ways to log values with different performance characteristics:

#### Format String Arguments

You can use prefixes **directly in format arguments** for fine-grained control over performance:

```rust
// For primitives: no prefix is fastest (~1-2ns)
info!("Order: id={}, price={}, size={}", order_id, price, size);

// For structs: use ^ prefix (~5-10ns with selective serialization)
info!("Order created: {}", ^order);

// Mix strategies based on type
info!("Fill: order={}, price={}, qty={}", ^order, price, qty);

// Unprefixed structs (clone entire struct: ~28-104ns)
info!("Order: {:?}", order_struct);
```

**What happens:**
- `^arg` → Serializes to bytes at callsite (~5-10ns for struct fields, requires `Serialize` trait)
- `arg` (no prefix) → Clones/copies and defers formatting to flush time (~1-2ns for primitives, ~28-104ns for structs)
- `%arg` → Eagerly formats with Display at callsite (~600ns)
- `?arg` → Eagerly formats with Debug at callsite (~600ns)

**Important:** For primitive types (`u64`, `f64`, `i32`, etc.), the unprefixed version is fastest since they're `Copy`. The `^` prefix is only beneficial for structs with selective serialization.

#### Structured Field Syntax

Alternatively, you can use structured fields that get appended after the message:

```rust
info!(?some_var, %other_var, "message");
// Output: "message some_var=<debug> other_var=<display>"

info!(^serialized, "message");
// Output: "message serialized=<value>"
```

#### Performance Comparison

| Syntax | Call Site Latency | When to Use |
|--------|------------------|-------------|
| `"text {}", var` (primitive) | **~1-2ns** | Primitives (`u64`, `f64`, etc.) - fastest option |
| `"text {}", ^var` (struct) | **~5-10ns** | Structs with `Serialize` - selective serialization |
| `"text {}", var` (struct) | **~28-104ns** | Structs without `Serialize` - clones entire struct |
| `"text {}", %var` or `?var` | **~600ns** | Debugging, non-critical paths - eager formatting |

**Recommendation:**
- **Use no prefix** for: primitives (`u32`, `f64`, `i32`, etc.) → **fastest at ~1-2ns**
- **Use `^` (Serialize)** for:
  - Structs with `#[derive(SerializeSelective)]` → **6-111× faster than clone**
  - `Vec<String>` or `Vec<ComplexStruct>` → **2-85× faster than clone**
  - Heap-allocated types in collections
- **Avoid `^` for**: `Vec` of primitives (slower than clone for 50+ elements)
- **Use `%`/`?`** only when: debugging, non-critical paths, immediate formatting needed

#### Example: Optimal Usage

```rust
// For structs: use ^ prefix
info!(
    "Received fill for position {}",
    ^filled_position,    // Struct with Serialize: ~5-10ns
);

// For primitives: no prefix is fastest
info!(
    "Order: id={}, price={}, size={}",
    order_id,    // Primitive u64: ~1-2ns
    price,       // Primitive f64: ~1-2ns
    size,        // Primitive f64: ~1-2ns
);

// Mixed: use appropriate strategy per field
info!(
    "Fill: position={}, price={}, qty={}",
    ^position,   // Struct: ~5-10ns
    price,       // Primitive: ~1-2ns
    qty,         // Primitive: ~1-2ns
);
```


### Logging Collections with High Performance

Quicklog automatically serializes common collections like `Vec<T>` and `Option<T>`:

```rust
use quicklog::{info, init, flush_all};

fn main() {
    init!();

    // Vec of strings (2.6× faster than cloning)
    let symbols: Vec<&str> = vec!["AAPL", "GOOGL", "MSFT"];
    info!("Symbols: {}", ^symbols);  // Use ^ for heap types

    // Vec of primitives (no prefix is faster)
    let prices: Vec<f64> = vec![100.5, 101.2, 99.8, 102.1];
    info!("Prices: {:?}", prices);  // No ^ for primitives

    // Vec of complex structs (6-7× faster with ^)
    let orders: Vec<Order> = get_orders();
    info!("Orders: {}", ^orders);  // Use ^ for structs

    // Nested collections
    let data: Vec<Option<i32>> = vec![Some(10), None, Some(20)];
    info!("Data: {}", ^data);  // Output: [Some(10), None, Some(20)]

    flush_all!();
}
```

**Performance benefits for Vec (use `^` prefix):**
- **Vec<&str> or Vec<String>**: 2-3× faster than clone (avoids heap allocations)
- **Vec<ComplexStruct>**: 6-7× faster with selective serialization
- **Vec<SelectiveOrder>**: Up to 85× faster for large structs

**When NOT to use `^`:**
- **Vec of primitives** (`Vec<u32>`, `Vec<f64>`): Clone is faster for 50+ elements

See [Vec benchmark results](VEC_BENCHMARK_RESULTS.md) for detailed performance analysis.

### Implementing Custom `Serialize`

For custom types, implement the `Serialize` trait to control exactly what gets serialized:

```rust
use quicklog::serialize::{Serialize, Store};

struct SomeStruct {
    num: i64
}

impl Serialize for SomeStruct {
   fn encode(&self, write_buf: &'static mut [u8]) -> Store { /* some impl */ }
   fn buffer_size_required(&self) -> usize { /* some impl */ }
}

fn main() {
    let s = SomeStruct { num: 1_000_000 };
    info!("some struct: {}", ^s);
}
```

For complete examples, refer to:
- `quicklog/examples/vec_serialization.rs` - Vec examples
- `quicklog/benches/logger_benchmark.rs` - Custom implementations

## High-Performance Selective Serialization

For maximum performance, quicklog provides **selective field serialization** that allows you to serialize only specific fields from large structs, achieving **111x faster encoding** than Debug formatting.

### Using FixedSizeSerialize for Custom Types

For maximum convenience, quicklog provides macros that automatically implement the `FixedSizeSerialize` trait for common patterns:

#### Easy Implementation with Macros

```rust
use quicklog::{impl_fixed_size_serialize_newtype, impl_fixed_size_serialize_enum};

// Simple wrapper types
pub struct OrderId(u64);
impl_fixed_size_serialize_newtype!(OrderId, u64, 8);

pub struct Price(f64);
impl_fixed_size_serialize_newtype!(Price, f64, 8);

pub struct Timestamp(u64);
impl_fixed_size_serialize_newtype!(Timestamp, u64, 8);

// Enums with discriminants
#[repr(u8)]
pub enum Side { Buy = 0, Sell = 1 }
impl_fixed_size_serialize_enum!(Side, Buy = 0, Sell = 1);
```

#### Manual Implementation (for complex cases)

For types requiring custom serialization logic, implement the trait manually:

```rust
use quicklog::FixedSizeSerialize;

// Complex type with custom serialization
pub struct MarketId([u8; 16]); // Fixed-size string with padding

impl FixedSizeSerialize<16> for MarketId {
    fn to_le_bytes(&self) -> [u8; 16] {
        self.0  // Already in correct format
    }
    fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}
```

### Selective Field Serialization

Use `#[derive(SerializeSelective)]` to automatically generate optimized serialization for only the fields you need:

```rust
#[derive(SerializeSelective)]
pub struct Order {
    // These fields will be serialized (high performance: ~5-10ns total)
    #[serialize] pub id: OrderId,
    #[serialize] pub side: Side,
    #[serialize] pub price: Option<f64>,
    #[serialize] pub size: f64,
    #[serialize] pub timestamp: u64,

    // These fields are excluded (reduces overhead by 50-60%)
    pub internal_id: String,
    pub metadata: HashMap<String, String>,
    pub debug_info: Vec<String>,
}

fn main() {
    init!();

    let order = Order { /* ... */ };

    // Ultra-fast selective serialization (~5-10ns vs ~600ns for Debug)
    info!(order = ^order, "Order created");

    flush!();
}
```

### Generic Type Support

`#[derive(SerializeSelective)]` fully supports generic type parameters:

```rust
use quicklog::SerializeSelective;

// Generic struct with trait bounds
#[derive(SerializeSelective)]
pub struct Order<T>
where
    T: quicklog::serialize::FixedSizeSerialize<8> + std::fmt::Display,
{
    #[serialize] pub id: T,           // Generic type in serialized field
    #[serialize] pub price: f64,
    #[serialize] pub size: Option<u64>,

    pub metadata: String,              // Not serialized
}

// Generic type NOT in serialized fields doesn't need trait bounds
#[derive(SerializeSelective)]
pub struct Container<T> {
    #[serialize] pub count: u64,

    pub data: T,  // Generic, not serialized - no trait bounds needed!
}

// Works with multiple generic parameters
#[derive(SerializeSelective)]
pub struct Trade<I, S>
where
    I: quicklog::serialize::FixedSizeSerialize<8> + std::fmt::Display,
    S: quicklog::serialize::FixedSizeSerialize<4> + std::fmt::Display,
{
    #[serialize] pub order_id: I,
    #[serialize] pub symbol_id: S,
    #[serialize] pub quantity: u32,
}
```

**Key points:**
- Generic types used in `#[serialize]` fields require `FixedSizeSerialize<N> + Display` bounds
- Generic types only in non-serialized fields don't need any trait bounds
- Supports multiple generic parameters, nested generics, and lifetime parameters
- Zero runtime overhead - standard Rust monomorphization

### Performance Characteristics

| Approach | Latency | Memory Usage | Use Case |
|----------|---------|--------------|----------|
| **Selective Serialization** | **~5-10ns** | **50-60% smaller** | High-frequency logging |
| Debug Formatting | ~600ns | Full struct size | Development/debugging |
| Individual Serialize | ~60-80ns | Field-dependent | Single values |

### Built-in Support

All primitive types and common collections automatically implement `Serialize`:
- **Integers**: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`, `usize`, `isize`
- **Floats**: `f32`, `f64`
- **Strings**: `&str`
- **References**: `&T` where `T: Serialize` (delegates to the underlying type)
- **Collections**: `Option<T>`, `Vec<T>` where `T: Serialize`

All primitive types also implement `FixedSizeSerialize` for use with selective serialization.

**Note**: Reference serialization (`&T`) works by delegating to the underlying type's `Serialize` implementation, avoiding unnecessary clones. This also works for nested types like `Option<&T>` and `Vec<&T>`:

```rust
let value = 12345u64;
let opt_ref: Option<&u64> = Some(&value);
info!("optional ref: {}", ^opt_ref);  // Works! Output: Some(12345)

let vec_ref: Vec<&u32> = vec![&100, &200, &300];
info!("vec of refs: {}", ^vec_ref);  // Works! Output: [100, 200, 300]
```

### Available Macros

Quicklog provides two simple macros to eliminate boilerplate when implementing `FixedSizeSerialize`:

| Macro | Use Case | Example |
|-------|----------|---------|
| `impl_fixed_size_serialize_newtype!` | Simple wrapper types | `impl_fixed_size_serialize_newtype!(UserId, u64, 8);` |
| `impl_fixed_size_serialize_enum!` | Unit enums | `impl_fixed_size_serialize_enum!(Status, Active = 1, Inactive = 0);` |

**Benefits of using macros:**
- ✅ **Reduced boilerplate** - No need to write repetitive trait implementations
- ✅ **Compile-time safety** - Automatic size calculations and type checks
- ✅ **Consistency** - Uniform implementation patterns across your codebase
- ✅ **Maintainability** - Easy to update if inner types change

### Utilising different flushing mechanisms

```rust
use quicklog_flush::stdout_flusher::StdoutFlusher;
use quicklog::{info, init, flush, with_flush_into_file, with_flush};

fn main() {
    init!();

    // flush into stdout
    with_flush!(StdoutFlusher);

    // item goes into logging queue
    info!("hello world");

    // flushed into stdout
    flush!()

    // flush into a file path specified
    with_flush_into_file!("logs/my_log.log");

    info!("shave yaks");

    // flushed into file
    flush!();
}
```

More usage examples are available:
- [Basic usage](quicklog/examples/macros.rs)
- [Vec and collection logging](quicklog/examples/vec_serialization.rs)
- [High-performance selective serialization](quicklog/examples/custom_types_selective_serialization.rs)

## Benchmark

Measurements are made on a 2020 16 core M1 Macbook Air with 16 GB RAM.

### 🚀 Selective Serialization Performance (NEW)

**Encoding performance comparison for complex structs:**

| Approach | Encoding Time | Performance Gain | Memory Usage |
| -------- | ------------- | ---------------- | ------------ |
| **Selective Serialization** | **5.68 ns** | **Baseline** | **50-60% reduction** |
| Debug Formatting | 632.23 ns | **111x slower** | Full struct |
| Individual Serialize calls | ~64-180 ns | **8-15x slower** | Field-dependent |

**Real-world impact:**
- **High-frequency trading**: 1M orders/second = 5.7ms CPU time (vs 632ms with Debug)
- **Memory efficiency**: 55 bytes vs 120 bytes for typical Order struct
- **Zero heap allocations** in encoding hot path

### Logging a struct with a vector of 10 large structs

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **103.76 ns** | **104.14 ns** | **104.53 ns** |
| tracing  | 22.336 µs     | 22.423 µs     | 22.506 µs     |
| delog    | 21.528 µs     | 21.589 µs     | 21.646 µs     |

### Logging a single struct with 100 array elements

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **61.399 ns** | **62.436 ns** | **63.507 ns** |
| tracing  | 2.6501 µs     | 2.6572 µs     | 2.6646 µs     |
| delog    | 2.7610 µs     | 2.7683 µs     | 2.7761 µs     |

### Logging a small struct with primitives

| Logger   | Lower Bound   | Estimate      | Upper Bound   |
| -------- | ------------- | ------------- | ------------- |
| quicklog | **28.561 ns** | **28.619 ns** | **28.680 ns** |
| tracing  | 627.79 µs     | 629.91 µs     | 632.06 µs     |
| delog    | 719.54 µs     | 721.19 µs     | 722.96 µs     |

### 🚀 Vec Serialization Performance (NEW)

**Serialize vs Clone comparison for `Vec<T>` logging:**

| Vec Type | Elements | Clone (no prefix) | Serialize (^ prefix) | Speedup |
| -------- | -------- | ----------------- | -------------------- | ------- |
| `Vec<String>` | 10 | 235.02 ns | **88.98 ns** | **2.64× faster** ✅ |
| `Vec<String>` | 100 | 1,902.9 ns | **805.99 ns** | **2.36× faster** ✅ |
| `Vec<Order>` | 10 | 761.44 ns | **123.41 ns** | **6.17× faster** ✅ |
| `Vec<Order>` | 100 | 7,109.2 ns | **1,077.5 ns** | **6.58× faster** ✅ |
| `Vec<SelectiveOrder>` | 10 | 967.98 ns | **31.41 ns** | **30.8× faster** ✅ |
| `Vec<SelectiveOrder>` | 100 | 9,336.9 ns | **109.43 ns** | **85.3× faster** ✅ |
| `Vec<u32>` | 10 | 40.9 ns | 33.4 ns | 1.2× faster ⚠️ |
| `Vec<u32>` | 100 | **68.6 ns** | 126.2 ns | **Clone wins** ⚠️ |

**When to use `^` for Vec:**
- ✅ **Heap-allocated elements**: `Vec<String>`, `Vec<Box<T>>` → 2-3× faster
- ✅ **Complex structs**: `Vec<Order>` with multiple fields → 6-7× faster
- ✅ **Selective serialization**: `Vec<SelectiveOrder>` → 30-85× faster
- ⚠️ **Primitive elements**: `Vec<u32>`, `Vec<f64>` → Clone is faster for 50+ elements

**Real-world impact:**
- **High-frequency trading**: Logging 1M orders/sec saves **730ms CPU time** with selective serialization
- **Market data**: 100 symbols @ 100Hz saves **109μs/sec** with Vec<String> serialization
- **Collection logging**: Best gains with heap-allocated and complex types

For detailed benchmarks, see [VEC_BENCHMARK_RESULTS.md](VEC_BENCHMARK_RESULTS.md).

## Contribution & Support

We are open to contributions and requests!

Please post your bug reports or feature requests on [Github Issues](https://github.com/ghpr-asia/quicklog/issues).

## Roadmap

- [x] **High-performance selective field serialization** (NEW in 0.2.1)
- [x] **FixedSizeSerialize trait for custom types** (NEW in 0.2.1)
- [x] **Vec<T> serialization with 2-85× speedup** (NEW in 0.2.2)
- [x] **Generic type parameter support for SerializeSelective** (NEW in 0.2.3)
- [] add single-threaded and multi-threaded variants
- [] Try to remove nested `lazy_format` in recursion
- [] Check number of copies of data made in each log line and possibly reduce it
- [] Review uses of unsafe code
- [] Benchmark multi-threaded performance
- [] Statically assert that strings inside Level and LevelFilter are the same size

## Authors and acknowledgment

[Zack Ng](https://github.com/nhzaci), Tien Dat Nguyen, Michiel van Slobbe, Dheeraj Oswal

### Crates
- [Lucretiel/lazy_format](https://github.com/Lucretiel/lazy_format)
- [japaric/heapless](https://github.com/japaric/heapless)

### References
- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- [trussed-dev/delog](https://github.com/trussed-dev/delog)

## License

Copyright 2023 [Grasshopper Asia](https://github.com/ghpr-asia)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
