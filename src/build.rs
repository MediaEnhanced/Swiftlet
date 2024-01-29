fn main() {
    // Tells rust program linker where to find the dynamic librarys
    // *.dll and/or *.so files should be in this folder
    println!("cargo:rustc-link-search=./bin");
}
