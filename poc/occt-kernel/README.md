# OCCT Kernel Shim

Thin Rust wrapper for OpenCASCADE (OCCT 8.0.1) via C ABI shim. Part of PoC K-2.

Geometry operations execute in OCCT (C++); Rust holds opaque handles and provides a safe interface.

## Build & Test

Set environment variables before running cargo:
```powershell
$env:CARGO_TARGET_DIR = 'C:\dev\cargo-target'
$env:PATH = 'C:\dev\vcpkg\installed\x64-windows\bin;' + $env:PATH
$env:OCCT_DIR = 'C:\dev\vcpkg\installed\x64-windows'  # optional, default shown
```

Build and test:
```
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## OCCT Libraries Linked

TKernel, TKMath, TKG2d, TKG3d, TKGeomBase, TKGeomAlgo, TKBRep, TKTopAlgo, TKPrim, TKBO, TKFillet, TKShHealing, TKMesh, TKXSBase
