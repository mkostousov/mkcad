//! Thin shim crate for PoC K-2: Rust bindings to OpenCASCADE (OCCT 8.0.1).
//!
//! Geometry stays in OCCT (C++), Rust holds opaque handles and provides a safe interface.
//!
//! ## Sub-shape Indexing
//! Faces, edges, and vertices are indexed 0..n-1 in the order of `TopExp::MapShapes`.
//! This corresponds to OCCT's 1-based indexing minus 1.

use std::ffi::CString;
use std::ptr::NonNull;

pub mod naming;

mod ffi {
    #[repr(C)]
    pub struct mk_shape {
        _opaque: [u8; 0],
    }

    #[repr(C)]
    pub struct mk_op {
        _opaque: [u8; 0],
    }

    unsafe extern "C" {
        // Original functions
        pub fn mk_box(dx: f64, dy: f64, dz: f64) -> *mut mk_shape;
        pub fn mk_cylinder(r: f64, h: f64) -> *mut mk_shape;
        pub fn mk_cut(a: *const mk_shape, b: *const mk_shape) -> *mut mk_shape;
        pub fn mk_fuse(a: *const mk_shape, b: *const mk_shape) -> *mut mk_shape;
        pub fn mk_fillet_all(s: *const mk_shape, r: f64) -> *mut mk_shape;
        pub fn mk_volume(s: *const mk_shape) -> f64;
        pub fn mk_count(s: *const mk_shape, kind: i32) -> i32;
        pub fn mk_is_valid(s: *const mk_shape) -> i32;
        pub fn mk_free(s: *mut mk_shape);

        // New functions
        pub fn mk_translate(s: *const mk_shape, dx: f64, dy: f64, dz: f64) -> *mut mk_shape;
        pub fn mk_cylinder_at(x: f64, y: f64, z: f64, r: f64, h: f64) -> *mut mk_shape;
        pub fn mk_prism_polygon(xy: *const f64, n: i32, h: f64) -> *mut mk_shape;

        // Operation functions
        pub fn mk_op_cut(a: *const mk_shape, b: *const mk_shape) -> *mut mk_op;
        pub fn mk_op_fuse(a: *const mk_shape, b: *const mk_shape) -> *mut mk_op;
        pub fn mk_op_fillet(s: *const mk_shape, edge_idx: *const i32, n: i32, r: f64)
        -> *mut mk_op;
        pub fn mk_op_prism_polygon(xy: *const f64, n: i32, h: f64) -> *mut mk_op;
        pub fn mk_op_shape(op: *const mk_op) -> *mut mk_shape;
        pub fn mk_op_free(op: *mut mk_op);
        pub fn mk_op_modified(
            op: *const mk_op,
            operand: i32,
            kind: i32,
            index: i32,
            out: *mut i32,
            cap: i32,
        ) -> i32;
        pub fn mk_op_generated(
            op: *const mk_op,
            operand: i32,
            kind: i32,
            index: i32,
            out_kind: *mut i32,
            out: *mut i32,
            cap: i32,
        ) -> i32;
        pub fn mk_op_is_deleted(op: *const mk_op, operand: i32, kind: i32, index: i32) -> i32;

        // Face info function
        pub fn mk_face_info(s: *const mk_shape, index: i32, out7: *mut f64) -> i32;

        // STEP import/export functions
        pub fn mk_export_step(s: *const mk_shape, path: *const u8) -> i32;
        pub fn mk_export_step_schema(s: *const mk_shape, path: *const u8, schema: *const u8)
        -> i32;
        pub fn mk_import_step(path: *const u8) -> *mut mk_shape;
    }
}

/// Error type for kernel operations.
#[derive(Debug, Clone, Copy)]
pub enum KernelError {
    OpFailed(&'static str),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::OpFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for KernelError {}

/// Sub-shape kind enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Face = 0,
    Edge = 1,
    Vertex = 2,
}

impl Kind {
    fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Information about a face.
#[derive(Debug, Clone, Copy)]
pub struct FaceInfo {
    pub area: f64,
    pub centroid: [f64; 3],
    pub normal: [f64; 3],
}

/// Safe wrapper for OCCT shapes.
pub struct Shape(NonNull<ffi::mk_shape>);

impl Drop for Shape {
    fn drop(&mut self) {
        unsafe {
            ffi::mk_free(self.0.as_mut());
        }
    }
}

impl Shape {
    /// Create a rectangular box with given dimensions.
    pub fn make_box(dx: f64, dy: f64, dz: f64) -> Result<Self, KernelError> {
        unsafe {
            let ptr = ffi::mk_box(dx, dy, dz);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_box"))
        }
    }

    /// Create a cylinder with given radius and height.
    pub fn make_cylinder(r: f64, h: f64) -> Result<Self, KernelError> {
        unsafe {
            let ptr = ffi::mk_cylinder(r, h);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_cylinder"))
        }
    }

    /// Boolean cut operation.
    pub fn cut(&self, other: &Shape) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_cut(self.0.as_ptr(), other.0.as_ptr());
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_cut"))
        }
    }

    /// Boolean fuse operation.
    pub fn fuse(&self, other: &Shape) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_fuse(self.0.as_ptr(), other.0.as_ptr());
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_fuse"))
        }
    }

    /// Apply fillet to all edges.
    pub fn fillet_all(&self, radius: f64) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_fillet_all(self.0.as_ptr(), radius);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_fillet_all"))
        }
    }

    /// Get volume of the shape.
    pub fn volume(&self) -> Result<f64, KernelError> {
        unsafe {
            let vol = ffi::mk_volume(self.0.as_ptr());
            if vol.is_nan() {
                Err(KernelError::OpFailed("mk_volume"))
            } else {
                Ok(vol)
            }
        }
    }

    /// Count faces.
    pub fn face_count(&self) -> Result<usize, KernelError> {
        unsafe {
            let count = ffi::mk_count(self.0.as_ptr(), 0);
            if count < 0 {
                Err(KernelError::OpFailed("mk_count faces"))
            } else {
                Ok(count as usize)
            }
        }
    }

    /// Count edges.
    pub fn edge_count(&self) -> Result<usize, KernelError> {
        unsafe {
            let count = ffi::mk_count(self.0.as_ptr(), 1);
            if count < 0 {
                Err(KernelError::OpFailed("mk_count edges"))
            } else {
                Ok(count as usize)
            }
        }
    }

    /// Count vertices.
    pub fn vertex_count(&self) -> Result<usize, KernelError> {
        unsafe {
            let count = ffi::mk_count(self.0.as_ptr(), 2);
            if count < 0 {
                Err(KernelError::OpFailed("mk_count vertices"))
            } else {
                Ok(count as usize)
            }
        }
    }

    /// Check if shape is valid.
    pub fn is_valid(&self) -> Result<bool, KernelError> {
        unsafe {
            let result = ffi::mk_is_valid(self.0.as_ptr());
            match result {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(KernelError::OpFailed("mk_is_valid")),
            }
        }
    }

    /// Translate the shape by the given displacement.
    pub fn translate(&self, dx: f64, dy: f64, dz: f64) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_translate(self.0.as_ptr(), dx, dy, dz);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_translate"))
        }
    }

    /// Create a cylinder at the given position (axis along +Z).
    pub fn cylinder_at(x: f64, y: f64, z: f64, r: f64, h: f64) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_cylinder_at(x, y, z, r, h);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_cylinder_at"))
        }
    }

    /// Create a prism from a polygon.
    /// The polygon is defined by n points in the z=0 plane, and is extruded by h along +Z.
    pub fn prism_polygon(points: &[(f64, f64)], h: f64) -> Result<Shape, KernelError> {
        if points.is_empty() {
            return Err(KernelError::OpFailed("prism_polygon: empty points"));
        }

        let mut xy = Vec::with_capacity(points.len() * 2);
        for (x, y) in points {
            xy.push(*x);
            xy.push(*y);
        }

        unsafe {
            let ptr = ffi::mk_prism_polygon(xy.as_ptr(), points.len() as i32, h);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_prism_polygon"))
        }
    }

    /// Count sub-shapes of the given kind.
    pub fn count(&self, kind: Kind) -> Result<usize, KernelError> {
        unsafe {
            let count = ffi::mk_count(self.0.as_ptr(), kind.as_i32());
            if count < 0 {
                Err(KernelError::OpFailed("mk_count"))
            } else {
                Ok(count as usize)
            }
        }
    }

    /// Get face information.
    pub fn face_info(&self, index: usize) -> Result<FaceInfo, KernelError> {
        unsafe {
            let mut out = [0.0f64; 7];
            let result = ffi::mk_face_info(self.0.as_ptr(), index as i32, out.as_mut_ptr());
            if result == 0 {
                Ok(FaceInfo {
                    area: out[0],
                    centroid: [out[1], out[2], out[3]],
                    normal: [out[4], out[5], out[6]],
                })
            } else {
                Err(KernelError::OpFailed("mk_face_info"))
            }
        }
    }

    /// Export shape to STEP file.
    pub fn export_step(&self, path: &str) -> Result<(), KernelError> {
        let path_cstr = CString::new(path)
            .map_err(|_| KernelError::OpFailed("invalid path: contains null byte"))?;
        unsafe {
            let result = ffi::mk_export_step(self.0.as_ptr(), path_cstr.as_ptr() as *const u8);
            if result == 0 {
                Ok(())
            } else {
                Err(KernelError::OpFailed("mk_export_step"))
            }
        }
    }

    /// Export shape to STEP file with explicit schema.
    /// Schema can be "AP203", "AP214IS", "AP242DIS", etc.
    pub fn export_step_schema(&self, path: &str, schema: &str) -> Result<(), KernelError> {
        let path_cstr = CString::new(path)
            .map_err(|_| KernelError::OpFailed("invalid path: contains null byte"))?;
        let schema_cstr = CString::new(schema)
            .map_err(|_| KernelError::OpFailed("invalid schema: contains null byte"))?;
        unsafe {
            let result = ffi::mk_export_step_schema(
                self.0.as_ptr(),
                path_cstr.as_ptr() as *const u8,
                schema_cstr.as_ptr() as *const u8,
            );
            if result == 0 {
                Ok(())
            } else {
                Err(KernelError::OpFailed("mk_export_step_schema"))
            }
        }
    }

    /// Import shape from STEP file.
    pub fn import_step(path: &str) -> Result<Shape, KernelError> {
        let path_cstr = CString::new(path)
            .map_err(|_| KernelError::OpFailed("invalid path: contains null byte"))?;
        unsafe {
            let ptr = ffi::mk_import_step(path_cstr.as_ptr() as *const u8);
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_import_step"))
        }
    }
}

// Safety: Shape holds a NonNull which is Send + Sync
unsafe impl Send for Shape {}
unsafe impl Sync for Shape {}

/// Operation result with history tracking.
pub struct Op(NonNull<ffi::mk_op>);

impl Drop for Op {
    fn drop(&mut self) {
        unsafe {
            ffi::mk_op_free(self.0.as_mut());
        }
    }
}

impl Op {
    /// Get the result shape.
    pub fn shape(&self) -> Result<Shape, KernelError> {
        unsafe {
            let ptr = ffi::mk_op_shape(self.0.as_ptr());
            NonNull::new(ptr)
                .map(Shape)
                .ok_or(KernelError::OpFailed("mk_op_shape"))
        }
    }

    /// Get indices of result shapes that the input sub-shape was modified into.
    /// Returns the indices in the result's indexing of the same kind.
    pub fn modified(
        &self,
        operand: usize,
        kind: Kind,
        index: usize,
    ) -> Result<Vec<usize>, KernelError> {
        unsafe {
            let mut out = vec![0i32; 256];
            let count = ffi::mk_op_modified(
                self.0.as_ptr(),
                operand as i32,
                kind.as_i32(),
                index as i32,
                out.as_mut_ptr(),
                out.len() as i32,
            );
            if count < 0 {
                Err(KernelError::OpFailed("mk_op_modified"))
            } else {
                Ok(out[..count as usize].iter().map(|&x| x as usize).collect())
            }
        }
    }

    /// Get indices of result shapes that the input sub-shape was generated into.
    /// Returns the indices and kinds of the generated shapes.
    pub fn generated(
        &self,
        operand: usize,
        kind: Kind,
        index: usize,
    ) -> Result<Vec<(Kind, usize)>, KernelError> {
        unsafe {
            let mut out = vec![0i32; 256];
            let mut out_kind = vec![0i32; 256];
            let count = ffi::mk_op_generated(
                self.0.as_ptr(),
                operand as i32,
                kind.as_i32(),
                index as i32,
                out_kind.as_mut_ptr(),
                out.as_mut_ptr(),
                out.len() as i32,
            );
            if count < 0 {
                Err(KernelError::OpFailed("mk_op_generated"))
            } else {
                Ok(out[..count as usize]
                    .iter()
                    .zip(out_kind[..count as usize].iter())
                    .map(|(&idx, &k)| {
                        let kind = match k {
                            0 => Kind::Face,
                            1 => Kind::Edge,
                            2 => Kind::Vertex,
                            _ => Kind::Face, // fallback
                        };
                        (kind, idx as usize)
                    })
                    .collect())
            }
        }
    }

    /// Check if the input sub-shape was deleted.
    pub fn is_deleted(
        &self,
        operand: usize,
        kind: Kind,
        index: usize,
    ) -> Result<bool, KernelError> {
        unsafe {
            let result =
                ffi::mk_op_is_deleted(self.0.as_ptr(), operand as i32, kind.as_i32(), index as i32);
            match result {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(KernelError::OpFailed("mk_op_is_deleted")),
            }
        }
    }

    /// Create a cut operation result.
    pub fn cut(a: &Shape, b: &Shape) -> Result<Op, KernelError> {
        unsafe {
            let ptr = ffi::mk_op_cut(a.0.as_ptr(), b.0.as_ptr());
            NonNull::new(ptr)
                .map(Op)
                .ok_or(KernelError::OpFailed("mk_op_cut"))
        }
    }

    /// Create a fuse operation result.
    pub fn fuse(a: &Shape, b: &Shape) -> Result<Op, KernelError> {
        unsafe {
            let ptr = ffi::mk_op_fuse(a.0.as_ptr(), b.0.as_ptr());
            NonNull::new(ptr)
                .map(Op)
                .ok_or(KernelError::OpFailed("mk_op_fuse"))
        }
    }

    /// Create a fillet operation result.
    pub fn fillet(s: &Shape, edge_indices: &[usize], radius: f64) -> Result<Op, KernelError> {
        let edge_idx: Vec<i32> = edge_indices.iter().map(|&i| i as i32).collect();
        unsafe {
            let ptr = ffi::mk_op_fillet(
                s.0.as_ptr(),
                edge_idx.as_ptr(),
                edge_idx.len() as i32,
                radius,
            );
            NonNull::new(ptr)
                .map(Op)
                .ok_or(KernelError::OpFailed("mk_op_fillet"))
        }
    }

    /// Create a prism operation result.
    pub fn prism_polygon(points: &[(f64, f64)], height: f64) -> Result<Op, KernelError> {
        if points.is_empty() {
            return Err(KernelError::OpFailed("prism_polygon: empty points"));
        }

        let mut xy = Vec::with_capacity(points.len() * 2);
        for (x, y) in points {
            xy.push(*x);
            xy.push(*y);
        }

        unsafe {
            let ptr = ffi::mk_op_prism_polygon(xy.as_ptr(), points.len() as i32, height);
            NonNull::new(ptr)
                .map(Op)
                .ok_or(KernelError::OpFailed("mk_op_prism_polygon"))
        }
    }
}

// Safety: Op holds a NonNull which is Send + Sync
unsafe impl Send for Op {}
unsafe impl Sync for Op {}
