use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenTree};
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn derive_tagged_fields(item: TokenStream) -> TokenStream {
	let s = parse_macro_input!(item as ItemStruct);
	let ident = s.ident;
	let mut external: Vec<String> = vec![];
	for field in s.fields.into_iter() {
		let field_ident = match field.ident {
			Some(ident) => ident,
			None => continue,
		};
		for attr in field.attrs.into_iter() {
			if attr.path.is_ident("tagged") {
				// found tagged attribute
				for token in attr.clone().tokens.into_iter() {
					match token {
						TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => {
							for group_item in g.stream().into_iter() {
								match group_item {
									TokenTree::Ident(ident) => {
										let ident_string = ident.to_string();
										if ident_string == "external" {
											external.push(field_ident.to_string());
										}
										// add new tagged() options here
									},
									_ => panic!("Invalid attribute usage"),
								}
							}
						},
						_ => panic!("Invalid attribute usage"),
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
