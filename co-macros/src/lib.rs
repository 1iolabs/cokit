use proc_macro::TokenStream;

mod co;
mod tagged_fields;

#[proc_macro_attribute]
pub fn co_data(_metadata: TokenStream, input: TokenStream) -> TokenStream {
	co::macro_co_data(input)
}

#[proc_macro_attribute]
pub fn co_state(_metadata: TokenStream, input: TokenStream) -> TokenStream {
	co::macro_co_state(input)
}

// #[proc_macro_derive(CoData)]
// pub fn derive_co_data(input: TokenStream) -> TokenStream {
// 	co::macro_co_data(input)
// }

/**
 * Attribute usage:
 * #[tagged(<tag>)]
 * Currently options for tag consist of:
 * - external
 *
 * More may be added in the future
 */
#[proc_macro_derive(TaggedFields, attributes(tagged))]
pub fn derive_tagged_fields(item: TokenStream) -> TokenStream {
	tagged_fields::derive_tagged_fields(item)
}
