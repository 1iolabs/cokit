// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use names::{Generator, Name};

pub fn generate_random_name(max_length: usize) -> String {
	loop {
		let node_name = Generator::with_naming(Name::Numbered).next().expect("RNG is available");
		if node_name.chars().count() < max_length {
			return node_name;
		}
	}
}
