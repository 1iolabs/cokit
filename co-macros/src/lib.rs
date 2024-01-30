use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenTree};
use quote::quote;
use syn::{parse, parse::Parser, parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn common_event_content(args: TokenStream, input: TokenStream) -> TokenStream {
	let mut s = parse_macro_input!(input as ItemStruct);
	let _ = parse_macro_input!(args as parse::Nothing);

	if let syn::Fields::Named(ref mut fields) = s.fields {
		fields.named.push(
			syn::Field::parse_named
				.parse2(quote! { #[serde(skip_serializing_if = "Option::is_none")] pub is_silent: Option<bool> })
				.unwrap(),
		);
		fields.named.push(
			syn::Field::parse_named
				.parse2(quote! { #[serde(skip_serializing_if = "Option::is_none")] pub relates_to: Option<RelatesTo> })
				.unwrap(),
		);
		fields.named.push(
			syn::Field::parse_named
				.parse2(
					quote! { #[serde(skip_serializing_if = "Option::is_none", rename = "m.new_content")] pub new_content: Option<Box<EventContent>> },
				)
				.unwrap(),
		);
	}

	return quote! {
		#s
	}
	.into();
}

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
						TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis =>
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
