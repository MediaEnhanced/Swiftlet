// Media Enhanced Swiftlet Build Script

fn main() {
    // Get the Host and Target Triple Strings
    let host_string = match std::env::var("HOST") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not get the host triple string from the Cargo build script set environment variable: {}", e);
        }
    };
    let target_string = match std::env::var("TARGET") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not get the target triple string from the Cargo build script set environment variable: {}", e);
        }
    };

    if host_string != target_string {
        panic!("Cross Compiling Not Currently Supported!");
    }

    // Distributed dynamic library search paths could be added here based on the target and program requirements
    //println!("cargo:rustc-link-search=bin/lib/");

    //panic!("Panic and Print Build Debug");
}
