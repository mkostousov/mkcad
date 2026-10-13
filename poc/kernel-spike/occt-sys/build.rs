use std::path::PathBuf;

// Toolkits needed by the shim (OCCT 7.8+/8.0 names). Missing ones are reported, not fatal, so a layout
// change shows up as a link error naming the symbol.
const TOOLKITS: &[&str] = &[
    "TKernel", "TKMath", "TKG2d", "TKG3d", "TKGeomBase", "TKBRep", "TKGeomAlgo", "TKTopAlgo", "TKPrim", "TKBO",
    "TKBool", "TKFillet", "TKOffset", "TKShHealing", "TKMesh", "TKHelix", "TKXSBase", "TKDE", "TKDESTEP",
];

fn main() {
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cpp/shim.cc");
    println!("cargo:rerun-if-changed=include/shim.h");

    let root =
        PathBuf::from(std::env::var("OCCT_ROOT").unwrap_or_else(|_| "C:/dev/vcpkg/installed/x64-windows".into()));
    let include = root.join("include").join("opencascade");
    let lib = root.join("lib");
    assert!(
        include.join("Standard.hxx").exists(),
        "OCCT headers not found under {} (set OCCT_ROOT)",
        include.display()
    );

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut b = cxx_build::bridge("src/lib.rs");
    b.file("cpp/shim.cc").include(&include).std("c++17");
    // The shim includes "occt-sys/include/shim.h" and "occt-sys/src/lib.rs.h": make the parent of this crate
    // (the workspace dir, where the crate directory is named occt-sys) an include root.
    b.include(manifest.parent().unwrap());
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        b.flag("/EHsc").flag("/utf-8").define("_USE_MATH_DEFINES", None);
    }
    b.compile("occt_shim");

    println!("cargo:rustc-link-search=native={}", lib.display());
    for tk in TOOLKITS {
        let present = lib.join(format!("{tk}.lib")).exists() || lib.join(format!("lib{tk}.so")).exists();
        if present {
            println!("cargo:rustc-link-lib={tk}");
        } else {
            println!("cargo:warning=OCCT toolkit {tk} not found in {}", lib.display());
        }
    }
}
