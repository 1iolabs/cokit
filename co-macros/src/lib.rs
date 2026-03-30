// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::co::CoMacroFeature;
use proc_macro::TokenStream;
use std::collections::BTreeSet;
use syn::{
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
	Meta, Token,
};

mod co;
mod tagged_fields;

struct CoArgs {
	features: BTreeSet<CoMacroFeature>,
}
impl Parse for CoArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
		let mut features = BTreeSet::new();
		for arg in &args {
			let flag = match arg {
				Meta::Path(path) => path.get_ident().map(|id| id.to_string()),
				other => {
					return Err(syn::Error::new_spanned(other, "Expected flag-style identifiers"));
				},
			};
			if let Some(flag) = flag {
				match CoMacroFeature::try_from(flag.as_str()) {
					Ok(flag) => {
						features.insert(flag);
					},
					Err(err) => {
						return Err(err);
					},
				}
			}
		}
		Ok(Self { features })
	}
}

#[proc_macro_attribute]
pub fn co(metadata: TokenStream, input: TokenStream) -> TokenStream {
	// flags
	let args = syn::parse_macro_input!(metadata as CoArgs);

	// generate
	co::macro_co(input, args.features)
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
	tagged_fields::derive_tagged_fields(item)
}
