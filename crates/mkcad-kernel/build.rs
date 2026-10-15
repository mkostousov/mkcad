fn main() {
    let occt_dir = std::env::var("OCCT_DIR")
        .unwrap_or_else(|_| r"C:\dev\vcpkg\installed\x64-windows".to_string());

    let include_dir = format!("{}\\include\\opencascade", occt_dir);
    let lib_dir = format!("{}\\lib", occt_dir);

    cc::Build::new()
        .cpp(true)
        .file("cpp/shim.cpp")
        .include(&include_dir)
        .flag("/std:c++17")
        .flag("/EHsc")
        .compile("occt_shim");

    println!("cargo:rustc-link-search=native={}", lib_dir);

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
