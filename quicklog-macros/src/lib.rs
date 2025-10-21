use proc_macro::TokenStream;

mod args;
mod derive;
mod expand;
mod format_arg;
mod quicklog;
mod selective_serialize;

use derive::derive;
use expand::expand;
use quicklog::Level;

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input)
}

#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input)
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(Level::Info, input)
}

#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input)
}

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(Level::Error, input)
}

/// Derive macro for generating `quicklog` `Serialize`
/// implementations.
#[proc_macro_derive(Serialize)]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    derive(input)
}

/// Derive macro for generating selective `quicklog` `Serialize`
/// implementations. Only fields marked with `#[serialize]` are included.
///
/// This is useful for large structs where you only want to log specific fields
/// to reduce serialization overhead in high-performance scenarios.
///
/// # Example
///
/// ```rust
/// use quicklog::SerializeSelective;
///
/// #[derive(SerializeSelective)]
/// pub struct Order {
///     #[serialize] pub oid: u64,
///     #[serialize] pub cloid: Option<u64>,
///     #[serialize] pub price: Option<f64>,
///     #[serialize] pub size: f64,
///     // These fields will NOT be serialized
///     pub status: OrderStatus,
///     pub filled_size: f64,
/// }
/// ```
#[proc_macro_derive(SerializeSelective, attributes(serialize))]
pub fn derive_serialize_selective(input: TokenStream) -> TokenStream {
    selective_serialize::derive_selective_serialize(input)
}
