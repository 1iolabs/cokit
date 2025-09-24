fn main() {
	for (k, v) in std::env::vars() {
		println!("cargo:warning={}={}", k, v);
	}
}
