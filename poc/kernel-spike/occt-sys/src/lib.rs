//! Raw cxx bridge to the OpenCASCADE shim. Use the `kernel` crate for the safe API.
//!
//! Sub-shape indices are 1-based positions in `TopExp::MapShapes` order (edges and faces are numbered
//! independently); they are stable for one `Shape` value only.

#[cxx::bridge(namespace = "mkcad")]
pub mod ffi {
    /// One input→output relation recorded from `BRepBuilderAPI_MakeShape` history.
    struct HistEntry {
        /// 0 = modified, 1 = generated, 2 = deleted, 3 = unchanged (same sub-shape in the result),
        /// 4 = end cap of a prism (`LastShape`, src = the profile face).
        kind: u8,
        /// Which operand: 0 = first/only argument, 1 = tool.
        arg: u8,
        /// 1 = edge, 2 = face.
        dim: u8,
        /// 1-based index in the operand.
        src: u32,
        /// 1 = edge, 2 = face (0 for deleted); an edge can generate a face.
        dst_dim: u8,
        /// 1-based index in the result (0 for deleted).
        dst: u32,
    }

    struct OpResult {
        shape: UniquePtr<Shape>,
        history: Vec<HistEntry>,
    }

    struct FaceProps {
        area: f64,
        cx: f64,
        cy: f64,
        cz: f64,
        /// Outward unit normal of a planar face; all zero for other surfaces.
        nx: f64,
        ny: f64,
        nz: f64,
    }

    struct EdgeProps {
        length: f64,
        cx: f64,
        cy: f64,
        cz: f64,
    }

    unsafe extern "C++" {
        include!("occt-sys/include/shim.h");

        type Shape;

        fn make_box(dx: f64, dy: f64, dz: f64) -> Result<UniquePtr<Shape>>;
        fn extrude_polygon(xy: &[f64], height: f64) -> Result<OpResult>;
        fn translate(a: &Shape, x: f64, y: f64, z: f64) -> Result<UniquePtr<Shape>>;
        fn fillet_edges(a: &Shape, edges: &[u32], radius: f64) -> Result<OpResult>;
        fn boolean_cut(a: &Shape, tool: &Shape) -> Result<OpResult>;
        fn boolean_fuse(a: &Shape, tool: &Shape) -> Result<OpResult>;

        fn volume(a: &Shape) -> Result<f64>;
        fn is_valid(a: &Shape) -> bool;
        fn count_faces(a: &Shape) -> u32;
        fn count_edges(a: &Shape) -> u32;
        fn face_props(a: &Shape, index: u32) -> Result<FaceProps>;
        fn edge_props(a: &Shape, index: u32) -> Result<EdgeProps>;
        fn edge_faces(a: &Shape, index: u32) -> Result<Vec<u32>>;
        fn shell_faces(a: &Shape, faces: &[u32], thickness: f64) -> Result<OpResult>;

        fn export_step(a: &Shape, path: &str) -> Result<()>;
        fn import_step(path: &str) -> Result<UniquePtr<Shape>>;
    }
}
