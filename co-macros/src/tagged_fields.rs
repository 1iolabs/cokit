// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Fields, ItemStruct, Path, Token};

pub fn derive_tagged_fields(item: TokenStream) -> TokenStream {
	let s = parse_macro_input!(item as ItemStruct);
	let ident = s.ident;
	let mut external: Vec<String> = vec![];
	if let Fields::Named(fields_named) = &s.fields {
		for field in &fields_named.named {
			let field_ident = match &field.ident {
				Some(ident) => ident,
				None => continue,
			};
			for attr in &field.attrs {
				if let Ok(flags) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated) {
					for flag in &flags {
						if let Some(flag) = flag.get_ident().map(|id| id.to_string()) {
							match flag.as_str() {
								"external" => {
									external.push(field_ident.to_string());
								},
								other => {
									return syn::Error::new_spanned(attr, format!("Unknown flag: {other}"))
										.to_compile_error()
										.into();
								},
							}
						}
					}
				}
			}
		}
	}

	TokenStream::from(quote! {
		impl CoMetadata for #ident {
			fn metadata() -> Vec<Metadata> {
				// metadata vector that will be returned
				let mut metadata: Vec<Metadata> = vec![];
				// build vector for fields tagged 'external'
				let external_metadata = vec![#(#external.to_owned()),*];
				// add external to metadata vector if entries exist
				if external_metadata.len() > 0 {
					metadata.push(Metadata::External(external_metadata));
				}
				return metadata;
			}
		}
	})
}
