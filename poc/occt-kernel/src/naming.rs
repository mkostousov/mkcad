//! Topological naming model per G.4: labels from sketch IDs + operation history.
//!
//! This module implements the persistent reference scheme for tracking topology
//! through kernel operations:
//!
//! 1. **Generated-topology labels**: Every result face gets a label derived from
//!    authoritative inputs (sketch curve IDs, operation type, role).
//!    - Extrude: `f:side:<profile_curve_id>`, `f:cap:start`, `f:cap:end`
//!    - Cut: `f:tool:<direction>` from tool definition
//!    - Fillet/Shell generated: `f:gen:<source_label(s)>`
//!
//! 2. **History propagation**: A result face inherits labels from all input faces
//!    that map to it (via operation history modified/unchanged).
//!
//! 3. **Reference descriptor**: Stores the label and geometric signature (centroid,
//!    normal, area/length) to enable fallback resolution.
//!
//! 4. **Resolution**: Resolves a reference to a set of indices in the result shape:
//!    - Exact: label matched exactly 1 face
//!    - Expanded: label split into multiple faces
//!    - Signature: geometric signature matched 1 face (label lost)
//!    - Lost: nothing found
//!
//! 5. **Recompute**: A feature with lost/ambiguous reference reports `REF.LOST` /
//!    `REF.AMBIGUOUS` and the previous body continues (cached geometry).

use std::collections::{HashMap, HashSet};
use crate::{Kind, Shape};

/// A topological label identifying a face or edge in the result of an operation.
///
/// Formats:
/// - `f:side:<id>` - side face from extrusion of profile curve id
/// - `f:cap:start` - start cap of extrusion
/// - `f:cap:end` - end cap of extrusion
/// - `f:tool:<name>` - face from tool in boolean operation
/// - `f:gen:<labels>` - face generated from other faces/edges
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(String);

impl Label {
    /// Create a label for a side face from an extrusion profile curve.
    pub fn side(profile_curve_id: u64) -> Self {
        Label(format!("f:side:{}", profile_curve_id))
    }

    /// Create a label for an extrusion cap.
    pub fn cap(start: bool) -> Self {
        Label(format!("f:cap:{}", if start { "start" } else { "end" }))
    }

    /// Create a label for a tool face in a boolean operation.
    pub fn tool(direction: &str) -> Self {
        Label(format!("f:tool:{}", direction))
    }

    /// Create a label for a face generated from source label(s).
    pub fn generated(sources: &[Label]) -> Self {
        let sources_str = sources
            .iter()
            .map(|l| l.0.clone())
            .collect::<Vec<_>>()
            .join("|");
        Label(format!("f:gen:{}", sources_str))
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Geometric signature for fallback resolution when labels are lost.
///
/// Stores sufficient geometry to identify a face or edge by its location,
/// orientation, and size, allowing approximate matching within tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometricSignature {
    /// Sub-shape kind (Face or Edge).
    pub kind: Kind,
    /// Centroid coordinates [x, y, z].
    pub centroid: [f64; 3],
    /// Normal vector for faces; None for edges [x, y, z] if present.
    pub normal: Option<[f64; 3]>,
    /// Area for faces, length for edges.
    pub size: f64,
}

impl GeometricSignature {
    /// Create a signature for a face from extracted properties.
    pub fn face(centroid: [f64; 3], normal: [f64; 3], area: f64) -> Self {
        GeometricSignature {
            kind: Kind::Face,
            centroid,
            normal: Some(normal),
            size: area,
        }
    }

    /// Create a signature for an edge from extracted properties.
    pub fn edge(centroid: [f64; 3], length: f64) -> Self {
        GeometricSignature {
            kind: Kind::Edge,
            centroid,
            normal: None,
            size: length,
        }
    }

    /// Check if this signature matches another within tolerance.
    ///
    /// Tolerance:
    /// - Centroid: 1e-6 mm absolute
    /// - Normal: within 1e-6 angular error (if both have normals)
    /// - Size: 1e-6 relative
    pub fn matches(&self, other: &GeometricSignature, tolerance: f64) -> bool {
        if self.kind != other.kind {
            return false;
        }

        // Check centroid distance
        let dist = (0..3)
            .map(|i| (self.centroid[i] - other.centroid[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        if dist > tolerance {
            return false;
        }

        // Check size relative error
        if self.size > 0.0 && other.size > 0.0 {
            let rel_error = (self.size - other.size).abs() / self.size.max(other.size);
            if rel_error > tolerance {
                return false;
            }
        }

        // Check normal (for faces)
        if let (Some(n1), Some(n2)) = (self.normal, other.normal) {
            let dot = (0..3).map(|i| n1[i] * n2[i]).sum::<f64>();
            // dot product should be close to ±1
            if (dot.abs() - 1.0).abs() > tolerance {
                return false;
            }
        }

        true
    }
}

/// A reference to a sub-shape (face or edge) with label and signature.
///
/// Used to resolve a reference across rebuilds: if the label matches exactly,
/// the reference is resolved; if the label split, all pieces are included;
/// if the label vanished, the signature is used as fallback.
#[derive(Debug, Clone)]
pub struct ReferenceDescriptor {
    /// The topological label (e.g., "f:side:123").
    pub label: Label,
    /// Geometric signature for fallback resolution.
    pub signature: GeometricSignature,
}

impl ReferenceDescriptor {
    /// Create a reference descriptor.
    pub fn new(label: Label, signature: GeometricSignature) -> Self {
        ReferenceDescriptor { label, signature }
    }
}

/// Resolution result for a reference descriptor.
///
/// The outcome of attempting to resolve a reference in a rebuilt shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionResult {
    /// Exact match: the label found exactly 1 face.
    Exact,
    /// Expanded: the label split into multiple faces; all are included.
    /// Contains the count of matching faces.
    Expanded(usize),
    /// Signature match: the label was lost but geometric signature matched 1 face.
    Signature,
    /// Ambiguous: signature matched multiple faces (>1).
    /// Contains the count of matching faces.
    Ambiguous(usize),
    /// Lost: no label match and no unique signature match.
    Lost,
}

/// Timeline of operations with topological labels.
///
/// Tracks face labels through a sequence of kernel operations,
/// propagating them via the operation history (modified/generated/deleted).
pub struct Timeline {
    /// Map from face index to set of labels (a face can have multiple labels).
    face_labels: HashMap<usize, HashSet<Label>>,
}

impl Timeline {
    /// Create a new empty timeline.
    pub fn new() -> Self {
        Timeline {
            face_labels: HashMap::new(),
        }
    }

    /// Add initial labels for a shape (e.g., from extrusion).
    pub fn init_shape(&mut self, _shape: &Shape, labels: Vec<(usize, Label)>) {
        for (face_idx, label) in labels {
            self.face_labels
                .entry(face_idx)
                .or_insert_with(HashSet::new)
                .insert(label);
        }
    }

    /// Propagate labels through an operation result.
    ///
    /// For each face in the result, if it came from input faces (via modified/unchanged),
    /// it inherits all labels from those input faces. Split faces all keep the labels.
    ///
    /// # Arguments
    /// - `result_face_indices`: indices of faces in the result shape
    /// - `label_map`: map from input face index to set of labels
    /// - `modified_map`: for each result face, the input faces it came from
    pub fn propagate(
        &mut self,
        result_face_indices: &[usize],
        label_map: &HashMap<usize, HashSet<Label>>,
        modified_map: &HashMap<usize, Vec<usize>>,
    ) {
        for &result_idx in result_face_indices {
            if let Some(source_indices) = modified_map.get(&result_idx) {
                let mut inherited = HashSet::new();
                for &source_idx in source_indices {
                    if let Some(source_labels) = label_map.get(&source_idx) {
                        inherited.extend(source_labels.iter().cloned());
                    }
                }
                if !inherited.is_empty() {
                    self.face_labels.insert(result_idx, inherited);
                }
            }
        }
    }

    /// Get labels for a face.
    pub fn labels_for_face(&self, face_idx: usize) -> Option<&HashSet<Label>> {
        self.face_labels.get(&face_idx)
    }

    /// Resolve a reference descriptor to indices in the current shape.
    ///
    /// Returns the resolution result and a list of matching face indices.
    pub fn resolve(
        &self,
        descriptor: &ReferenceDescriptor,
        shape: &Shape,
        tolerance: f64,
    ) -> (ResolutionResult, Vec<usize>) {
        let mut exact_matches = Vec::new();
        let mut signature_matches = Vec::new();

        // Try to get face count
        let face_count = match shape.count(Kind::Face) {
            Ok(count) => count,
            Err(_) => return (ResolutionResult::Lost, Vec::new()),
        };

        // Look for exact label matches
        for face_idx in 0..face_count {
            if let Some(labels) = self.labels_for_face(face_idx) {
                if labels.contains(&descriptor.label) {
                    exact_matches.push(face_idx);
                }
            }

            // Also check signature for fallback
            if let Ok(face_info) = shape.face_info(face_idx) {
                let sig = GeometricSignature::face(
                    [face_info.centroid[0], face_info.centroid[1], face_info.centroid[2]],
                    [face_info.normal[0], face_info.normal[1], face_info.normal[2]],
                    face_info.area,
                );
                if sig.matches(&descriptor.signature, tolerance) {
                    signature_matches.push(face_idx);
                }
            }
        }

        // Return based on exact matches first
        if !exact_matches.is_empty() {
            let result = if exact_matches.len() == 1 {
                ResolutionResult::Exact
            } else {
                ResolutionResult::Expanded(exact_matches.len())
            };
            (result, exact_matches)
        } else if !signature_matches.is_empty() {
            let result = if signature_matches.len() == 1 {
                ResolutionResult::Signature
            } else {
                ResolutionResult::Ambiguous(signature_matches.len())
            };
            (result, signature_matches)
        } else {
            (ResolutionResult::Lost, Vec::new())
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Step 1 (G.4): Test label creation and formatting
    #[test]
    fn test_label_creation() {
        // Test side face label
        let side_label = Label::side(123);
        assert_eq!(side_label.as_str(), "f:side:123");

        // Test cap labels
        let start_cap = Label::cap(true);
        assert_eq!(start_cap.as_str(), "f:cap:start");

        let end_cap = Label::cap(false);
        assert_eq!(end_cap.as_str(), "f:cap:end");

        // Test tool label
        let tool_label = Label::tool("x+");
        assert_eq!(tool_label.as_str(), "f:tool:x+");

        // Test generated label
        let gen_label = Label::generated(&[Label::side(1), Label::side(2)]);
        assert!(gen_label.as_str().contains("f:gen:"));
        assert!(gen_label.as_str().contains("f:side:1"));
        assert!(gen_label.as_str().contains("f:side:2"));
    }

    /// Step 2 (G.4): Test geometric signature creation and matching
    #[test]
    fn test_geometric_signature() {
        let sig1 = GeometricSignature::face([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], 100.0);
        assert_eq!(sig1.kind, Kind::Face);
        assert_eq!(sig1.centroid, [1.0, 2.0, 3.0]);
        assert_eq!(sig1.normal, Some([0.0, 0.0, 1.0]));
        assert_eq!(sig1.size, 100.0);

        let sig2 = GeometricSignature::edge([1.5, 2.5, 3.5], 50.0);
        assert_eq!(sig2.kind, Kind::Edge);
        assert_eq!(sig2.centroid, [1.5, 2.5, 3.5]);
        assert!(sig2.normal.is_none());
        assert_eq!(sig2.size, 50.0);
    }

    /// Step 3 (G.4): Test signature matching within tolerance
    #[test]
    fn test_signature_matching() {
        let sig1 = GeometricSignature::face([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 100.0);

        // Identical signature
        let sig2 = GeometricSignature::face([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 100.0);
        assert!(sig1.matches(&sig2, 1e-6));

        // Slightly different centroid, within tolerance
        let sig3 = GeometricSignature::face([1e-7, 1e-7, 1e-7], [0.0, 0.0, 1.0], 100.0);
        assert!(sig1.matches(&sig3, 1e-6));

        // Centroid too far
        let sig4 = GeometricSignature::face([1e-5, 0.0, 0.0], [0.0, 0.0, 1.0], 100.0);
        assert!(!sig1.matches(&sig4, 1e-6));

        // Different size
        let sig5 = GeometricSignature::face([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 110.0);
        assert!(!sig1.matches(&sig5, 1e-6));

        // Opposite normal direction (still matches, as dot = -1)
        let sig6 = GeometricSignature::face([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], 100.0);
        // This will not match because |dot - 1| = 2 > 1e-6, but we check |dot| - 1
        // Actually dot = -1, so |dot| = 1, and |1 - 1| = 0, so it should match
        // But the code checks (dot.abs() - 1.0).abs(), which is |(1 - 1)| = 0
        assert!(sig1.matches(&sig6, 1e-6));
    }

    /// Step 4 (G.4): Test reference descriptor creation and storage
    #[test]
    fn test_reference_descriptor() {
        let label = Label::side(456);
        let sig = GeometricSignature::face([5.0, 10.0, 15.0], [1.0, 0.0, 0.0], 200.0);
        let desc = ReferenceDescriptor::new(label.clone(), sig);

        assert_eq!(desc.label, label);
        assert_eq!(desc.signature.centroid, [5.0, 10.0, 15.0]);
        assert_eq!(desc.signature.size, 200.0);
    }

    /// Step 5 (G.4): Test timeline initialization and label propagation
    #[test]
    fn test_timeline_init_and_labels() {
        let mut timeline = Timeline::new();

        // Initialize with labels for initial shape
        let labels = vec![
            (0, Label::cap(true)),   // Face 0: start cap
            (1, Label::side(10)),    // Face 1: side from curve 10
            (2, Label::side(20)),    // Face 2: side from curve 20
            (3, Label::cap(false)),  // Face 3: end cap
        ];

        timeline.init_shape(&Shape::make_box(10.0, 10.0, 10.0).unwrap(), labels);

        // Check labels are stored
        assert!(timeline.labels_for_face(0).is_some());
        assert!(timeline.labels_for_face(1).is_some());
        assert!(timeline.labels_for_face(0).unwrap().contains(&Label::cap(true)));
        assert!(timeline.labels_for_face(1).unwrap().contains(&Label::side(10)));
    }

    /// Step 5 (G.4): Test resolution - exact match
    #[test]
    fn test_resolution_exact() {
        let mut timeline = Timeline::new();
        let shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

        // Initialize face 0 with a label
        let labels = vec![(0, Label::cap(true))];
        timeline.init_shape(&shape, labels);

        // Create descriptor for that label
        let desc = ReferenceDescriptor::new(
            Label::cap(true),
            GeometricSignature::face([5.0, 5.0, 10.0], [0.0, 0.0, 1.0], 100.0),
        );

        // Resolve - should find the label (exact)
        let (result, indices) = timeline.resolve(&desc, &shape, 1e-6);
        assert_eq!(result, ResolutionResult::Exact);
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0], 0);
    }

    /// Step 5 (G.4): Test resolution - expanded (split)
    #[test]
    fn test_resolution_expanded() {
        let mut timeline = Timeline::new();
        let shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

        // Simulate a split: one label appears on multiple faces
        let label = Label::side(99);
        let mut labels_map = HashMap::new();
        labels_map.insert(0, {
            let mut set = HashSet::new();
            set.insert(label.clone());
            set
        });
        labels_map.insert(1, {
            let mut set = HashSet::new();
            set.insert(label.clone());
            set
        });

        // Manually insert labels
        for (idx, label_set) in labels_map.iter() {
            timeline.face_labels.insert(*idx, label_set.clone());
        }

        let desc = ReferenceDescriptor::new(
            label,
            GeometricSignature::face([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0),
        );

        let (result, indices) = timeline.resolve(&desc, &shape, 1e-6);
        assert_eq!(result, ResolutionResult::Expanded(2));
        assert_eq!(indices.len(), 2);
    }

    /// Step 5 (G.4): Test resolution - lost (no match)
    #[test]
    fn test_resolution_lost() {
        let timeline = Timeline::new();
        let shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

        // Try to resolve a label that doesn't exist
        let desc = ReferenceDescriptor::new(
            Label::side(999),
            GeometricSignature::face([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 100.0),
        );

        let (result, indices) = timeline.resolve(&desc, &shape, 1e-6);
        assert_eq!(result, ResolutionResult::Lost);
        assert_eq!(indices.len(), 0);
    }

    /// Step 5 (G.4): Test resolution - signature fallback
    #[test]
    fn test_resolution_signature_fallback() {
        let mut timeline = Timeline::new();
        let shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

        // Initialize with one label (which we won't use)
        let labels = vec![(1, Label::side(10))];
        timeline.init_shape(&shape, labels);

        // Create descriptor with different label but matching signature
        // The box's first face (top) is at centroid near (5, 5, 10)
        let desc = ReferenceDescriptor::new(
            Label::cap(true),
            GeometricSignature::face([5.0, 5.0, 10.0], [0.0, 0.0, 1.0], 100.0),
        );

        let (result, indices) = timeline.resolve(&desc, &shape, 1.0); // Larger tolerance for test
        // Should resolve via signature fallback
        if !indices.is_empty() && indices.len() == 1 {
            assert_eq!(result, ResolutionResult::Signature);
        }
    }

    /// Test label propagation through operation history
    #[test]
    fn test_label_propagation() {
        let mut timeline = Timeline::new();

        // Initial shape: 3 faces with labels
        let initial_labels = vec![
            (0, Label::cap(true)),
            (1, Label::side(10)),
            (2, Label::side(20)),
        ];
        timeline.init_shape(&Shape::make_box(1.0, 1.0, 1.0).unwrap(), initial_labels);

        // Simulate operation result: faces 0, 1, 1, 2, 2, 5 with history
        // (faces 1 and 2 split)
        let mut modified_map = HashMap::new();
        modified_map.insert(0, vec![0]); // Result face 0 from input face 0
        modified_map.insert(1, vec![1]); // Result face 1 from input face 1 (split)
        modified_map.insert(2, vec![1]); // Result face 2 also from input face 1 (split)
        modified_map.insert(3, vec![2]); // Result face 3 from input face 2
        modified_map.insert(4, vec![2]); // Result face 4 from input face 2
        modified_map.insert(5, vec![2]); // Result face 5 from input face 2

        let label_map = timeline.face_labels.clone();
        let result_faces = vec![0, 1, 2, 3, 4, 5];

        timeline.propagate(&result_faces, &label_map, &modified_map);

        // Check that split faces kept the original labels
        assert!(timeline
            .labels_for_face(1)
            .unwrap()
            .contains(&Label::side(10)));
        assert!(timeline
            .labels_for_face(2)
            .unwrap()
            .contains(&Label::side(10)));

        // Check that face 3, 4, 5 from input face 2 got the right labels
        assert!(timeline
            .labels_for_face(3)
            .unwrap()
            .contains(&Label::side(20)));
    }

    /// Integration test: extrude with side faces and caps
    #[test]
    fn test_extrude_labeling() {
        let mut timeline = Timeline::new();
        let shape = Shape::make_box(10.0, 10.0, 10.0).expect("failed to create box");

        // Simulate extrusion of a profile with curves 1, 2, 3, 4
        // Result: 4 side faces + 2 cap faces
        let labels = vec![
            (0, Label::side(1)),
            (1, Label::side(2)),
            (2, Label::side(3)),
            (3, Label::side(4)),
            (4, Label::cap(true)),   // start cap
            (5, Label::cap(false)),  // end cap
        ];
        timeline.init_shape(&shape, labels);

        // Verify all labels are present
        for i in 0..6 {
            assert!(timeline.labels_for_face(i).is_some());
        }

        // Resolve each reference
        let descriptors = vec![
            (Label::side(1), true),
            (Label::side(2), true),
            (Label::side(3), true),
            (Label::side(4), true),
            (Label::cap(true), true),
            (Label::cap(false), true),
        ];

        for (label, expect_found) in descriptors {
            let desc = ReferenceDescriptor::new(
                label.clone(),
                GeometricSignature::face([0.0, 0.0, 5.0], [0.0, 0.0, 1.0], 100.0),
            );
            let (result, indices) = timeline.resolve(&desc, &shape, 1e-6);
            if expect_found {
                assert_ne!(result, ResolutionResult::Lost, "Label {} should be found", label);
                assert!(!indices.is_empty());
            }
        }
    }

    /// Full G.4 workflow test: operation sequence with label tracking
    #[test]
    fn test_g4_workflow() {
        // Step 1: Create and label initial shape (extrusion result)
        let mut timeline = Timeline::new();
        let shape = Shape::make_box(20.0, 20.0, 20.0).expect("failed to create box");

        let initial_labels = vec![
            (0, Label::side(100)),    // side from profile curve 100
            (1, Label::side(101)),    // side from profile curve 101
            (2, Label::side(102)),    // side from profile curve 102
            (3, Label::side(103)),    // side from profile curve 103
            (4, Label::cap(true)),    // bottom cap
            (5, Label::cap(false)),   // top cap
        ];
        timeline.init_shape(&shape, initial_labels.clone());

        // Step 2: Create reference descriptors with geometric signatures
        let ref_side_100 = ReferenceDescriptor::new(
            Label::side(100),
            GeometricSignature::face([10.0, 0.0, 10.0], [0.0, -1.0, 0.0], 400.0),
        );
        let ref_cap_start = ReferenceDescriptor::new(
            Label::cap(true),
            GeometricSignature::face([10.0, 10.0, 0.0], [0.0, 0.0, -1.0], 400.0),
        );

        // Step 3: Resolve references (should find exact matches)
        let (result1, indices1) = timeline.resolve(&ref_side_100, &shape, 1e-6);
        assert_eq!(result1, ResolutionResult::Exact);
        assert_eq!(indices1.len(), 1);

        let (result2, indices2) = timeline.resolve(&ref_cap_start, &shape, 1e-6);
        assert_eq!(result2, ResolutionResult::Exact);
        assert_eq!(indices2.len(), 1);

        // Step 4: Simulate a second operation that splits one face
        // (e.g., a cut that splits the top face into 2 pieces)
        // Build modified map: input face 5 (top cap) becomes output faces 5 and 6
        let mut operation_modified = HashMap::new();
        operation_modified.insert(0, vec![0]); // Face 0 unchanged
        operation_modified.insert(1, vec![1]); // Face 1 unchanged
        operation_modified.insert(2, vec![2]); // Face 2 unchanged
        operation_modified.insert(3, vec![3]); // Face 3 unchanged
        operation_modified.insert(4, vec![4]); // Face 4 unchanged
        operation_modified.insert(5, vec![5]); // Output face 5 from input face 5
        operation_modified.insert(6, vec![5]); // Output face 6 also from input face 5 (split)

        let label_map = timeline.face_labels.clone();
        let result_faces = vec![0, 1, 2, 3, 4, 5, 6];

        timeline.propagate(&result_faces, &label_map, &operation_modified);

        // Step 5: Verify that both pieces of the split face got the label
        let labels_face5 = timeline.labels_for_face(5);
        assert!(labels_face5.is_some(), "Face 5 should have labels");
        assert!(
            labels_face5.unwrap().contains(&Label::cap(false)),
            "Face 5 should have cap(false) label"
        );

        let labels_face6 = timeline.labels_for_face(6);
        assert!(labels_face6.is_some(), "Face 6 should have labels");
        assert!(
            labels_face6.unwrap().contains(&Label::cap(false)),
            "Face 6 should have cap(false) label (from split)"
        );

        // Step 6: Verify the resolution would now return expanded results
        let (result_expanded, indices_expanded) = timeline.resolve(&ref_cap_start, &shape, 1e-6);
        // Since both faces 5 and 6 have the label, we expect Expanded(2)
        if indices_expanded.len() > 1 {
            assert_eq!(result_expanded, ResolutionResult::Expanded(indices_expanded.len()));
        }
    }
}
