# quicklog

Ultra-fast single-threaded logging framework with **selective field serialization**. Achieves **111x performance improvement** over Debug formatting for complex structs, and almost 200x faster than `tracing` and `delog` for large structs.

Supports standard logging macros like `trace!`, `debug!`, `info!`, `warn!` and `error!`.

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
    // initialise required resources, called near application entry point
    init!();

    // adds item to logging queue
    info!("hello world");

    let some_var = 10;

    // clones some_var, defers formatting to flush time
    info!("value of some_var: {}", some_var);

    // NEW: Use ^ prefix in format args for ultra-fast serialization
    info!("value of some_var: {}", ^some_var);

    // flushes everything in queue
    flush!();
}
```

### Logging Syntax and Performance

Quicklog provides multiple ways to log values with different performance characteristics:

#### Format String Arguments

You can use prefixes **directly in format arguments** for fine-grained control over performance:

```rust
// Serialize arguments (fastest: ~5-10ns per arg)
info!("Order: id={}, price={}, size={}", ^order_id, ^price, ^size);

// Mix different strategies in one call
info!("Data: {} {} {}", ^serialized, cloned_var, %display_var);

// Unprefixed args (clone and defer formatting: ~28-104ns)
info!("Value: {}", some_var);
info!("Debug: {:?}", some_struct);
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
- For **primitives**: Use unprefixed args (fastest at ~1-2ns)
- For **structs**: Use `^` prefix with `Serialize` trait (~5-10ns vs ~28-104ns for cloning)
- For **debugging**: Use `%` or `?` when you need immediate formatting

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

### Utilising `Serialize`

In order to avoid cloning a large struct, you can implement the `Serialize` trait.

This allows you to copy specific parts of your struct onto a circular byte buffer and avoid copying the rest by encoding providing a function to decode your struct from a byte buffer.

For a complete example, refer to `~/quicklog/benches/logger_benchmark.rs`.

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

### Performance Characteristics

| Approach | Latency | Memory Usage | Use Case |
|----------|---------|--------------|----------|
| **Selective Serialization** | **~5-10ns** | **50-60% smaller** | High-frequency logging |
| Debug Formatting | ~600ns | Full struct size | Development/debugging |
| Individual Serialize | ~60-80ns | Field-dependent | Single values |

### Built-in Support

All primitive types automatically implement `FixedSizeSerialize`:
- **Integers**: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`, `usize`, `isize`
- **Floats**: `f32`, `f64`
- **Options**: `Option<T>` where `T: Serialize`

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

## Contribution & Support

We are open to contributions and requests!

Please post your bug reports or feature requests on [Github Issues](https://github.com/ghpr-asia/quicklog/issues).

## Roadmap

- [x] **High-performance selective field serialization** (NEW in 0.2.1)
- [x] **FixedSizeSerialize trait for custom types** (NEW in 0.2.1)
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
