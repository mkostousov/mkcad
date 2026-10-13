#include "occt-sys/include/shim.h"

#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeShape.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <GProp_GProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
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
    return FaceProps{p.Mass(), c.X(), c.Y(), c.Z()};
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

void export_step(const Shape& a, rust::Str path) {
  guard([&] {
    STEPControl_Writer w;
    if (w.Transfer(a.s, STEPControl_AsIs) != IFSelect_RetDone) throw std::runtime_error("STEP transfer failed");
    std::string p(path.data(), path.size());
    if (w.Write(p.c_str()) != IFSelect_RetDone) throw std::runtime_error("STEP write failed: " + p);
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
