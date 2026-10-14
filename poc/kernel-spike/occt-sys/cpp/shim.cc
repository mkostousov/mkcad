#include "occt-sys/include/shim.h"

#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeShape.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepGProp.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <GeomAbs_SurfaceType.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <gp_Pln.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <GProp_GProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Interface_Static.hxx>
#include <fstream>
#include <iterator>
#include <regex>
#include <Standard_Failure.hxx>
#include <TopExp.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

#include <stdexcept>
#include <string>

#include "occt-sys/src/lib.rs.h"

namespace mkcad {
namespace {

template <class F> auto guard(F&& f) -> decltype(f()) {
  try {
    return f();
  } catch (const Standard_Failure& e) {
    const char* msg = e.GetMessageString();
    throw std::runtime_error(std::string(e.ExceptionType()) + ": " + (msg ? msg : ""));
  }
}

std::unique_ptr<Shape> wrap(const TopoDS_Shape& s) {
  auto p = std::make_unique<Shape>();
  p->s = s;
  return p;
}

// Record how `op` mapped the sub-shapes (edges, faces) of `in` into `out`. Results are resolved in the map
// matching their own type (an edge generates a face), and `dst_dim` says which one.
void collect(BRepBuilderAPI_MakeShape& op, uint8_t arg, const TopoDS_Shape& in, const TopoDS_Shape& out,
             rust::Vec<HistEntry>& h) {
  TopTools_IndexedMapOfShape dst_edges, dst_faces;
  TopExp::MapShapes(out, TopAbs_EDGE, dst_edges);
  TopExp::MapShapes(out, TopAbs_FACE, dst_faces);
  auto find = [&](const TopoDS_Shape& s, uint8_t& dst_dim) -> uint32_t {
    if (s.ShapeType() == TopAbs_EDGE) {
      dst_dim = 1;
      return static_cast<uint32_t>(dst_edges.FindIndex(s));
    }
    if (s.ShapeType() == TopAbs_FACE) {
      dst_dim = 2;
      return static_cast<uint32_t>(dst_faces.FindIndex(s));
    }
    dst_dim = 0;
    return 0;
  };
  const TopAbs_ShapeEnum kinds[2] = {TopAbs_EDGE, TopAbs_FACE};
  for (int k = 0; k < 2; ++k) {
    TopTools_IndexedMapOfShape src;
    TopExp::MapShapes(in, kinds[k], src);
    const uint8_t dim = static_cast<uint8_t>(k + 1);
    for (int i = 1; i <= src.Extent(); ++i) {
      const TopoDS_Shape& sub = src(i);
      auto push = [&](uint8_t kind, uint8_t dst_dim, uint32_t d) {
        h.push_back(HistEntry{kind, arg, dim, static_cast<uint32_t>(i), dst_dim, d});
      };
      const bool deleted = op.IsDeleted(sub);
      if (deleted) push(2, 0, 0);
      bool modified = false;
      for (const TopoDS_Shape& m : op.Modified(sub)) {
        uint8_t dd;
        if (uint32_t d = find(m, dd)) {
          push(0, dd, d);
          modified = true;
        }
      }
      for (const TopoDS_Shape& g : op.Generated(sub)) {
        uint8_t dd;
        if (uint32_t d = find(g, dd)) push(1, dd, d);
      }
      if (!deleted && !modified) {
        uint8_t dd;
        if (uint32_t d = find(sub, dd)) push(3, dd, d);
      }
    }
  }
}

TopoDS_Shape sub_shape(const Shape& a, TopAbs_ShapeEnum kind, uint32_t index) {
  TopTools_IndexedMapOfShape m;
  TopExp::MapShapes(a.s, kind, m);
  if (index < 1 || static_cast<int>(index) > m.Extent()) throw std::runtime_error("sub-shape index out of range");
  return m(static_cast<int>(index));
}

void require_done(BRepBuilderAPI_MakeShape& op, const char* what) {
  if (!op.IsDone()) throw std::runtime_error(std::string(what) + ": operation not done");
}

} // namespace

std::unique_ptr<Shape> make_box(double dx, double dy, double dz) {
  return guard([&] { return wrap(BRepPrimAPI_MakeBox(dx, dy, dz).Shape()); });
}

OpResult extrude_polygon(rust::Slice<const double> xy, double height) {
  return guard([&] {
    if (xy.size() < 6 || xy.size() % 2 != 0) throw std::runtime_error("polygon needs >= 3 points");
    BRepBuilderAPI_MakePolygon poly;
    for (size_t i = 0; i < xy.size(); i += 2) poly.Add(gp_Pnt(xy[i], xy[i + 1], 0.0));
    poly.Close();
    if (!poly.IsDone()) throw std::runtime_error("polygon wire failed");
    BRepBuilderAPI_MakeFace face(poly.Wire(), /*OnlyPlane=*/true);
    if (!face.IsDone()) throw std::runtime_error("planar face failed");
    BRepPrimAPI_MakePrism prism(face.Face(), gp_Vec(0, 0, height));
    require_done(prism, "prism");
    OpResult r;
    r.shape = wrap(prism.Shape());
    collect(prism, 0, face.Face(), prism.Shape(), r.history);
    {
      // The end cap is `LastShape()` of the sweep, not in the per-edge history (kind 4).
      TopTools_IndexedMapOfShape faces;
      TopExp::MapShapes(prism.Shape(), TopAbs_FACE, faces);
      int d = faces.FindIndex(prism.LastShape());
      if (d > 0) r.history.push_back(HistEntry{4, 0, 2, 1, 2, static_cast<uint32_t>(d)});
    }
    return r;
  });
}

std::unique_ptr<Shape> translate(const Shape& a, double x, double y, double z) {
  return guard([&] {
    gp_Trsf t;
    t.SetTranslation(gp_Vec(x, y, z));
    return wrap(BRepBuilderAPI_Transform(a.s, t, /*Copy=*/true).Shape());
  });
}

OpResult fillet_edges(const Shape& a, rust::Slice<const uint32_t> edges, double radius) {
  return guard([&] {
    BRepFilletAPI_MakeFillet mk(a.s);
    for (uint32_t e : edges) mk.Add(radius, TopoDS::Edge(sub_shape(a, TopAbs_EDGE, e)));
    mk.Build();
    require_done(mk, "fillet");
    OpResult r;
    r.shape = wrap(mk.Shape());
    collect(mk, 0, a.s, mk.Shape(), r.history);
    return r;
  });
}

template <class Algo> OpResult boolean(const Shape& a, const Shape& tool, const char* what) {
  return guard([&] {
    Algo op(a.s, tool.s);
    if (op.HasErrors()) throw std::runtime_error(std::string(what) + ": boolean reported errors");
    require_done(op, what);
    OpResult r;
    r.shape = wrap(op.Shape());
    collect(op, 0, a.s, op.Shape(), r.history);
    collect(op, 1, tool.s, op.Shape(), r.history);
    return r;
  });
}

OpResult boolean_cut(const Shape& a, const Shape& tool) { return boolean<BRepAlgoAPI_Cut>(a, tool, "cut"); }
OpResult boolean_fuse(const Shape& a, const Shape& tool) { return boolean<BRepAlgoAPI_Fuse>(a, tool, "fuse"); }

double volume(const Shape& a) {
  return guard([&] {
    GProp_GProps p;
    BRepGProp::VolumeProperties(a.s, p);
    return p.Mass();
  });
}

bool is_valid(const Shape& a) { return BRepCheck_Analyzer(a.s).IsValid(); }

uint32_t count_faces(const Shape& a) {
  TopTools_IndexedMapOfShape m;
  TopExp::MapShapes(a.s, TopAbs_FACE, m);
  return static_cast<uint32_t>(m.Extent());
}

uint32_t count_edges(const Shape& a) {
  TopTools_IndexedMapOfShape m;
  TopExp::MapShapes(a.s, TopAbs_EDGE, m);
  return static_cast<uint32_t>(m.Extent());
}

FaceProps face_props(const Shape& a, uint32_t index) {
  return guard([&] {
    GProp_GProps p;
    BRepGProp::SurfaceProperties(sub_shape(a, TopAbs_FACE, index), p);
    gp_Pnt c = p.CentreOfMass();
    double nx = 0, ny = 0, nz = 0;
    const TopoDS_Face f = TopoDS::Face(sub_shape(a, TopAbs_FACE, index));
    BRepAdaptor_Surface surf(f);
    if (surf.GetType() == GeomAbs_Plane) {
      gp_Dir n = surf.Plane().Axis().Direction();
      if (f.Orientation() == TopAbs_REVERSED) n.Reverse();
      nx = n.X();
      ny = n.Y();
      nz = n.Z();
    }
    return FaceProps{p.Mass(), c.X(), c.Y(), c.Z(), nx, ny, nz};
  });
}

EdgeProps edge_props(const Shape& a, uint32_t index) {
  return guard([&] {
    GProp_GProps p;
    BRepGProp::LinearProperties(sub_shape(a, TopAbs_EDGE, index), p);
    gp_Pnt c = p.CentreOfMass();
    return EdgeProps{p.Mass(), c.X(), c.Y(), c.Z()};
  });
}

rust::Vec<uint32_t> edge_faces(const Shape& a, uint32_t index) {
  return guard([&] {
    TopTools_IndexedDataMapOfShapeListOfShape anc;
    TopExp::MapShapesAndAncestors(a.s, TopAbs_EDGE, TopAbs_FACE, anc);
    TopTools_IndexedMapOfShape faces;
    TopExp::MapShapes(a.s, TopAbs_FACE, faces);
    int ei = anc.FindIndex(sub_shape(a, TopAbs_EDGE, index));
    rust::Vec<uint32_t> out;
    if (ei == 0) return out;
    for (const TopoDS_Shape& f : anc(ei)) {
      int fi = faces.FindIndex(f);
      if (fi > 0) out.push_back(static_cast<uint32_t>(fi));
    }
    return out;
  });
}

OpResult shell_faces(const Shape& a, rust::Slice<const uint32_t> faces, double thickness) {
  return guard([&] {
    TopTools_ListOfShape closing;
    for (uint32_t f : faces) closing.Append(sub_shape(a, TopAbs_FACE, f));
    BRepOffsetAPI_MakeThickSolid mk;
    mk.MakeThickSolidByJoin(a.s, closing, -thickness, 1e-3);
    require_done(mk, "shell");
    OpResult r;
    r.shape = wrap(mk.Shape());
    collect(mk, 0, a.s, mk.Shape(), r.history);
    return r;
  });
}

void export_step(const Shape& a, rust::Str path) {
  guard([&] {
    STEPControl_Writer w;
    if (w.Transfer(a.s, STEPControl_AsIs) != IFSelect_RetDone) throw std::runtime_error("STEP transfer failed");
    std::string p(path.data(), path.size());
    if (w.Write(p.c_str()) != IFSelect_RetDone) throw std::runtime_error("STEP write failed: " + p);
    return 0;
  });
}

void export_step_schema(const Shape& a, rust::Str path, rust::Str schema) {
  guard([&] {
    STEPControl_Writer w;  // registers the STEP parameters (Interface_Static) on construction
    std::string s(schema.data(), schema.size());
    if (!Interface_Static::SetCVal("write.step.schema", s.c_str())) throw std::runtime_error("unknown STEP schema: " + s);
    Interface_Static::SetCVal("write.step.unit", "MM");
    // Without this the product name carries a per-process export counter ("... 8.0 2", "... 8.0 6").
    Interface_Static::SetCVal("write.step.product.name", "mkcad-part");
    if (w.Transfer(a.s, STEPControl_AsIs) != IFSelect_RetDone) throw std::runtime_error("STEP transfer failed");
    std::string p(path.data(), path.size());
    if (w.Write(p.c_str()) != IFSelect_RetDone) throw std::runtime_error("STEP write failed: " + p);
    // OCCT appends a per-process export counter to the product name ("mkcad-part 3"); strip it so that the
    // DATA section depends on the shape only (K-4).
    std::string text;
    {
      std::ifstream in(p, std::ios::binary);
      text.assign(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
    }
    text = std::regex_replace(text, std::regex("'mkcad-part [0-9]+'"), "'mkcad-part'");
    std::ofstream out(p, std::ios::binary | std::ios::trunc);
    out << text;
    return 0;
  });
}

std::unique_ptr<Shape> import_step(rust::Str path) {
  return guard([&] {
    STEPControl_Reader rd;
    std::string p(path.data(), path.size());
    if (rd.ReadFile(p.c_str()) != IFSelect_RetDone) throw std::runtime_error("STEP read failed: " + p);
    if (rd.TransferRoots() <= 0) throw std::runtime_error("STEP has no transferable roots");
    return wrap(rd.OneShape());
  });
}

} // namespace mkcad
