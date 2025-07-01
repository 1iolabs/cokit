use crate::co::CoMacroFeature;
use proc_macro::TokenStream;
use std::collections::BTreeSet;

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

#[proc_macro_attribute]
pub fn co(metadata: TokenStream, input: TokenStream) -> TokenStream {
	// flags
	let mut features = BTreeSet::new();
	let args: syn::AttributeArgs = syn::parse_macro_input!(metadata as syn::AttributeArgs);
	for arg in &args {
		let flag = match arg {
			syn::NestedMeta::Meta(syn::Meta::Path(path)) => path.get_ident().map(|id| id.to_string()),
			other => {
				return syn::Error::new_spanned(other, "Expected flag-style identifiers")
					.to_compile_error()
					.into();
			},
		};
		if let Some(flag) = flag {
			match CoMacroFeature::try_from(flag.as_str()) {
				Ok(flag) => {
					features.insert(flag);
				},
				Err(err) => {
					return err.to_compile_error().into();
				},
			}
		}
	}

	// generate
	co::macro_co(input, features)
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
