use quicklog::{init, info, flush, FixedSizeSerialize, SerializeSelective};
use quicklog::{impl_fixed_size_serialize_newtype, impl_fixed_size_serialize_enum};
use quicklog::serialize::Serialize;
use std::fmt::Display;

// Example custom types demonstrating different FixedSizeSerialize implementation approaches:
//
// 1. impl_fixed_size_serialize_newtype! - for simple wrapper types (Id, Price, Size, Timestamp)
// 2. impl_fixed_size_serialize_enum! - for enums with explicit discriminants (Side)
// 3. Manual implementation - for complex types requiring custom logic (MarketId)

// Simple newtype wrapper around u128
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Id(u128);

// Use macro for automatic FixedSizeSerialize implementation
impl_fixed_size_serialize_newtype!(Id, u128, 16);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

// Fixed-size string using ArrayString (simulated here)
#[derive(Debug, Clone, PartialEq)]
pub struct MarketId {
    data: [u8; 16],
    len: u8,
}

impl MarketId {
    pub fn new(s: &str) -> Result<Self, &'static str> {
        if s.len() > 16 {
            return Err("MarketId too long");
        }
        let mut data = [0u8; 16];
        data[..s.len()].copy_from_slice(s.as_bytes());
        Ok(Self {
            data,
            len: s.len() as u8,
        })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len as usize]).unwrap()
    }
}

impl FixedSizeSerialize<16> for MarketId {
    fn to_le_bytes(&self) -> [u8; 16] {
        // Serialize as null-padded string (length is implicit from null terminator)
        self.data
    }

    fn from_le_bytes(bytes: [u8; 16]) -> Self {
        // Find the length by looking for first null byte or use full length
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(16) as u8;
        Self { data: bytes, len }
    }
}

impl Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarketId({})", self.as_str())
    }
}

// Wrapper around OrderedFloat<f64> (simulated here as regular f64)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price(f64);

// Simple wrapper around f64
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size(f64);

// Timestamp wrapper
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timestamp(u64);

// Use individual macros for each newtype wrapper
impl_fixed_size_serialize_newtype!(Price, f64, 8);
impl_fixed_size_serialize_newtype!(Size, f64, 8);
impl_fixed_size_serialize_newtype!(Timestamp, u64, 8);

impl Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Price({})", self.0)
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Size({})", self.0)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timestamp({})", self.0)
    }
}

// Enum with explicit discriminant values
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

// Use enum macro for automatic FixedSizeSerialize implementation
impl_fixed_size_serialize_enum!(Side, Buy = 0, Sell = 1);

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

// Now define the Order struct using selective serialization
#[derive(Debug, Clone, SerializeSelective)]
pub struct Order {
    #[serialize]
    pub oid: Id,
    #[serialize]
    pub cloid: Option<Id>,
    #[serialize]
    pub external_name: MarketId, // Market ID
    #[serialize]
    pub side: Side, // 0=Buy, 1=Sell
    #[serialize]
    pub price: Option<Price>,
    #[serialize]
    pub size: Size,
    #[serialize]
    pub time: Timestamp, // Unix timestamp
    #[serialize]
    pub time_received: Timestamp, // Unix timestamp

    // Not serialized fields (lower priority for logging)
    pub order_type: u8, // Market, Limit, etc.
    pub reduce_only: bool,
    pub time_in_force: u8, // GTC, IOC, FOK
    pub post_only: bool,
    pub status: u8, // Pending, Filled, etc.
    pub filled_size: f64,
    pub remaining_size: f64,
    pub avg_fill_price: Option<f64>,
}

impl Order {
    pub fn new_sample() -> Self {
        Order {
            oid: Id(12345678901234567890),
            cloid: Some(Id(98765432109876543210)),
            external_name: MarketId::new("BTCUSD").unwrap(),
            side: Side::Buy,
            price: Some(Price(45000.50)),
            size: Size(2.5),
            time: Timestamp(1642781234),
            time_received: Timestamp(1642781235),
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
}

impl Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Order {{ oid={}, cloid={:?}, external_name={}, side={}, price={:?}, size={}, time={}, time_received={} }}",
            self.oid, self.cloid, self.external_name, self.side, self.price, self.size, self.time, self.time_received
        )
    }
}

fn main() {
    // Initialize quicklog
    init!();

    let order = Order::new_sample();

    println!("=== Custom Types Selective Serialization Example ===\n");

    // Show the full order (Debug formatting)
    println!("Full Order (Debug): {:#?}\n", order);

    // Show selective serialization vs full display
    println!("=== Performance Comparison ===");

    // Test 1: Selective serialization (high performance)
    info!(order_selective = ^order, "Order with selective serialization");
    println!("✅ Logged with selective serialization (^order syntax)");

    // Test 2: Full Debug formatting (slower)
    info!(order_debug = ?order, "Order with debug formatting");
    println!("✅ Logged with debug formatting (?order syntax)");

    // Test 3: Display formatting
    info!(order_display = %order, "Order with display formatting");
    println!("✅ Logged with display formatting (%order syntax)");

    // Test buffer size calculation
    let buffer_size = order.buffer_size_required();
    println!("\n=== Buffer Size Analysis ===");
    println!("Selective serialization buffer size: {} bytes", buffer_size);
    println!("Expected breakdown:");
    println!("  - oid (Id): 16 bytes");
    println!("  - cloid (Option<Id>): 1 + 16 = 17 bytes");
    println!("  - external_name (MarketId): 16 bytes");
    println!("  - side (Side): 1 byte");
    println!("  - price (Option<Price>): 1 + 8 = 9 bytes");
    println!("  - size (Size): 8 bytes");
    println!("  - time (Timestamp): 8 bytes");
    println!("  - time_received (Timestamp): 8 bytes");
    println!("  Total expected: 83 bytes");

    // Test encoding and decoding
    let mut buffer = [0u8; 128];
    let (store, _remaining) = order.encode(&mut buffer);
    let decoded_string = store.as_string();
    println!("\n=== Encoded/Decoded Result ===");
    println!("Decoded string: {}", decoded_string);

    // Demonstrate high-frequency usage pattern
    println!("\n=== High-Frequency Simulation ===");
    let iterations = 10000;
    let start = std::time::Instant::now();

    for i in 0..iterations {
        let mut order = Order::new_sample();
        order.oid = Id(i as u128);
        info!(order = ^order, "HFT order");
    }

    let duration = start.elapsed();
    println!("Logged {} orders in {:?}", iterations, duration);
    println!("Average latency per log: {:.2} ns", duration.as_nanos() as f64 / iterations as f64);

    // Flush all logs
    flush!();

    println!("\n=== Summary ===");
    println!("✅ Successfully demonstrated FixedSizeSerialize implementations:");
    println!("   - Id, Price, Size, Timestamp (wrapper types) - using impl_fixed_size_serialize_newtype!");
    println!("   - Side (enum) - using impl_fixed_size_serialize_enum!");
    println!("   - MarketId (fixed-size string) - manual implementation for complex logic");
    println!("✅ Two macro approaches for different use cases:");
    println!("   - impl_fixed_size_serialize_newtype! for wrapper types");
    println!("   - impl_fixed_size_serialize_enum! for unit enums");
    println!("   - Manual implementations when custom logic is needed");
    println!("✅ All types work seamlessly with #[derive(SerializeSelective)]");
    println!("✅ Achieved high-performance selective field serialization");
}

#[cfg(test)]
mod tests {
    use super::*;
    use quicklog::serialize::Serialize;

    #[test]
    fn test_id_fixed_size_serialize() {
        let id = Id(12345678901234567890);
        let bytes = id.to_le_bytes();
        let restored = Id::from_le_bytes(bytes);
        assert_eq!(id, restored);
    }

    #[test]
    fn test_market_id_fixed_size_serialize() {
        let market_id = MarketId::new("BTCUSD").unwrap();
        let bytes = market_id.to_le_bytes();
        let restored = MarketId::from_le_bytes(bytes);
        assert_eq!(market_id.as_str(), restored.as_str());
    }

    #[test]
    fn test_side_enum_fixed_size_serialize() {
        for side in [Side::Buy, Side::Sell] {
            let bytes = side.to_le_bytes();
            let restored = Side::from_le_bytes(bytes);
            assert_eq!(side, restored);
        }
    }

    #[test]
    fn test_order_selective_serialization() {
        let order = Order::new_sample();
        let mut buffer = [0u8; 256];
        let (store, _) = order.encode(&mut buffer);

        // Should be able to decode without panicking
        let decoded = store.as_string();
        assert!(!decoded.is_empty());

        // Should contain all selected field names
        assert!(decoded.contains("oid="));
        assert!(decoded.contains("cloid="));
        assert!(decoded.contains("external_name="));
        assert!(decoded.contains("side="));
        assert!(decoded.contains("price="));
        assert!(decoded.contains("size="));
        assert!(decoded.contains("time="));
        assert!(decoded.contains("time_received="));

        // Should NOT contain non-selected fields
        assert!(!decoded.contains("order_type"));
        assert!(!decoded.contains("reduce_only"));
        assert!(!decoded.contains("status"));
    }

    #[test]
    fn test_buffer_size_consistency() {
        let order = Order::new_sample();
        let calculated_size = order.buffer_size_required();

        // Manually calculate expected size
        let expected_size = 16 + // oid: Id
                           17 + // cloid: Option<Id> (1 + 16)
                           16 + // external_name: MarketId
                            1 + // side: Side
                            9 + // price: Option<Price> (1 + 8)
                            8 + // size: Size
                            8 + // time: Timestamp
                            8;  // time_received: Timestamp

        assert_eq!(calculated_size, expected_size);
    }
}