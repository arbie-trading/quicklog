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
///     a: &'static str,
///     b: Option<&'static str>,
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
/// # Requirements
///
/// Types implementing this trait must also implement `Display` for formatting
/// during the decode phase at flush time.
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
/// use std::fmt;
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
///
/// impl fmt::Display for OrderId {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.0)
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
///     #[serialize] pub id: u64,        // Uses FixedSizeSerialize
///     #[serialize] pub price: f64,     // Uses FixedSizeSerialize
///     // ... other fields
/// }
/// ```
pub trait FixedSizeSerialize<const N: usize>: std::fmt::Display {
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

// Primitive Serialize implementations removed - primitives should use
// FixedSizeSerialize for selective serialization, or be logged unprefixed
// for best performance (since they're Copy).

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
/// **Note**: Since `FixedSizeSerialize` now requires `Display`, you must also implement
/// `Display` separately. For convenience, consider using `impl_serializable_newtype!`
/// which implements both traits.
///
/// # Example
///
/// ```rust
/// use quicklog::{impl_fixed_size_serialize_newtype, impl_serializable_newtype};
/// use std::fmt;
///
/// // Recommended: Use impl_serializable_newtype! instead
/// pub struct OrderId(u64);
/// impl_serializable_newtype!(OrderId, u64, 8);
///
/// // Or implement Display manually if you need custom formatting
/// pub struct Price(f64);
/// impl_fixed_size_serialize_newtype!(Price, f64, 8);
/// impl fmt::Display for Price {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "${:.2}", self.0)
///     }
/// }
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

/// Macro to generate both `FixedSizeSerialize` and `Display` implementations for newtype wrappers.
///
/// This is a convenience macro that implements both required traits for selective serialization
/// in a single invocation. It delegates to the inner type's implementations.
///
/// # Example
///
/// ```rust
/// use quicklog::impl_serializable_newtype;
///
/// pub struct OrderId(u64);
/// impl_serializable_newtype!(OrderId, u64, 8);
/// // Now OrderId implements both FixedSizeSerialize<8> and Display
///
/// pub struct Price(f64);
/// impl_serializable_newtype!(Price, f64, 8);
/// ```
///
/// # Generated Code
///
/// This macro expands to:
/// ```rust
/// # pub struct OrderId(u64);
/// impl quicklog::serialize::FixedSizeSerialize<8> for OrderId {
///     fn to_le_bytes(&self) -> [u8; 8] {
///         self.0.to_le_bytes()
///     }
///     fn from_le_bytes(bytes: [u8; 8]) -> Self {
///         Self(u64::from_le_bytes(bytes))
///     }
/// }
///
/// impl std::fmt::Display for OrderId {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         std::fmt::Display::fmt(&self.0, f)
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_serializable_newtype {
    ($wrapper:ty, $inner:ty, $size:expr) => {
        impl $crate::serialize::FixedSizeSerialize<$size> for $wrapper {
            fn to_le_bytes(&self) -> [u8; $size] {
                self.0.to_le_bytes()
            }

            fn from_le_bytes(bytes: [u8; $size]) -> Self {
                Self(<$inner>::from_le_bytes(bytes))
            }
        }

        impl std::fmt::Display for $wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

/// Macro to generate `FixedSizeSerialize` implementations for enums.
///
/// This macro handles unit enums with explicit discriminant values,
/// serializing them as single bytes.
///
/// **Note**: Since `FixedSizeSerialize` requires `Display`, you must also implement
/// `Display` for your enum.
///
/// # Example
///
/// ```rust
/// use quicklog::impl_fixed_size_serialize_enum;
/// use std::fmt;
///
/// #[repr(u8)]
/// #[derive(Clone, Copy)]
/// pub enum Side {
///     Buy = 0,
///     Sell = 1,
/// }
/// impl_fixed_size_serialize_enum!(Side, Buy = 0, Sell = 1);
///
/// impl fmt::Display for Side {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         match self {
///             Side::Buy => write!(f, "Buy"),
///             Side::Sell => write!(f, "Sell"),
///         }
///     }
/// }
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

// gen_serialize_enum! removed - use impl_fixed_size_serialize_enum! instead
// for selective serialization with FixedSizeSerialize trait

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
                let (_inner_store, _) = value.encode(&mut chunk[1..]);

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
            None => 1,                                           // just the marker
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
mod tests;
