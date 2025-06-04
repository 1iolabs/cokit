use quote::quote;
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

		#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
		#[no_mangle]
		pub extern "C" fn state() {
			co_api::async_api::reduce::<#name, _>()
		}
	};

	proc_macro::TokenStream::from(expanded)
}
