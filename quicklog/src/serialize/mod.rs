use std::{fmt::Display, str::from_utf8};

pub mod buffer;

/// Allows specification of a custom way to serialize the Struct.
///
/// This is the key trait to implement to improve logging performance. While
/// `Debug` and `Display` usages are eagerly formatted on the hot path,
/// `Serialize` usages copy the minimal required bytes to a separate buffer,
/// and then allow for formatting when flushing elsewhere. Consider ensuring
/// that all logging arguments implement `Serialize` for best performance.
///
/// Furthermore, you would usually not be required to implement `Serialize` by
/// hand for most types. The option that would work for most use cases would be
/// [deriving `Serialize`](crate::Serialize), similar to how `Debug` is
/// derived on user-defined types. Although, do note that all fields on the user
/// struct must also derive/implement `Serialize` (similar to `Debug` again).
///
/// For instance, this would work since all fields have a `Serialize`
/// implementation:
/// ```
/// use quicklog::Serialize;
///
/// #[derive(Serialize)]
/// struct SerializeStruct {
///     a: usize,
///     b: i32,
///     c: &'static str,
/// }
/// ```
///
/// But a field with a type that does not implement `Serialize` will fail to compile:
/// ```compile_fail
/// use quicklog::Serialize;
///
/// struct NoSerializeStruct {
///     a: &'static str,
///     b: &'static str,
/// }
///
/// #[derive(Serialize)]
/// struct SerializeStruct {
///     a: usize,
///     b: i32,
///     // doesn't implement `Serialize`!
///     c: NoSerializeStruct,
/// }
/// ```
pub trait Serialize {
    /// Describes how to encode the implementing type into a byte buffer.
    ///
    /// Returns a [Store](crate::serialize::Store) and the remainder of `write_buf`
    /// passed in that was not written to.
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]);
    /// Describes how to decode the implementing type from a byte buffer.
    ///
    /// Returns a formatted String after parsing the byte buffer, as well as
    /// the remainder of `read_buf` pass in that was not read.
    fn decode(read_buf: &[u8]) -> (String, &[u8]);
    /// The number of bytes required to `encode` the type into a byte buffer.
    fn buffer_size_required(&self) -> usize;
}

/// High-performance, fixed-size serialization for primitive-like types.
///
/// This trait is optimized for selective serialization where types have a known,
/// fixed binary representation. Unlike the general `Serialize` trait, this trait
/// uses compile-time const generics to specify exact byte sizes, enabling
/// significant performance optimizations.
///
/// # Performance Benefits
///
/// - **Compile-time size calculation**: Buffer sizes are computed at compile time
/// - **Zero virtual dispatch**: Direct method calls instead of trait objects
/// - **Optimal memory layout**: Sequential encoding without Store overhead
/// - **Cache-friendly access**: Predictable memory access patterns
///
/// # Example
///
/// ```rust
/// use quicklog::serialize::FixedSizeSerialize;
///
/// pub struct OrderId(u64);
///
/// impl FixedSizeSerialize<8> for OrderId {
///     fn to_le_bytes(&self) -> [u8; 8] {
///         self.0.to_le_bytes()
///     }
///
///     fn from_le_bytes(bytes: [u8; 8]) -> Self {
///         Self(u64::from_le_bytes(bytes))
///     }
/// }
/// ```
///
/// # Usage with Selective Serialization
///
/// Types implementing this trait can be used with the `#[derive(SerializeSelective)]`
/// macro for optimal performance:
///
/// ```rust
/// use quicklog::SerializeSelective;
///
/// #[derive(SerializeSelective)]
/// pub struct Order {
///     #[serialize] pub id: OrderId,    // Uses FixedSizeSerialize
///     #[serialize] pub price: f64,     // Uses FixedSizeSerialize
///     // ... other fields
/// }
/// ```
pub trait FixedSizeSerialize<const N: usize> {
    /// Convert to little-endian byte array.
    ///
    /// This method should produce a deterministic, fixed-size binary
    /// representation of the type suitable for logging and serialization.
    fn to_le_bytes(&self) -> [u8; N];

    /// Convert from little-endian byte array.
    ///
    /// This method should be able to reconstruct the type from the
    /// bytes produced by `to_le_bytes()`.
    fn from_le_bytes(bytes: [u8; N]) -> Self;

    /// The number of bytes required for serialization (always N).
    ///
    /// This is provided as a const for generic programming convenience.
    const BYTE_SIZE: usize = N;
}

/// Function pointer which decodes a byte buffer back into `String` representation
pub type DecodeFn = fn(&[u8]) -> (String, &[u8]);

/// Number of bytes it takes to store the size of a type.
pub const SIZE_LENGTH: usize = std::mem::size_of::<usize>();

/// Contains the decode function required to decode `buffer` back into a `String`
/// representation.
#[derive(Clone)]
pub struct Store<'buf> {
    decode_fn: DecodeFn,
    buffer: &'buf [u8],
}

impl Store<'_> {
    pub fn new(decode_fn: DecodeFn, buffer: &[u8]) -> Store {
        Store { decode_fn, buffer }
    }

    pub fn as_string(&self) -> String {
        let (s, _) = (self.decode_fn)(self.buffer);
        s
    }
}

impl Display for Store<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

macro_rules! gen_serialize {
    ($primitive:ty) => {
        impl Serialize for $primitive {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
                let size = self.buffer_size_required();
                let (x, rest) = write_buf.split_at_mut(size);
                x.copy_from_slice(&self.to_le_bytes());

                (Store::new(Self::decode, x), rest)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(std::mem::size_of::<$primitive>());
                let x = <$primitive>::from_le_bytes(chunk.try_into().unwrap());

                (format!("{}", x), rest)
            }

            fn buffer_size_required(&self) -> usize {
                std::mem::size_of::<$primitive>()
            }
        }
    };
}

gen_serialize!(i32);
gen_serialize!(i64);
gen_serialize!(isize);
gen_serialize!(f32);
gen_serialize!(f64);
gen_serialize!(u32);
gen_serialize!(u64);
gen_serialize!(u128);
gen_serialize!(usize);

/// Macro to generate `FixedSizeSerialize` implementations for primitive types.
///
/// This macro creates implementations that delegate to the primitive type's
/// native `to_le_bytes()` and `from_le_bytes()` methods.
macro_rules! impl_fixed_size_serialize {
    ($($t:ty, $n:expr),* $(,)?) => {
        $(
            impl FixedSizeSerialize<$n> for $t {
                fn to_le_bytes(&self) -> [u8; $n] {
                    <$t>::to_le_bytes(*self)
                }

                fn from_le_bytes(bytes: [u8; $n]) -> Self {
                    <$t>::from_le_bytes(bytes)
                }
            }
        )*
    };
}

// Implement FixedSizeSerialize for all primitive numeric types
impl_fixed_size_serialize! {
    u8, 1,
    i8, 1,
    u16, 2,
    i16, 2,
    u32, 4,
    i32, 4,
    u64, 8,
    i64, 8,
    u128, 16,
    i128, 16,
    usize, 8,   // Assuming 64-bit target
    isize, 8,   // Assuming 64-bit target
    f32, 4,
    f64, 8,
}

/// Macro to generate `FixedSizeSerialize` implementations for newtype wrappers.
///
/// This macro handles the common pattern of wrapper types that delegate
/// to their inner type's `to_le_bytes()` and `from_le_bytes()` methods.
///
/// # Example
///
/// ```rust
/// use quicklog::impl_fixed_size_serialize_newtype;
///
/// pub struct OrderId(u64);
/// impl_fixed_size_serialize_newtype!(OrderId, u64, 8);
///
/// pub struct Price(f64);
/// impl_fixed_size_serialize_newtype!(Price, f64, 8);
/// ```
#[macro_export]
macro_rules! impl_fixed_size_serialize_newtype {
    ($wrapper:ty, $inner:ty, $size:expr) => {
        impl $crate::serialize::FixedSizeSerialize<$size> for $wrapper {
            fn to_le_bytes(&self) -> [u8; $size] {
                self.0.to_le_bytes()
            }

            fn from_le_bytes(bytes: [u8; $size]) -> Self {
                Self(<$inner>::from_le_bytes(bytes))
            }
        }
    };
}

/// Macro to generate `FixedSizeSerialize` implementations for enums.
///
/// This macro handles unit enums with explicit discriminant values,
/// serializing them as single bytes.
///
/// # Example
///
/// ```rust
/// use quicklog::impl_fixed_size_serialize_enum;
///
/// #[repr(u8)]
/// #[derive(Clone, Copy)]
/// pub enum Side {
///     Buy = 0,
///     Sell = 1,
/// }
/// impl_fixed_size_serialize_enum!(Side, Buy = 0, Sell = 1);
///
/// #[repr(u8)]
/// #[derive(Clone, Copy)]
/// pub enum OrderType {
///     Market = 0,
///     Limit = 1,
///     Stop = 2,
/// }
/// impl_fixed_size_serialize_enum!(OrderType, Market = 0, Limit = 1, Stop = 2);
/// ```
#[macro_export]
macro_rules! impl_fixed_size_serialize_enum {
    ($enum_type:ty, $($variant:ident = $value:expr),+ $(,)?) => {
        impl $crate::serialize::FixedSizeSerialize<1> for $enum_type {
            fn to_le_bytes(&self) -> [u8; 1] {
                [*self as u8]
            }

            fn from_le_bytes(bytes: [u8; 1]) -> Self {
                match bytes[0] {
                    $($value => Self::$variant,)+
                    _ => panic!(
                        "Invalid {} discriminant: {}",
                        stringify!($enum_type),
                        bytes[0]
                    ),
                }
            }
        }
    };
}


/// Generates a `Serialize` implementation for unit enums.
///
/// This macro creates a `Serialize` implementation for enums with unit variants
/// (no associated data). It serializes the enum by converting its discriminant
/// to a `u8` value and encoding it as a single byte.
///
/// The enum must have `#[repr(u8)]` to ensure consistent discriminant values
/// and must have no more than 256 variants (0-255).
///
/// # Examples
///
/// ```rust
/// use quicklog::gen_serialize_enum;
///
/// #[repr(u8)]
/// #[derive(Clone, Copy)]
/// enum Color {
///     Red = 0,
///     Green = 1,
///     Blue = 2,
/// }
///
/// gen_serialize_enum!(Color, Red, Green, Blue);
/// ```
///
/// The macro takes the enum type as the first argument, followed by all
/// its variant names. This is necessary to generate the string representation
/// for the `decode` function.
#[macro_export]
macro_rules! gen_serialize_enum {
    ($enum_type:ty, $($variant:ident),+) => {
        impl $crate::serialize::Serialize for $enum_type {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> ($crate::serialize::Store<'buf>, &'buf mut [u8]) {
                let discriminant = *self as u8;
                let size = self.buffer_size_required();
                let (x, rest) = write_buf.split_at_mut(size);
                x.copy_from_slice(&discriminant.to_le_bytes());

                ($crate::serialize::Store::new(Self::decode, x), rest)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(std::mem::size_of::<u8>());
                let discriminant = u8::from_le_bytes(chunk.try_into().unwrap());

                let variant_name = match discriminant {
                    $(
                        x if x == <$enum_type>::$variant as u8 => stringify!($variant),
                    )+
                    _ => "UnknownVariant",
                };

                (variant_name.to_string(), rest)
            }

            fn buffer_size_required(&self) -> usize {
                std::mem::size_of::<u8>()
            }
        }
    };
}

impl Serialize for &str {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let str_len = self.len();
        let (chunk, rest) = write_buf.split_at_mut(str_len + SIZE_LENGTH);
        let (len_chunk, str_chunk) = chunk.split_at_mut(SIZE_LENGTH);

        len_chunk.copy_from_slice(&str_len.to_le_bytes());
        str_chunk.copy_from_slice(self.as_bytes());

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (len_chunk, chunk) = read_buf.split_at(SIZE_LENGTH);
        let str_len = usize::from_le_bytes(len_chunk.try_into().unwrap());

        let (str_chunk, rest) = chunk.split_at(str_len);
        let s = from_utf8(str_chunk).unwrap();

        (s.to_string(), rest)
    }

    fn buffer_size_required(&self) -> usize {
        SIZE_LENGTH + self.len()
    }
}

/// Blanket implementation of Serialize for Option<T> where T implements Serialize
impl<T> Serialize for Option<T>
where
    T: Serialize,
{
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        match self {
            Some(ref value) => {
                let total_size = self.buffer_size_required();
                let (chunk, rest) = write_buf.split_at_mut(total_size);

                // Write Some marker
                chunk[0] = 1;

                // Encode the value after the marker
                let (inner_store, _) = value.encode(&mut chunk[1..]);

                // Create new store that includes the marker
                (Store::new(Self::decode, chunk), rest)
            }
            None => {
                let (chunk, rest) = write_buf.split_at_mut(1);
                chunk[0] = 0; // None marker
                (Store::new(Self::decode, chunk), rest)
            }
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let marker = read_buf[0];
        if marker == 0 {
            // None case
            ("None".to_string(), &read_buf[1..])
        } else {
            // Some case - decode the inner value
            let (inner_string, remaining) = T::decode(&read_buf[1..]);
            (format!("Some({})", inner_string), remaining)
        }
    }

    fn buffer_size_required(&self) -> usize {
        match self {
            Some(ref value) => 1 + value.buffer_size_required(), // marker + value size
            None => 1, // just the marker
        }
    }
}

/// Eager evaluation into a String for debug structs
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &mut [u8]) -> (Store, &mut [u8]) {
    let val_string = format!("{:?}", val);
    let str_len = val_string.len();

    let (chunk, rest) = write_buf.split_at_mut(str_len + SIZE_LENGTH);
    let (len_chunk, str_chunk) = chunk.split_at_mut(SIZE_LENGTH);
    len_chunk.copy_from_slice(&str_len.to_le_bytes());
    str_chunk.copy_from_slice(val_string.as_bytes());

    (Store::new(<&str as Serialize>::decode, chunk), rest)
}

#[cfg(test)]
mod tests {
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
        let (store, _) = some_value.encode(&mut buf);

        // Verify encoding
        assert_eq!(buf[0], 1); // Some marker
        assert_eq!(&buf[1..5], &42i32.to_le_bytes()); // i32 value

        // Verify decoding
        assert_eq!(store.as_string(), "Some(42)");

        // Verify buffer size
        assert_eq!(some_value.buffer_size_required(), 5); // 1 marker + 4 bytes for i32
    }

    #[test]
    fn serialize_option_none() {
        let mut buf = [0; 128];

        // Test Option<i32> with None value
        let none_value: Option<i32> = None;
        let (store, _) = none_value.encode(&mut buf);

        // Verify encoding
        assert_eq!(buf[0], 0); // None marker

        // Verify decoding
        assert_eq!(store.as_string(), "None");

        // Verify buffer size
        assert_eq!(none_value.buffer_size_required(), 1); // Just the marker
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
        assert_eq!(original_some.buffer_size_required(), 9); // 1 marker + 8 bytes for u64
        assert_eq!(original_none.buffer_size_required(), 1); // Just marker
    }
}

/// Declarative macro for generating selective Serialize implementations
///
/// This provides a fallback when proc macros can't be used or for more complex scenarios.
/// It generates a Serialize implementation that only includes specified fields.
///
/// # Example
///
/// ```rust
/// use quicklog::serialize_selected_fields;
///
/// #[derive(Debug)]
/// struct Order {
///     pub oid: u64,
///     pub cloid: Option<u64>,
///     pub price: Option<f64>,
///     pub size: f64,
///     pub status: String,  // Not included in serialization
/// }
///
/// serialize_selected_fields!(Order, {
///     oid: u64,
///     cloid: Option<u64>,
///     price: Option<f64>,
///     size: f64
/// });
/// ```
#[macro_export]
macro_rules! serialize_selected_fields {
    ($struct_name:ty, { $($field:ident: $field_type:ty),* $(,)? }) => {
        impl $crate::serialize::Serialize for $struct_name {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> ($crate::serialize::Store<'buf>, &'buf mut [u8]) {
                let total_size = self.buffer_size_required();
                let (chunk, rest) = write_buf.split_at_mut(total_size);
                let mut offset = 0;

                $(
                    $crate::__encode_field_impl!(self.$field, chunk, offset, $field_type);
                )*

                ($crate::serialize::Store::new(Self::decode, chunk), rest)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let mut offset = 0;
                let mut parts = Vec::new();

                $(
                    $crate::__decode_field_impl!(stringify!($field), read_buf, offset, parts, $field_type);
                )*

                let formatted = parts.join(" ");
                (formatted, &read_buf[offset..])
            }

            fn buffer_size_required(&self) -> usize {
                let mut total = 0;
                $(
                    total += $crate::__field_size_impl!(self.$field, $field_type);
                )*
                total
            }
        }
    };
}

/// Internal helper macro for encoding fields (handles Option<T> and direct types)
#[doc(hidden)]
#[macro_export]
macro_rules! __encode_field_impl {
    // Handle Option<T> types
    ($field:expr, $chunk:expr, $offset:expr, Option<$inner:ty>) => {
        if let Some(ref value) = $field {
            $chunk[$offset] = 1; // Some marker
            $offset += 1;
            let bytes = value.to_le_bytes();
            $chunk[$offset..$offset + bytes.len()].copy_from_slice(&bytes);
            $offset += bytes.len();
        } else {
            $chunk[$offset] = 0; // None marker
            $offset += 1;
        }
    };
    // Handle direct types that implement to_le_bytes
    ($field:expr, $chunk:expr, $offset:expr, $field_type:ty) => {
        let bytes = $field.to_le_bytes();
        $chunk[$offset..$offset + bytes.len()].copy_from_slice(&bytes);
        $offset += bytes.len();
    };
}

/// Internal helper macro for decoding fields
#[doc(hidden)]
#[macro_export]
macro_rules! __decode_field_impl {
    // Handle Option<T> types
    ($field_name:expr, $read_buf:expr, $offset:expr, $parts:expr, Option<$inner:ty>) => {
        let has_value = $read_buf[$offset] != 0;
        $offset += 1;
        if has_value {
            let value = <$inner>::from_le_bytes(
                $read_buf[$offset..$offset + std::mem::size_of::<$inner>()].try_into().unwrap()
            );
            $parts.push(format!("{}={}", $field_name, value));
            $offset += std::mem::size_of::<$inner>();
        } else {
            $parts.push(format!("{}=None", $field_name));
        }
    };
    // Handle direct types
    ($field_name:expr, $read_buf:expr, $offset:expr, $parts:expr, $field_type:ty) => {
        let value = <$field_type>::from_le_bytes(
            $read_buf[$offset..$offset + std::mem::size_of::<$field_type>()].try_into().unwrap()
        );
        $parts.push(format!("{}={}", $field_name, value));
        $offset += std::mem::size_of::<$field_type>();
    };
}

/// Internal helper macro for calculating field sizes
#[doc(hidden)]
#[macro_export]
macro_rules! __field_size_impl {
    // Handle Option<T> types
    ($field:expr, Option<$inner:ty>) => {
        1 + $field.map_or(0, |_| std::mem::size_of::<$inner>())
    };
    // Handle direct types
    ($field:expr, $field_type:ty) => {
        std::mem::size_of::<$field_type>()
    };
}
