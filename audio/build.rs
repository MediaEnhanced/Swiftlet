// Media Enhanced Swiftlet Audio Build Script with Cross Compile Support

#![allow(dead_code)]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

struct CrossCompileInfo {
    zcc: &'static str,
    zar: &'static str,
    zig_target: &'static str,
}

impl CrossCompileInfo {
    fn new(host_str: &str, target_str: &str) -> Option<Self> {
        if host_str == target_str {
            return None;
        }

        let (zcc, zar) = match host_str {
            "x86_64-pc-windows-gnu" => {
                // Windows 64-bit
                ("../zig/zcc.bat", "../zig/zar.bat")
            }
            "x86_64-pc-windows-msvc" => {
                // Windows 64-bit Alt
                ("../zig/zcc.bat", "../zig/zar.bat")
            }
            "aarch64-pc-windows-msvc" => {
                // Windows 64-bit Arm
                ("../zig/zcc.bat", "../zig/zar.bat")
            }
            "x86_64-unknown-linux-gnu" => {
                // Linux 64-bit
                ("../zig/zcc.sh", "../zig/zar.sh")
            }
            "aarch64-unknown-linux-gnu" => {
                // Linux 64-bit Arm
                ("../zig/zcc.sh", "../zig/zar.sh")
            }
            "aarch64-apple-darwin" => {
                // MacOS 64-bit Arm
                ("../zig/zcc.sh", "../zig/zar.sh")
            }
            "x86_64-apple-darwin" => {
                // MacOS 64-bit Intel/Legacy
                ("../zig/zcc.sh", "../zig/zar.sh")
            }
            _ => {
                panic!(
                    "This host platform, {}, is currently unsupported for cross compiling/linking!",
                    host_str
                );
            }
        };

        let zig_target = match target_str {
            "x86_64-pc-windows-gnu" => {
                // Windows 64-bit
                "--target=x86_64-windows"
            }
            // "x86_64-pc-windows-msvc" => {
            //     // Windows 64-bit Alt
            //     "--target=x86_64-windows"
            // }
            // "aarch64-pc-windows-msvc" => {
            //     // Windows 64-bit Arm
            //     "--target=aarch64-windows"
            // }
            "x86_64-unknown-linux-gnu" => {
                // Linux 64-bit
                "--target=x86_64-linux-gnu"
            }
            "aarch64-unknown-linux-gnu" => {
                // Linux 64-bit Arm
                "--target=aarch64-linux-gnu"
            }
            "aarch64-apple-darwin" => {
                // MacOS 64-bit Arm
                "--target=aarch64-macos-none"
            }
            "x86_64-apple-darwin" => {
                // MacOS 64-bit Intel/Legacy
                "--target=x86_64-macos-none"
            }
            _ => {
                panic!("Not a valid cross compile/linker target: {}", target_str);
            }
        };

        Some(CrossCompileInfo {
            zcc,
            zar,
            zig_target,
        })
    }
}

fn main() {
    //std::env::set_var("RUST_BACKTRACE", "1");

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

    #[allow(unused_variables)]
    let cross_info_opt = CrossCompileInfo::new(host_string.as_str(), target_string.as_str());

    // Build Linkable Opus "Static" Library if the opus feature is selected
    // Cross compiling will happen if necessary
    #[cfg(feature = "opus")]
    build_opus(&cross_info_opt);

    // Distributed dynamic library search paths could be added here based on the target and program requirements
    //println!("cargo:rustc-link-search=bin/lib/");

    //panic!("Panic and Print Build Debug");
}

// Create a Compiler Builder that will be setup for cross compiling if necessary (by using Zig)
// Expects Zig binaries to be "installed" on the host system if cross compiling is necessary
fn create_builder(cross_info_opt: &Option<CrossCompileInfo>) -> cc::Build {
    let mut builder = cc::Build::new();

    if let Some(cross_info) = cross_info_opt {
        // Builder environment variable set
        std::env::set_var("CC", cross_info.zcc);
        std::env::set_var("AR", cross_info.zar);
        std::env::set_var("CFLAGS", cross_info.zig_target);

        //std::env::set_var("CRATE_CC_NO_DEFAULTS", "1");
        builder.no_default_flags(true);
    }

    builder
}

// Build Opus for a Cross Compile Target
fn build_opus(cross_info_opt: &Option<CrossCompileInfo>) {
    let opus_include_path = Path::new("./src/opus/include");
    let opus_celt_path = Path::new("./src/opus/celt");
    let opus_silk_path = Path::new("./src/opus/silk");

    let mut include_paths = Vec::new();

    let mut celt_files = Vec::new();
    for element in opus_celt_path.read_dir().unwrap() {
        let path = element.unwrap().path();
        if path.is_dir() {
            include_paths.push(path);
        } else if let Some(extension) = path.extension() {
            if extension == "c" {
                celt_files.push(path);
            }
        }
    }

    let mut silk_files = Vec::new();
    for element in opus_silk_path.read_dir().unwrap() {
        let path = element.unwrap().path();
        if path.is_dir() {
            include_paths.push(path);
        } else if let Some(extension) = path.extension() {
            if extension == "c" {
                silk_files.push(path);
            }
        }
    }

    let mut builder = create_builder(cross_info_opt);
    builder.flag("-O3");
    builder.static_flag(true);
    builder.include(opus_include_path);
    builder.include(opus_celt_path);
    builder.include(opus_silk_path);
    builder.includes(include_paths);
    builder.define("OPUS_BUILD", None);
    builder.define("USE_ALLOCA", None);
    builder.file("./src/opus/src/opus.c");
    builder.file("./src/opus/src/opus_decoder.c");
    builder.file("./src/opus/src/analysis.c");
    builder.files(celt_files);
    builder.files(silk_files);

    let out_dir = std::env::var("OUT_DIR").unwrap();

    if cross_info_opt.is_none() {
        builder.compile("opus");
        //println!("cargo:rustc-link-search=native={}", out_dir.as_str());
        return;
    }

    builder.cargo_metadata(false);

    let compiled_files = builder.compile_intermediates();

    let mut archive_path = PathBuf::from(out_dir.clone());
    archive_path.push("libopus.a");
    //panic!("Archive Path: {:?}", archive_path);
    if let Err(e) = std::fs::remove_file(archive_path.as_path()) {
        match e.kind() {
            ErrorKind::NotFound => {
                // No issue if it wasn't found
            }
            _ => {
                panic!("Problem Deleting Archive: {:?}", archive_path);
            }
        }
    }

    let mut file_count = 0;
    let file_len = compiled_files.len();

    while file_count < file_len {
        let mut archiver = builder.get_archiver();
        archiver.arg("cq");
        archiver.arg(archive_path.as_path());

        let mut command_length = archive_path.as_os_str().len() + 4;
        while (command_length < 4096) && (file_count < file_len) {
            archiver.arg(compiled_files[file_count].as_path());
            command_length += compiled_files[file_count].as_os_str().len();
            file_count += 1;
        }

        let output = archiver.output().unwrap();
        if !output.status.success() {
            panic!(
                "Issue with archiver command results: {} {:?} {}",
                file_count,
                output.status,
                String::from_utf8(output.stdout).unwrap()
            );
        }
    }

    let mut archiver = builder.get_archiver();
    archiver.arg("s");
    archiver.arg(archive_path.as_path());

    println!("cargo:rustc-link-search=native={}", out_dir.as_str());
}
