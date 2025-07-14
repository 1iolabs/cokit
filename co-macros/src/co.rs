use quote::quote;
use std::collections::BTreeSet;
use syn::{parse_macro_input, DeriveInput};

pub fn macro_co_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	// let _name = &input.ident;
	// let input: proc_macro2::TokenStream = input.into();

	let expanded = quote! {
		#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
		#input
	};

	proc_macro::TokenStream::from(expanded)
}

pub fn macro_co_state(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	let name = &input.ident;

	let expanded = quote! {
		#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
		#input

		#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
		#[no_mangle]
		pub extern "C" fn state() {
			co_api::async_api::reduce::<#name, _>()
		}
	};

	proc_macro::TokenStream::from(expanded)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoMacroFeature {
	State,
	Guard,
	StateSync,
	NoDefault,
	NoDerive,
	Repr,
}
impl TryFrom<&str> for CoMacroFeature {
	type Error = syn::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		Ok(match value {
			"state" => Self::State,
			"state_sync" => Self::StateSync,
			"guard" => Self::Guard,
			"no_default" => Self::NoDefault,
			"no_derive" => Self::NoDerive,
			"repr" => Self::Repr,
			other => {
				return Err(syn::Error::new_spanned(other, format!("Unknown flag: {}", other)));
			},
		})
	}
}

pub fn macro_co(input: proc_macro::TokenStream, features: BTreeSet<CoMacroFeature>) -> proc_macro::TokenStream {
	// input
	let input = parse_macro_input!(input as DeriveInput);
	let name = &input.ident;

	// derives
	let derives = if !features.contains(&CoMacroFeature::NoDerive) {
		let mut derives: Vec<syn::Path> = vec![
			syn::parse_quote!(Debug),
			syn::parse_quote!(Clone),
			syn::parse_quote!(Hash),
			syn::parse_quote!(PartialEq),
			syn::parse_quote!(Eq),
			syn::parse_quote!(PartialOrd),
			syn::parse_quote!(Ord),
		];
		if features.contains(&CoMacroFeature::Repr) {
			derives.push(syn::parse_quote!(Copy));
			derives.push(syn::parse_quote!(serde_repr::Serialize_repr));
			derives.push(syn::parse_quote!(serde_repr::Deserialize_repr));
		} else {
			derives.push(syn::parse_quote!(serde::Serialize));
			derives.push(syn::parse_quote!(serde::Deserialize));
		}
		if !features.contains(&CoMacroFeature::NoDefault) {
			if features.contains(&CoMacroFeature::State) || features.contains(&CoMacroFeature::StateSync) {
				derives.push(syn::parse_quote!(Default));
			}
		}
		derives
	} else {
		Default::default()
	};

	// feature: state
	let mut tokens = Vec::new();
	if features.contains(&CoMacroFeature::State) {
		tokens.push(quote! {
			#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
			#[no_mangle]
			pub extern "C" fn state() {
				co_api::async_api::reduce::<#name, _>()
			}
		});
	}

	// feature: state sync
	if features.contains(&CoMacroFeature::StateSync) {
		tokens.push(quote! {
			#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
			#[no_mangle]
			pub extern "C" fn state() {
				co_api::reduce::<#name>()
			}
		});
	}

	// feature: guard
	if features.contains(&CoMacroFeature::Guard) {
		tokens.push(quote! {
			#[cfg(all(feature = "core", target_arch = "wasm32", target_os = "unknown"))]
			#[no_mangle]
			pub extern "C" fn guard() -> bool {
				co_api::guard::<#name>()
			}
		});
	}

	// result
	let expanded = quote! {
		#[derive(#(#derives),*)]
		#input

		#(#tokens)*
	};

	proc_macro::TokenStream::from(expanded)
}
