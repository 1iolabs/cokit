// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitInt};

pub fn macro_timeout(attr: TokenStream, item: TokenStream) -> TokenStream {
	let millis = parse_macro_input!(attr as LitInt);
	let mut input = parse_macro_input!(item as ItemFn);

	if input.sig.asyncness.is_none() {
		return syn::Error::new_spanned(
			&input.sig,
			"#[timeout] must be applied to an async function. \
			 When used together with #[tokio::test], place #[co_macros::timeout(...)] above #[tokio::test].",
		)
		.to_compile_error()
		.into();
	}

	let millis_value: u64 = match millis.base10_parse() {
		Ok(v) => v,
		Err(err) => return err.to_compile_error().into(),
	};
	let msg = format!("test timed out after {millis_value}ms");
	let body = &input.block;

	let new_block: syn::Block = syn::parse_quote! {
		{
			::tokio::time::timeout(
				::std::time::Duration::from_millis(#millis_value),
				async move #body
			)
			.await
			.expect(#msg)
		}
	};

	*input.block = new_block;
	quote!(#input).into()
}
