// Testing SerializeSelective with multiple generic type parameters
use quicklog::serialize::Serialize as _;
use quicklog::SerializeSelective;

// Struct with two generic parameters
// Note: Both T and U must implement FixedSizeSerialize and Display
#[derive(SerializeSelective)]
struct Trade<T, U>
where
    T: quicklog::serialize::FixedSizeSerialize<8> + std::fmt::Display,
    U: quicklog::serialize::FixedSizeSerialize<4> + std::fmt::Display,
{
    #[serialize]
    pub order_id: T,
    #[serialize]
    pub symbol_id: U,
    #[serialize]
    pub quantity: u32,
    #[serialize]
    pub price: Option<f64>,

    // Not serialized
    pub metadata: String,
}

fn main() {
    // Test with two different types
    let trade = Trade::<u64, u32> {
        order_id: 1000000,
        symbol_id: 5000,
        quantity: 100,
        price: Some(99.99),
        metadata: "Some metadata".to_string(),
    };

    let mut buf = [0; 256];
    let (store, _) = trade.encode(&mut buf);
    let output = format!("{}", store);

    assert!(output.contains("order_id=1000000"));
    assert!(output.contains("symbol_id=5000"));
    assert!(output.contains("quantity=100"));
    assert!(output.contains("price=99.99"));
    assert!(!output.contains("metadata")); // Not serialized
}
