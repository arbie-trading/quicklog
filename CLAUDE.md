# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**quicklog** is an ultra-fast single-threaded logging framework for Rust that achieves 111x performance improvement over Debug formatting through **selective field serialization**. The core design philosophy is to minimize call site latency by deferring both formatting and I/O operations.

Key performance characteristics:
- Deferred formatting and I/O on the hot path
- Selective field serialization for complex structs (~5-10ns encoding vs ~600ns for Debug)
- Zero heap allocations during encoding
- Single-threaded design optimized for HFT and low-latency systems

## Workspace Structure

This is a Cargo workspace with four crates:

1. **quicklog** (main crate) - Core logging functionality, macros, and serialization traits
2. **quicklog-macros** - Procedural macros for logging (`info!`, `debug!`, etc.) and derive macros (`Serialize`, `SerializeSelective`)
3. **quicklog-clock** - Clock trait and implementations for timestamping
4. **quicklog-flush** - Flush trait and implementations (stdout, file, noop)

## Common Development Commands

### Building
```bash
# Build all workspace members
cargo build

# Build with release optimizations
cargo build --release

# Build specific crate
cargo build -p quicklog
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p quicklog

# Run UI tests (compile-fail tests using trybuild)
cargo test --test ui

# Run derive macro tests
cargo test --test derive

# Run specific test
cargo test --test serialize
```

### Benchmarking
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench selective_serialization_benchmark
cargo bench --bench logger_benchmark
cargo bench --bench quicklog_benchmark
```

### Examples
```bash
# Run basic usage example
cargo run --example macros

# Run selective serialization example
cargo run --example custom_types_selective_serialization
```

## Architecture

### Core Serialization System

The serialization architecture has two main traits:

**`Serialize` trait** - General trait for types that need custom serialization:
- `encode()` - Copies minimal bytes to buffer, returns `Store` with decode function
- `decode()` - Reconstructs string representation from buffer
- `buffer_size_required()` - Returns bytes needed for encoding
- Used for variable-size types like strings and complex structs
- Implementation location: `quicklog/src/serialize/mod.rs`

**`FixedSizeSerialize<N>` trait** - High-performance trait for fixed-size types:
- Uses const generics for compile-time size calculation
- `to_le_bytes()` / `from_le_bytes()` for primitive-like types
- Zero virtual dispatch overhead
- Used by selective serialization for maximum performance
- Implementation location: `quicklog/src/serialize/mod.rs`

### Selective Serialization Pattern

The `#[derive(SerializeSelective)]` macro generates optimal code for logging only specific struct fields:

1. Fields marked `#[serialize]` are included
2. Buffer size is computed at compile time
3. Sequential encoding without `Store` overhead for each field
4. Implementation: `quicklog-macros/src/selective_serialize.rs`

**Usage pattern:**
```rust
#[derive(SerializeSelective)]
pub struct Order {
    #[serialize] pub id: u64,
    #[serialize] pub price: Option<f64>,
    // excluded fields...
}
```

### Logging Flow

1. **Call site**: Log macro captures args → serializes to byte buffer → enqueues `(Instant, LogRecord)` tuple
2. **Queue**: Lock-free SPSC ring buffer (`heapless::spsc::Queue`) with capacity `MAX_LOGGER_CAPACITY`
3. **Flush site**: Dequeues records → formats strings → writes to flusher (stdout/file)

The byte buffer is a circular buffer with capacity `MAX_SERIALIZE_BUFFER_CAPACITY` that wraps around in release mode.

### Macro System

Logging macros (`info!`, `debug!`, etc.) are procedural macros that:
- Parse structured fields (`field = value` syntax)
- Handle prefixes: `^` (serialize), `%` (display), `?` (debug)
- Generate code to encode args into byte buffer
- Create `LogRecord` and enqueue to logger
- Implementation: `quicklog-macros/src/expand.rs`

### Build-Time Configuration

The `build.rs` script generates `src/constants.rs` with:
- `MAX_LOGGER_CAPACITY` (default: 1,000,000) - from `QUICKLOG_MAX_LOGGER_CAPACITY` env var
- `MAX_SERIALIZE_BUFFER_CAPACITY` (default: 1,000,000) - from `QUICKLOG_MAX_SERIALIZE_BUFFER_CAPACITY` env var

These control the ring buffer and byte buffer sizes at compile time.

## Implementation Patterns

### Adding a New Fixed-Size Type

For simple wrapper types, use convenience macros:
```rust
pub struct OrderId(u64);
impl_fixed_size_serialize_newtype!(OrderId, u64, 8);

#[repr(u8)]
pub enum Side { Buy = 0, Sell = 1 }
impl_fixed_size_serialize_enum!(Side, Buy = 0, Sell = 1);
```

For custom types, implement the trait:
```rust
impl FixedSizeSerialize<16> for CustomType {
    fn to_le_bytes(&self) -> [u8; 16] { /* ... */ }
    fn from_le_bytes(bytes: [u8; 16]) -> Self { /* ... */ }
}
```

### Blanket Implementations

Important blanket implementations to be aware of:
- `Option<T>` implements `Serialize` where `T: Serialize` (encodes Some/None marker + value)
- All primitive numeric types implement `FixedSizeSerialize<N>`

### Test Organization

- Unit tests: In `quicklog/tests/` with individual test files
- UI tests: In `quicklog/tests/failures/` - compile-fail tests using `trybuild`
- Derive tests: In `quicklog/tests/derive/` - tests for macro-generated code
- Common utilities: In `quicklog/tests/common/mod.rs`

The custom test configuration in `Cargo.toml` disables `autotests = false` and explicitly defines test targets to work with `trybuild`.

## Performance Considerations

- **Hot path optimization**: Minimize work at call sites - only serialize, never format
- **Buffer management**: The byte buffer wraps in release mode but panics in debug mode to catch sizing issues
- **Memory layout**: Selective serialization generates sequential encoding for cache-friendly access
- **Compile-time dispatch**: `FixedSizeSerialize` uses const generics to avoid trait object overhead

## Testing Performance Changes

When modifying serialization code:
1. Run benchmarks before changes: `cargo bench --bench selective_serialization_benchmark > before.txt`
2. Make changes
3. Run benchmarks after: `cargo bench --bench selective_serialization_benchmark > after.txt`
4. Compare results to ensure no regressions
