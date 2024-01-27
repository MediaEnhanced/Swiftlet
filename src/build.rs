fn main() {
	// Tells rust program linker where to find the dynamic librarys
	// msquic.dll and/or libmsquic.so should be in the folder for msquic.rs
	println!("cargo:rustc-link-search=./bin");
}

