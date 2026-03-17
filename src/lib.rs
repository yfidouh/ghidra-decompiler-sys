// This crate builds Ghidra's C++ decompiler (libdecomp) as a native static library.
//
// No Rust API is exposed. Downstream crates link against the C++ library
// and access the include path via the `DEP_GHIDRA_DECOMP_INCLUDE` environment
// variable in their build scripts.
//
// Based on ghidra-native: https://github.com/radareorg/ghidra-native
