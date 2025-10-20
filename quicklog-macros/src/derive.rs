use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Type};

/// Generates a `quicklog` `Serialize` implementation for a user-defined struct.
///
/// There is no new real logic in the generated `encode` and `decode` functions
/// for the struct. The macro simply walks every field of the struct and
/// sequentially calls `encode` or `decode` corresponding to the `Serialize`
/// implementation for the type of the field.
///
/// For instance:
/// ```ignore
/// use quicklog::Serialize;
///
/// #[derive(Serialize)]
/// struct TestStruct {
///     a: usize,
///     b: i32,
///     c: u32,
/// }
///
/// // Generated code
/// impl quicklog::serialize::Serialize for TestStruct {
///     fn encode<'buf>(
///         &self,
///         write_buf: &'buf mut [u8],
///     ) -> quicklog::serialize::Store<'buf> {
///         let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
///         let (_, chunk_rest) = self.a.encode(chunk);
///         let (_, chunk_rest) = self.b.encode(chunk_rest);
///         let (_, chunk_rest) = self.c.encode(chunk_rest);
///         assert!(chunk_rest.is_empty());
///         (quicklog::serialize::Store::new(Self::decode, chunk), rest)
///     }
///     fn decode(read_buf: &[u8]) -> (String, &[u8]) {
///         let (a, read_buf) = <usize as quicklog::serialize::Serialize>::decode(read_buf);
///         let (b, read_buf) = <i32 as quicklog::serialize::Serialize>::decode(read_buf);
///         let (c, read_buf) = <u32 as quicklog::serialize::Serialize>::decode(read_buf);
///         (
///             {
///                 let res = ::alloc::fmt::format(format_args!("{0} {1} {2}", a, b, c));
///                 res
///             },
///             read_buf,
///         )
///     }
///     fn buffer_size_required(&self) -> usize {
///         self.a.buffer_size_required() + self.b.buffer_size_required()
///             + self.c.buffer_size_required()
///     }
/// }
/// ```
pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Data::Struct(DataStruct { fields, .. }) = input.data else {
        todo!("Deriving Serialize only supported for structs currently")
    };

    if fields.is_empty() {
        return quote! {}.into();
    }

    // Handle both named fields (regular structs) and unnamed fields (tuple structs)
    let field_accessors: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            if let Some(name) = &field.ident {
                quote! { #name } // Named field: self.field_name
            } else {
                let index = syn::Index::from(i);
                quote! { #index } // Unnamed field: self.0, self.1, etc.
            }
        })
        .collect();

    // If we have > 1 field, then we split once at the top-level to get the
    // single chunk that has enough capacity to encode all the fields.
    // From there, each field will just encode into this single chunk.
    //
    // Otherwise, if we only have 1 field, we can simply let the single field
    // directly read off the main `write_buf` chunk and return the remainder
    // unread.
    let (initial_chunk_split, chunk_encode_and_store): (TokenStream2, TokenStream2) =
        if field_accessors.len() > 1 {
            // Split off just large enough chunk to be kept in final Store
            let initial_split = quote! {
                let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
            };

            // Sequentially encode
            let encode: Vec<_> = field_accessors
                .iter()
                .enumerate()
                .map(|(idx, accessor)| {
                    if idx == 0 {
                        quote! {
                            let (_, chunk_rest) = self.#accessor.encode(chunk);
                        }
                    } else {
                        quote! {
                            let (_, chunk_rest) = self.#accessor.encode(chunk_rest);
                        }
                    }
                })
                .collect();

            let encode_and_store = quote! {
                #(#encode)*

                assert!(chunk_rest.is_empty());
                (quicklog::serialize::Store::new(Self::decode, chunk), rest)
            };

            (initial_split, encode_and_store)
        } else {
            let initial_split = quote! {
                let chunk = write_buf;
            };

            // Only one field, so can directly encode in main chunk
            let field_accessor = &field_accessors[0];
            let encode_and_store = quote! {
                self.#field_accessor.encode(chunk)
            };

            (initial_split, encode_and_store)
        };

    // Combine decode implementations from all field types
    let field_tys: Vec<_> = fields
         .iter()
         .enumerate()
         .map(|(i, field)| {
             let mut field_ty = field.ty.clone();
             if let Type::Reference(ty_ref) = &mut field_ty {
                 _ = ty_ref.lifetime.take();
                 _ = ty_ref.mutability.take();
             }

             // Create a unique variable name for each decoded field
             let decoded_ident = if let Some(name) = &field.ident {
                 // Named field: use the field name
                 Ident::new(&format!("{}", name), name.span())
             } else {
                 // Unnamed field: use field_0, field_1, etc.
                 Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site())
             };

             quote! {
                 let (#decoded_ident, read_buf) = <#field_ty as quicklog::serialize::Serialize>::decode(read_buf);
             }
         })
         .collect();

    // Create variable names for the format string
    let decode_var_names: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            if let Some(name) = &field.ident {
                // Named field: use the field name
                Ident::new(&format!("{}", name), name.span())
            } else {
                // Unnamed field: use field_0, field_1, etc.
                Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site())
            }
        })
        .collect();

    // Assuming that each field in the output should just be separated by a space
    // TODO: proper field naming?
    let mut decode_fmt_str = String::new();
    for _ in 0..fields.len() {
        decode_fmt_str.push_str("{} ");
    }
    let decode_fmt_str = decode_fmt_str.trim_end();

    quote! {
         impl #impl_generics quicklog::serialize::Serialize for #struct_name #ty_generics #where_clause {
             fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (quicklog::serialize::Store<'buf>, &'buf mut [u8]) {
                 // Perform initial split to get combined byte buffer that will be
                 // sufficient for all fields to be encoded in
                 #initial_chunk_split

                 #chunk_encode_and_store
             }

             fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                 #(#field_tys)*

                 (format!(#decode_fmt_str, #(#decode_var_names),*), read_buf)
             }

             fn buffer_size_required(&self) -> usize {
                 #(self.#field_accessors.buffer_size_required())+*
             }
         }
     }
     .into()
}
