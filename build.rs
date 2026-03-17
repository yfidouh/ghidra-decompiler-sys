use std::path::PathBuf;
use std::process::Command;

const VERSION: &str = "0.6.4";

/// Files to exclude from compilation.
const EXCLUDE_FILES: &[&str] = &[
    // BFD library dependencies (not available / not needed)
    "bfd_arch.cc",
    "loadimage_bfd.cc",
    "codedata.cc",
    "analyzesigs.cc",
    // Ghidra client-server integration (needs Java Ghidra process)
    "comment_ghidra.cc",
    "cpool_ghidra.cc",
    "database_ghidra.cc",
    "ghidra_arch.cc",
    "ghidra_context.cc",
    "ghidra_process.cc",
    "ghidra_translate.cc",
    "inject_ghidra.cc",
    "loadimage_ghidra.cc",
    "signature_ghidra.cc",
    "string_ghidra.cc",
    "typegrp_ghidra.cc",
    // Console/terminal interface (has main() or terminal deps)
    "consolemain.cc",
    "ifaceterm.cc",
    // Sleigh compiler (has main(), not needed at runtime)
    "slgh_compile.cc",
    // Test and example harnesses
    "test.cc",
    "sleighexample.cc",
];

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let extract_dir = out_dir.join(format!("ghidra-native-{}", VERSION));
    let decomp_dir = extract_dir.join("src").join("decompiler");

    // Download and extract if not already present
    if !decomp_dir.exists() {
        download_and_extract(&out_dir);
    }

    // Collect .cc files, excluding problematic ones
    let cc_files: Vec<PathBuf> = std::fs::read_dir(&decomp_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", decomp_dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|ext| ext == "cc")
                && !EXCLUDE_FILES
                    .iter()
                    .any(|exc| p.file_name().is_some_and(|f| f == *exc))
        })
        .collect();

    assert!(
        !cc_files.is_empty(),
        "No .cc files found in {:?}",
        decomp_dir
    );

    eprintln!(
        "cargo:warning=ghidra-decompiler-sys: compiling {} C++ files",
        cc_files.len()
    );

    let mut build = cc::Build::new();

    for cc in &cc_files {
        build.file(cc);
    }

    build.include(&decomp_dir);
    build.cpp(true);
    build.flag_if_supported("-std=c++17");
    build.flag_if_supported("-w"); // suppress warnings from vendored code

    if std::env::var("PROFILE").unwrap_or_default() == "release" {
        build.opt_level(2);
    }

    // Suppress the default `rustc-link-lib=static=ghidra_decomp` emitted by cc.
    // We need whole-archive linking so the linker keeps static global
    // constructors that register ArchitectureCapability subclasses (e.g.
    // RawBinaryArchitectureCapability). Without this, those objects are
    // stripped as "unreferenced" and no capabilities are found at runtime.
    build.cargo_metadata(false);
    build.compile("ghidra_decomp");

    // Export the include path so downstream crates can find the headers
    println!("cargo:include={}", decomp_dir.display());

    // Emit link directives manually with whole-archive semantics
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static:+whole-archive=ghidra_decomp");

    // Link the C++ standard library (cargo_metadata(false) suppresses the
    // automatic c++ linkage that cc would normally emit)
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // Link zlib (used by compression.cc for .sla file decompression)
    println!("cargo:rustc-link-lib=z");

    // Only rerun if the build script itself changes
    println!("cargo:rerun-if-changed=build.rs");
}

fn download_and_extract(out_dir: &PathBuf) {
    let url = format!(
        "https://github.com/radareorg/ghidra-native/archive/refs/tags/{}.tar.gz",
        VERSION
    );
    let tarball = out_dir.join(format!("ghidra-native-{}.tar.gz", VERSION));

    eprintln!("cargo:warning=Downloading ghidra-native {} ...", VERSION);

    let status = Command::new("curl")
        .args(["-L", "--fail", "--retry", "3", "-o"])
        .arg(&tarball)
        .arg(&url)
        .status()
        .expect("Failed to run curl. Please install curl to build this crate.");

    assert!(
        status.success(),
        "Failed to download ghidra-native from {}",
        url
    );

    eprintln!("cargo:warning=Extracting ghidra-native {} ...", VERSION);

    let status = Command::new("tar")
        .args(["xzf"])
        .arg(&tarball)
        .arg("-C")
        .arg(out_dir)
        .status()
        .expect("Failed to run tar");

    assert!(status.success(), "Failed to extract ghidra-native tarball");

    // Clean up tarball to save disk space
    let _ = std::fs::remove_file(&tarball);
}
