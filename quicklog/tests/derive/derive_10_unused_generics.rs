// Testing SerializeSelective with generics NOT used in serialization
// The generic type T is only in non-serialized fields
use quicklog::serialize::Serialize as _;
use quicklog::SerializeSelective;

// Struct with generic T that is NOT serialized
#[derive(SerializeSelective)]
struct Order<T> {
    #[serialize]
    pub id: u64,
    #[serialize]
    pub price: f64,

    // Generic type NOT serialized - should NOT require FixedSizeSerialize
    pub metadata: T,
}

fn main() {
    // Test with String (does NOT implement FixedSizeSerialize)
    let order = Order::<String> {
        id: 12345,
        price: 100.5,
        metadata: "some metadata".to_string(),
    };

    let mut buf = [0; 256];
    let (store, _) = order.encode(&mut buf);
    let output = format!("{}", store);

    assert!(output.contains("id=12345"));
    assert!(output.contains("price=100.5"));
    assert!(!output.contains("metadata")); // Not serialized
}
