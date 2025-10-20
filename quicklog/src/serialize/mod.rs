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

#[macro_export]
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

/// Generates a `Serialize` implementation for unit enums.
///
/// This macro creates a `Serialize` implementation for enums with unit variants
/// (no associated data). It serializes the enum by converting its discriminant
/// to a `usize` value and encoding it as little-endian bytes.
///
/// The enum must have `#[repr(usize)]` to ensure consistent discriminant values.
///
/// # Examples
///
/// ```rust
/// use quicklog::gen_serialize_enum;
///
/// #[repr(usize)]
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
        impl Serialize for $enum_type {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> ($crate::serialize::Store<'buf>, &'buf mut [u8]) {
                let discriminant = *self as usize;
                let size = self.buffer_size_required();
                let (x, rest) = write_buf.split_at_mut(size);
                x.copy_from_slice(&discriminant.to_le_bytes());

                ($crate::serialize::Store::new(Self::decode, x), rest)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(std::mem::size_of::<usize>());
                let discriminant = usize::from_le_bytes(chunk.try_into().unwrap());

                let variant_name = match discriminant {
                    $(
                        x if x == <$enum_type>::$variant as usize => stringify!($variant),
                    )+
                    _ => "UnknownVariant",
                };

                (variant_name.to_string(), rest)
            }

            fn buffer_size_required(&self) -> usize {
                std::mem::size_of::<usize>()
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
        #[repr(usize)]
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
        #[repr(usize)]
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
        assert_eq!(active.buffer_size_required(), std::mem::size_of::<usize>());
    }

    #[test]
    fn serialize_multiple_enums() {
        #[repr(usize)]
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
        #[repr(usize)]
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
        let discriminant = original as usize;
        let expected_bytes = discriminant.to_le_bytes();
        assert_eq!(&buf[0..std::mem::size_of::<usize>()], &expected_bytes);
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
        assert_eq!(Level::Trace as usize, 0);
        assert_eq!(Level::Debug as usize, 1);
        assert_eq!(Level::Info as usize, 2);
        assert_eq!(Level::Warn as usize, 3);
        assert_eq!(Level::Error as usize, 4);
    }
}
