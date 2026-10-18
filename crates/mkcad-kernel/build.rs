use std::path::PathBuf;

fn main() {
    let default_dir = if cfg!(windows) {
        r"C:\dev\vcpkg\installed\x64-windows"
    } else {
        "/usr/local"
    };
    let occt_dir =
        PathBuf::from(std::env::var("OCCT_DIR").unwrap_or_else(|_| default_dir.to_string()));

    let include_dir = occt_dir.join("include").join("opencascade");
    let lib_dir = occt_dir.join("lib");

    let mut build = cc::Build::new();
    build.cpp(true).file("cpp/shim.cpp").include(&include_dir);
    if build.get_compiler().is_like_msvc() {
        build.flag("/std:c++17").flag("/EHsc");
    } else {
        build.flag("-std=c++17");
    }
    build.compile("occt_shim");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if !cfg!(windows) {
        // Find the OCCT shared libraries at test/run time without LD_LIBRARY_PATH.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    // Link OCCT libraries
    let tk_libs = [
        "TKernel",
        "TKMath",
        "TKG2d",
        "TKG3d",
        "TKGeomBase",
        "TKGeomAlgo",
        "TKBRep",
        "TKTopAlgo",
        "TKPrim",
        "TKBO",
        "TKFillet",
        "TKShHealing",
        "TKMesh",
        "TKXSBase",
        "TKDESTEP",
        "TKExpress",
    ];

    for lib in &tk_libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    println!("cargo:rerun-if-changed=cpp/shim.cpp");
    println!("cargo:rerun-if-env-changed=OCCT_DIR");
}
