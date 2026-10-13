// Thin, coarse-grained OCCT shim. All functions throw std::runtime_error on OCCT failure
// (cxx turns that into a Rust `Err`).
#pragma once

#include <TopoDS_Shape.hxx>
#include <cstdint>
#include <memory>
#include <string>

#include "rust/cxx.h"

namespace mkcad {

struct HistEntry;
struct OpResult;
struct FaceProps;
struct EdgeProps;

class Shape {
public:
  TopoDS_Shape s;
};

std::unique_ptr<Shape> make_box(double dx, double dy, double dz);
// `xy` is a polygon [x0,y0,x1,y1,...] in the XY plane (implicitly closed), extruded along +Z.
OpResult extrude_polygon(rust::Slice<const double> xy, double height);
std::unique_ptr<Shape> translate(const Shape& a, double x, double y, double z);
OpResult fillet_edges(const Shape& a, rust::Slice<const uint32_t> edges, double radius);
OpResult boolean_cut(const Shape& a, const Shape& tool);
OpResult boolean_fuse(const Shape& a, const Shape& tool);

double volume(const Shape& a);
bool is_valid(const Shape& a);
uint32_t count_faces(const Shape& a);
uint32_t count_edges(const Shape& a);
FaceProps face_props(const Shape& a, uint32_t index);
EdgeProps edge_props(const Shape& a, uint32_t index);

void export_step(const Shape& a, rust::Str path);
std::unique_ptr<Shape> import_step(rust::Str path);

} // namespace mkcad
