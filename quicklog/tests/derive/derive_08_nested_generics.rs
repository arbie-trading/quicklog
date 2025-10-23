// Testing SerializeSelective with nested generic types (Option<T>)
use quicklog::serialize::Serialize as _;
use quicklog::SerializeSelective;

// Struct with Option<T> where T is generic
// Note: T must implement FixedSizeSerialize and Display (even when wrapped in Option)
#[derive(SerializeSelective)]
struct Record<T>
where
    T: quicklog::serialize::FixedSizeSerialize<8> + std::fmt::Display,
{
    #[serialize]
    pub id: u64,
    #[serialize]
    pub value: Option<T>,
    #[serialize]
    pub count: u32,

    // Not serialized
    pub description: String,
}

fn main() {
    // Test with Some value
    let record1 = Record::<u64> {
        id: 100,
        value: Some(42),
        count: 5,
        description: "Test record".to_string(),
    };

    let mut buf = [0; 256];
    let (store, _) = record1.encode(&mut buf);
    let output = format!("{}", store);

    assert!(output.contains("id=100"));
    assert!(output.contains("value=42"));
    assert!(output.contains("count=5"));

    // Test with None value
    let record2 = Record::<f64> {
        id: 200,
        value: None,
        count: 10,
        description: "Another record".to_string(),
    };

    let mut buf2 = [0; 256];
    let (store2, _) = record2.encode(&mut buf2);
    let output2 = format!("{}", store2);

    assert!(output2.contains("id=200"));
    assert!(output2.contains("value=None"));
    assert!(output2.contains("count=10"));
}
