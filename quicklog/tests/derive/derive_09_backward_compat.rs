// Testing backward compatibility - non-generic structs should work exactly as before
use quicklog::serialize::Serialize as _;
use quicklog::SerializeSelective;

// Non-generic struct (should work exactly as before)
#[derive(SerializeSelective)]
struct SimpleOrder {
    #[serialize]
    pub id: u64,
    #[serialize]
    pub price: f64,
    #[serialize]
    pub size: Option<u32>,

    // Not serialized
    pub status: String,
    pub metadata: Vec<String>,
}

fn main() {
    let order = SimpleOrder {
        id: 12345,
        price: 100.5,
        size: Some(50),
        status: "Active".to_string(),
        metadata: vec!["tag1".to_string(), "tag2".to_string()],
    };

    let mut buf = [0; 256];
    let (store, _) = order.encode(&mut buf);
    let output = format!("{}", store);

    assert!(output.contains("id=12345"));
    assert!(output.contains("price=100.5"));
    assert!(output.contains("size=50"));
    assert!(!output.contains("Active"));
    assert!(!output.contains("tag1"));
    assert!(!output.contains("tag2"));

    // Verify the serialized size is minimal (only serialized fields)
    let buffer_size = order.buffer_size_required();
    // 8 bytes (u64) + 8 bytes (f64) + 1 byte (Option marker) + 4 bytes (u32) = 21 bytes
    assert_eq!(buffer_size, 21);
}
