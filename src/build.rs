// Rust Cross Compile Support

fn main() {
    let host_str = match std::env::var("HOST") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not find host triple: {}", e);
        }
    };
    let target_str = match std::env::var("TARGET") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not find target triple: {}", e);
        }
    };
    if target_str != host_str {
        // Cross Compiling
        // Each Target should set the appropriate OS library link search paths
        // and dynamic library search path if necessary
        // For now it is based on zig helper files
        match target_str.as_str() {
            "x86_64-pc-windows-gnu" => {
                // Windows 64-bit
                if host_str.as_str() == "x86_64-pc-windows-msvc" {
                    // Partial cross compiling
                }
            }
            "x86_64-pc-windows-msvc" => {
                // Windows 64-bit Alt
                if host_str.as_str() == "x86_64-pc-windows-gnu" {
                    // Partial cross compiling
                }
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
                panic!("Not a valid build target: {}", target_str);
            }
        }
    }
}
