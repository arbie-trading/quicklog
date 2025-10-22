use quicklog::{flush_all, impl_fixed_size_serialize_newtype, info, init, SerializeSelective};
use quicklog_flush::stdout_flusher::StdoutFlusher;
use std::fmt;

// Define some custom types with FixedSizeSerialize
#[derive(Clone, Copy)]
pub struct OrderId(u64);
impl_fixed_size_serialize_newtype!(OrderId, u64, 8);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OrderId({})", self.0)
    }
}

#[derive(Clone, Copy)]
pub struct Price(f64);
impl_fixed_size_serialize_newtype!(Price, f64, 8);

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.0)
    }
}

// Define a struct with selective serialization
#[derive(SerializeSelective, Debug, Clone)]
pub struct Order {
    #[serialize]
    pub id: u64,
    #[serialize]
    pub price: Option<f64>,
    #[serialize]
    pub size: f64,
    // Not serialized
    pub metadata: String,
}

fn main() {
    init!();
    quicklog::with_flush!(StdoutFlusher);

    let order_id = 12345u64;
    let price = 100.50f64;
    let size = 2.5f64;
    let filled_qty = 100u32;

    // Test 1: All args with ^ prefix (serialize primitives)
    info!(
        "Order: id={}, price={}, size={}, filled_qty={}",
        ^order_id,
        ^price,
        ^size,
        ^filled_qty
    );

    // Test 2: Mix of prefixes with struct
    let order = Order {
        id: 67890,
        price: Some(200.75),
        size: 1.5,
        metadata: "test metadata".to_string(),
    };

    info!(
        "Order details: {}, price={}, debug={:?}",
        ^order,
        %price,
        ?order
    );

    // Test 3: Unprefixed args (should still work)
    let x = 42;
    let y = "hello";
    info!("Normal args: x={}, y={}", x, y);

    // Test 4: Mix everything
    info!(
        "Mixed: {} {} {} {}",
        ^order_id,
        x,
        ?order,
        %order_id
    );

    // Test 5: With structured fields too
    info!(?order, "Structured field with format args: {}", ^size);

    // Test 6: Just to show custom types work with Display
    let custom_price = Price(999.99);
    info!("Custom price display: {}", %custom_price);

    flush_all!();
}
