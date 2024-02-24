fn main() {
    // Tells rust program linker where to find the dynamic librarys
    // *.dll and/or *.so files should be in this folder
    // Will probably have this be in .config/config.toml in future if needed
    //println!("cargo:rustc-link-search=./bin");

    if let Ok(tgt) = std::env::var("TARGET") {
        //println!("Build Target: {}", tgt);
        match tgt.as_str() {
            "x86_64-pc-windows-gnu" => {
                // Windows 64-bit
                //std::env::set_var("CARGO_TARGET_DIR", "./target/win-x64");
            }
            "x86_64-pc-windows-msvc" => {
                // Windows 64-bit Alt
            }
            "aarch64-pc-windows-msvc" => {
                // Windows 64-bit Arm
            }
            "x86_64-unknown-linux-gnu" => {
                // Linux 64-bit
            }
            "aarch64-unknown-linux-gnu" => {
                // Linux 64-bit Arm
            }
            "aarch64-apple-darwin" => {
                // MacOS 64-bit Arm
            }
            "x86_64-apple-darwin" => {
                // MacOS 64-bit Intel/Legacy
            }
            _ => {
                panic!("Not a valid build target!");
            }
        }
    }

    //println!("Building for: {:?}", std::env::var("TARGET"));
}
