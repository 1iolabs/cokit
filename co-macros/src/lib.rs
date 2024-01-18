use proc_macro::TokenStream;
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
	}

	return quote! {
		#s
	}
	.into();
}
