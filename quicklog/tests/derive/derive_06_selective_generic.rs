// Testing SerializeSelective with generic type parameters
use quicklog::impl_fixed_size_serialize_newtype;
use quicklog::serialize::Serialize as _;
use quicklog::SerializeSelective;

// Custom newtype wrapper
#[derive(Clone, Copy)]
struct Id<T>(T);

// Implement FixedSizeSerialize for Id<u64>
impl_fixed_size_serialize_newtype!(Id<u64>, u64, 8);

// Implement Display for Id<u64>
impl std::fmt::Display for Id<u64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Generic struct with SerializeSelective
// Note: T must implement FixedSizeSerialize and Display for selective serialization
#[derive(SerializeSelective)]
struct Order<T>
where
    T: quicklog::serialize::FixedSizeSerialize<8> + std::fmt::Display,
{
    #[serialize]
    pub id: T,
    #[serialize]
    pub price: f64,
    #[serialize]
    pub size: Option<u64>,

    // Not serialized
    pub status: String,
}

fn main() {
    // Test with concrete type u64
    let order1 = Order::<u64> {
        id: 12345,
        price: 100.5,
        size: Some(10),
        status: "Active".to_string(),
    };

    let mut buf = [0; 256];
    let (store, _) = order1.encode(&mut buf);
    let output = format!("{}", store);

    assert!(output.contains("id=12345"));
    assert!(output.contains("price=100.5"));
    assert!(output.contains("size=10"));
    assert!(!output.contains("Active")); // status not serialized

    // Test with custom newtype Id<u64>
    let order2 = Order::<Id<u64>> {
        id: Id(99999),
        price: 250.75,
        size: None,
        status: "Pending".to_string(),
    };

    let mut buf2 = [0; 256];
    let (store2, _) = order2.encode(&mut buf2);
    let output2 = format!("{}", store2);

    assert!(output2.contains("id=99999"));
    assert!(output2.contains("price=250.75"));
    assert!(output2.contains("size=None"));
    assert!(!output2.contains("Pending")); // status not serialized
}
