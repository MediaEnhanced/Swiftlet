// Media Enhanced Swiftlet Audio Build Script with Cross Compile Support

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

fn main() {
    let host_string = match std::env::var("HOST") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not find host triple: {}", e);
        }
    };
    let target_string = match std::env::var("TARGET") {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not find target triple: {}", e);
        }
    };
    if target_string == host_string {
        #[cfg(feature = "opus")]
        build_opus_native();
    } else {
        let mut builder = cross_compile_setup(host_string.as_str(), target_string.as_str());
        #[cfg(feature = "opus")]
        build_opus_cross(&mut builder);
    }
    // Distributed dynamic library search paths can be added here based on the target
}

// Build Opus Natively (NOT cross compile)
fn build_opus_native() {
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

    cc::Build::new()
        .flag("-O3")
        .include(opus_include_path)
        .include(opus_celt_path)
        .include(opus_silk_path)
        .includes(include_paths)
        .define("OPUS_BUILD", None)
        .define("USE_ALLOCA", None)
        .file("./src/opus/src/opus.c")
        .file("./src/opus/src/opus_decoder.c")
        .file("./src/opus/src/analysis.c")
        .files(celt_files)
        .files(silk_files)
        .compile("libopus");
}

// Build Opus for a Cross Compile Target
fn build_opus_cross(builder: &mut cc::Build) {
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

    builder.flag("-O3");
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

    let compiled_files = builder.compile_intermediates();

    let out_dir = std::env::var("OUT_DIR").unwrap();
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

    println!("cargo:rustc-link-lib=static=opus");
    println!("cargo:rustc-link-search=native={}", out_dir.as_str());

    //panic!("Panic and Print Build Debug");

    // Nothing Yet
}

// Cross Compiling Setup
// Currently Uses Zig Binaries (and expects them to be installed on the host system)
fn cross_compile_setup(host_str: &str, target_str: &str) -> cc::Build {
    // Each host type should set the appropriate CC Environmental Variable
    match host_str {
        "x86_64-pc-windows-gnu" => {
            // Windows 64-bit
            std::env::set_var("CC", "../zig/zcc.bat");
            std::env::set_var("AR", "../zig/zar.bat");
        }
        "x86_64-pc-windows-msvc" => {
            // Windows 64-bit Alt
            std::env::set_var("CC", "../zig/zcc.bat");
            //std::env::set_var("AR", "../zig/zar.bat");
        }
        "aarch64-pc-windows-msvc" => {
            // Windows 64-bit Arm
            std::env::set_var("CC", "../zig/zcc.bat");
            //std::env::set_var("AR", "../zig/zar.bat");
        }
        "x86_64-unknown-linux-gnu" => {
            // Linux 64-bit
            std::env::set_var("CC", "../zig/zcc.sh");
            //std::env::set_var("AR", "../zig/zar.sh");
        }
        "aarch64-unknown-linux-gnu" => {
            // Linux 64-bit Arm
            std::env::set_var("CC", "../zig/zcc.sh");
            //std::env::set_var("AR", "../zig/zar.sh");
        }
        "aarch64-apple-darwin" => {
            // MacOS 64-bit Arm
            std::env::set_var("CC", "../zig/zcc.sh");
            //std::env::set_var("AR", "../zig/zar.sh");
        }
        "x86_64-apple-darwin" => {
            // MacOS 64-bit Intel/Legacy
            std::env::set_var("CC", "../zig/zcc.sh");
            //std::env::set_var("AR", "../zig/zar.sh");
        }
        _ => {
            panic!(
                "This host platform, {}, is currently unsupported for cross compiling!",
                host_str
            );
        }
    }

    println!("cargo:rustc-linker=ld");
    //println!("cargo:rustc-link-arg=-T../zig/zar.bat");

    //std::env::set_var("CRATE_CC_NO_DEFAULTS", "1");
    //std::env::set_var("CFLAGS", "-testflag");

    let mut builder = cc::Build::new();
    builder.no_default_flags(true);
    // If builder cannot find the Cross Compiler installed system-wide, then check in the zig path
    //let zig_path = Path::new("../bin/zig/zig.exe");

    // Each Target should set the appropriate builder target flags and OS library link search paths
    match target_str {
        "x86_64-pc-windows-gnu" => {
            // Windows 64-bit
            if host_str == "x86_64-pc-windows-msvc" {
                // Partial cross compiling (Maybe do something different)
            }
            builder.flag("-target x86_64-windows-gnu");
        }
        "x86_64-pc-windows-msvc" => {
            // Windows 64-bit Alt
            panic!(
                "{} is currently NOT supported as a cross-compile target. 
                Maybe try x86_64-pc-windows-gnu as the target instead",
                target_str
            );
        }
        "aarch64-pc-windows-msvc" => {
            // Windows 64-bit Arm
        }
        "x86_64-unknown-linux-gnu" => {
            // Linux 64-bit
            builder.flag("-target x86_64-linux-gnu");
        }
        "aarch64-unknown-linux-gnu" => {
            // Linux 64-bit Arm
            builder.flag("-target aarch64-linux-gnu");
        }
        "aarch64-apple-darwin" => {
            // MacOS 64-bit Arm
            std::env::set_var("CFLAGS", "-target aarch64-macos-none");
            //builder.flag("-target aarch64-macos-none");
            //builder.ar_flag("-target aarch64-macos-none");
        }
        "x86_64-apple-darwin" => {
            // MacOS 64-bit Intel/Legacy
            builder.flag("-target x86_64-macos-gnu");
        }
        _ => {
            panic!("Not a valid build target: {}", target_str);
        }
    }

    builder
}
