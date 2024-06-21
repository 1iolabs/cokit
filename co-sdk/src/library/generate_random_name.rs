use names::{Generator, Name};

pub fn generate_random_name(max_length: usize) -> String {
	loop {
		let node_name = Generator::with_naming(Name::Numbered).next().expect("RNG is available");
		if node_name.chars().count() < max_length {
			return node_name;
		}
	}
}
