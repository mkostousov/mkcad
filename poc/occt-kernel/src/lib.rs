//! Thin shim crate for PoC K-2: Rust bindings to OpenCASCADE (OCCT 8.0.1).
//!
//! Geometry stays in OCCT (C++), Rust holds opaque handles and provides a safe interface.

use std::ptr::NonNull;

mod ffi {
    #[repr(C)]
    pub struct mk_shape {
        _opaque: [u8; 0],
    }

    unsafe extern "C" {
        pub fn mk_box(dx: f64, dy: f64, dz: f64) -> *mut mk_shape;
        pub fn mk_cylinder(r: f64, h: f64) -> *mut mk_shape;
        pub fn mk_cut(a: *const mk_shape, b: *const mk_shape) -> *mut mk_shape;
        pub fn mk_fuse(a: *const mk_shape, b: *const mk_shape) -> *mut mk_shape;
        pub fn mk_fillet_all(s: *const mk_shape, r: f64) -> *mut mk_shape;
        pub fn mk_volume(s: *const mk_shape) -> f64;
        pub fn mk_count(s: *const mk_shape, kind: i32) -> i32;
        pub fn mk_is_valid(s: *const mk_shape) -> i32;
        pub fn mk_free(s: *mut mk_shape);
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
}

// Safety: Shape holds a NonNull which is Send + Sync
unsafe impl Send for Shape {}
unsafe impl Sync for Shape {}
