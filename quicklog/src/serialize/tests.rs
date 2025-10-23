use crate::gen_serialize_enum;
use crate::serialize::encode_debug;

use super::Serialize;

macro_rules! assert_primitive_encode_decode {
    ($primitive:ty, $val:expr) => {{
        const BUF_SIZE: usize = std::mem::size_of::<$primitive>();
        let mut buf = [0u8; BUF_SIZE];

        let x: $primitive = $val;
        let (x_store, _) = x.encode(&mut buf);
        assert_eq!(format!("{}", x), format!("{}", x_store));
    }};
}

#[test]
fn serialize_primitives() {
    assert_primitive_encode_decode!(i32, -1);
    assert_primitive_encode_decode!(i64, -123);
    assert_primitive_encode_decode!(isize, -1234);
    assert_primitive_encode_decode!(f32, 1.23);
    assert_primitive_encode_decode!(f64, 1.23456);
    assert_primitive_encode_decode!(u32, 999);
    assert_primitive_encode_decode!(u64, 9999);
    assert_primitive_encode_decode!(usize, 99999);
}

#[test]
fn serialize_multiple_primitives() {
    let mut buf = [0; 128];
    let a: i32 = -1;
    let b: u32 = 999;
    let c: usize = 100000;

    let (a_store, chunk) = a.encode(&mut buf);
    let (b_store, chunk) = b.encode(chunk);
    let (c_store, _) = c.encode(chunk);

    assert_eq!(
        format!("{} {} {}", a, b, c),
        format!("{} {} {}", a_store, b_store, c_store)
    )
}

#[test]
fn serialize_str() {
    let mut buf = [0; 128];
    let s = "hello world";
    let (store, _) = s.encode(&mut buf);

    assert_eq!(s, format!("{}", store).as_str())
}

#[test]
fn serialize_debug() {
    #[derive(Debug)]
    #[allow(unused)]
    struct DebugStruct {
        s: &'static str,
    }

    let mut buf = [0; 128];
    let s = DebugStruct { s: "Hello World" };
    let (store, _) = encode_debug(&s, &mut buf);

    assert_eq!(format!("{:?}", s), format!("{}", store))
}

#[test]
fn serialize_unit_enum() {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Color {
        Red = 0,
        Green = 1,
        Blue = 2,
    }

    gen_serialize_enum!(Color, Red, Green, Blue);

    let mut buf = [0; 32];

    // Test Red variant
    let red = Color::Red;
    let (red_store, remaining) = red.encode(&mut buf);
    assert_eq!(red_store.as_string(), "Red");

    // Test Green variant
    let green = Color::Green;
    let (green_store, remaining) = green.encode(remaining);
    assert_eq!(green_store.as_string(), "Green");

    // Test Blue variant
    let blue = Color::Blue;
    let (blue_store, _) = blue.encode(remaining);
    assert_eq!(blue_store.as_string(), "Blue");
}

#[test]
fn serialize_enum_with_explicit_discriminants() {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Status {
        Inactive = 10,
        Active = 20,
        Suspended = 30,
    }

    gen_serialize_enum!(Status, Inactive, Active, Suspended);

    let mut buf = [0; 32];

    let active = Status::Active;
    let (store, _) = active.encode(&mut buf);
    assert_eq!(store.as_string(), "Active");

    // Verify buffer size requirement
    assert_eq!(active.buffer_size_required(), std::mem::size_of::<u8>());
}

#[test]
fn serialize_multiple_enums() {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Priority {
        Low = 0,
        Medium = 1,
        High = 2,
    }

    gen_serialize_enum!(Priority, Low, Medium, High);

    let mut buf = [0; 64];
    let low = Priority::Low;
    let medium = Priority::Medium;
    let high = Priority::High;

    let (low_store, chunk) = low.encode(&mut buf);
    let (medium_store, chunk) = medium.encode(chunk);
    let (high_store, _) = high.encode(chunk);

    assert_eq!(
        format!("{} {} {}", low_store, medium_store, high_store),
        "Low Medium High"
    );
}

#[test]
fn serialize_enum_roundtrip() {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Direction {
        North = 0,
        East = 1,
        South = 2,
        West = 3,
    }

    gen_serialize_enum!(Direction, North, East, South, West);

    let original = Direction::South;
    let mut buf = [0; 16];

    // Encode
    let (store, _) = original.encode(&mut buf);

    // Verify the encoded representation can be decoded back to string
    let decoded_string = store.as_string();
    assert_eq!(decoded_string, "South");

    // Verify the discriminant matches
    let discriminant = original as u8;
    let expected_bytes = discriminant.to_le_bytes();
    assert_eq!(&buf[0..std::mem::size_of::<u8>()], &expected_bytes);
}

#[test]
fn serialize_level_enum() {
    // Test with existing Level enum from crate::level
    use crate::level::Level;

    gen_serialize_enum!(Level, Trace, Debug, Info, Warn, Error);

    let mut buf = [0; 64];

    // Test all levels
    let trace = Level::Trace;
    let debug = Level::Debug;
    let info = Level::Info;
    let warn = Level::Warn;
    let error = Level::Error;

    let (trace_store, remaining) = trace.encode(&mut buf);
    let (debug_store, remaining) = debug.encode(remaining);
    let (info_store, remaining) = info.encode(remaining);
    let (warn_store, remaining) = warn.encode(remaining);
    let (error_store, _) = error.encode(remaining);

    assert_eq!(trace_store.as_string(), "Trace");
    assert_eq!(debug_store.as_string(), "Debug");
    assert_eq!(info_store.as_string(), "Info");
    assert_eq!(warn_store.as_string(), "Warn");
    assert_eq!(error_store.as_string(), "Error");

    // Verify discriminant values match Level enum representation
    assert_eq!(Level::Trace as u8, 0);
    assert_eq!(Level::Debug as u8, 1);
    assert_eq!(Level::Info as u8, 2);
    assert_eq!(Level::Warn as u8, 3);
    assert_eq!(Level::Error as u8, 4);
}

#[test]
fn serialize_option_some() {
    let mut buf = [0; 128];

    // Test Option<i32> with Some value
    let some_value: Option<i32> = Some(42);

    // Verify buffer size first
    assert_eq!(some_value.buffer_size_required(), 5); // 1 marker + 4 bytes for i32

    let (store, _) = some_value.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "Some(42)");
}

#[test]
fn serialize_option_none() {
    let mut buf = [0; 128];

    // Test Option<i32> with None value
    let none_value: Option<i32> = None;

    // Verify buffer size first
    assert_eq!(none_value.buffer_size_required(), 1); // Just the marker

    let (store, _) = none_value.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "None");
}

#[test]
fn serialize_option_string() {
    let mut buf = [0; 128];

    // Test Option<&str> with Some value
    let some_str: Option<&str> = Some("hello");
    let (store, _) = some_str.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "Some(hello)");

    // Test Option<&str> with None value
    let none_str: Option<&str> = None;
    let (store_none, _) = none_str.encode(&mut buf);
    assert_eq!(store_none.as_string(), "None");
}

#[test]
fn serialize_nested_option() {
    let mut buf = [0; 128];

    // Test Option<Option<i32>>
    let nested_some: Option<Option<i32>> = Some(Some(99));
    let (store, _) = nested_some.encode(&mut buf);

    // Should decode as "Some(Some(99))"
    assert_eq!(store.as_string(), "Some(Some(99))");

    // Test Option<Option<i32>> with inner None
    let nested_inner_none: Option<Option<i32>> = Some(None);
    let (store2, _) = nested_inner_none.encode(&mut buf);
    assert_eq!(store2.as_string(), "Some(None)");

    // Test Option<Option<i32>> with outer None
    let nested_outer_none: Option<Option<i32>> = None;
    let (store3, _) = nested_outer_none.encode(&mut buf);
    assert_eq!(store3.as_string(), "None");
}

#[test]
fn serialize_option_roundtrip() {
    let mut buf = [0; 128];

    // Test roundtrip encoding/decoding
    let original_some: Option<u64> = Some(12345678901234567890);
    let original_none: Option<u64> = None;

    // Encode both
    let (store_some, remaining) = original_some.encode(&mut buf);
    let (store_none, _) = original_none.encode(remaining);

    // Verify they can be decoded correctly
    assert_eq!(store_some.as_string(), "Some(12345678901234567890)");
    assert_eq!(store_none.as_string(), "None");

    // Verify buffer sizes
}

#[test]
fn serialize_vec_empty() {
    let mut buf = [0; 128];

    // Test empty Vec<i32>
    let empty_vec: Vec<i32> = Vec::new();

    // Verify buffer size (just the length prefix)
    assert_eq!(empty_vec.buffer_size_required(), 8); // SIZE_LENGTH for empty vec

    let (store, _) = empty_vec.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[]");
}

#[test]
fn serialize_vec_primitives() {
    let mut buf = [0; 128];

    // Test Vec<i32> with values
    let vec_i32: Vec<i32> = vec![1, 2, 3, 4, 5];

    // Verify buffer size: 8 (length) + 5 * 4 (i32 size) = 28 bytes
    assert_eq!(vec_i32.buffer_size_required(), 28);

    let (store, _) = vec_i32.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[1, 2, 3, 4, 5]");
}

#[test]
fn serialize_vec_single_element() {
    let mut buf = [0; 128];

    // Test Vec<u64> with single element
    let vec_single: Vec<u64> = vec![42];

    // Verify buffer size: 8 (length) + 8 (u64) = 16 bytes
    assert_eq!(vec_single.buffer_size_required(), 16);

    let (store, _) = vec_single.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[42]");
}

#[test]
fn serialize_vec_strings() {
    let mut buf = [0; 256];

    // Test Vec<&str>
    let vec_str: Vec<&str> = vec!["hello", "world", "test"];

    let (store, _) = vec_str.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[hello, world, test]");
}

#[test]
fn serialize_vec_floats() {
    let mut buf = [0; 128];

    // Test Vec<f64>
    let vec_floats: Vec<f64> = vec![1.5, 2.5, 3.5];

    // Verify buffer size: 8 (length) + 3 * 8 (f64 size) = 32 bytes
    assert_eq!(vec_floats.buffer_size_required(), 32);

    let (store, _) = vec_floats.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[1.5, 2.5, 3.5]");
}

#[test]
fn serialize_vec_nested() {
    let mut buf = [0; 256];

    // Test Vec<Option<i32>>
    let vec_option: Vec<Option<i32>> = vec![Some(10), None, Some(20)];

    let (store, _) = vec_option.encode(&mut buf);

    // Verify decoding
    assert_eq!(store.as_string(), "[Some(10), None, Some(20)]");
}

#[test]
fn serialize_multiple_vecs() {
    let mut buf = [0; 256];

    let vec1: Vec<i32> = vec![1, 2];
    let vec2: Vec<i32> = vec![3, 4, 5];

    let (store1, remaining) = vec1.encode(&mut buf);
    let (store2, _) = vec2.encode(remaining);

    assert_eq!(store1.as_string(), "[1, 2]");
    assert_eq!(store2.as_string(), "[3, 4, 5]");
}

#[test]
fn serialize_vec_large() {
    let mut buf = [0; 1024];

    // Test with larger vector
    let vec_large: Vec<u32> = (0..50).collect();

    let (store, _) = vec_large.encode(&mut buf);

    // Verify it contains expected elements
    let decoded = store.as_string();
    assert!(decoded.starts_with("[0, 1, 2"));
    assert!(decoded.ends_with("48, 49]"));
}

#[test]
fn serialize_vec_roundtrip() {
    let mut buf = [0; 256];

    // Test roundtrip with different types
    let original_i64: Vec<i64> = vec![100, -200, 300];
    let (store, _) = original_i64.encode(&mut buf);

    assert_eq!(store.as_string(), "[100, -200, 300]");

    // Verify buffer consumption
    let expected_size = 8 + (3 * 8); // length + 3 i64s
    assert_eq!(original_i64.buffer_size_required(), expected_size);
}
