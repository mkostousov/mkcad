#include <Standard_TypeDef.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Edge.hxx>
#include <GProp_GProps.hxx>
#include <limits>
#include <cmath>

extern "C" {

struct mk_shape {
    TopoDS_Shape* shape;
};

mk_shape* mk_box(double dx, double dy, double dz) {
    try {
        TopoDS_Shape shape = BRepPrimAPI_MakeBox(dx, dy, dz);
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_cylinder(double r, double h) {
    try {
        TopoDS_Shape shape = BRepPrimAPI_MakeCylinder(r, h);
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_cut(const mk_shape* a, const mk_shape* b) {
    try {
        if (!a || !a->shape || !b || !b->shape) return nullptr;
        TopoDS_Shape shape = BRepAlgoAPI_Cut(*a->shape, *b->shape);
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_fuse(const mk_shape* a, const mk_shape* b) {
    try {
        if (!a || !a->shape || !b || !b->shape) return nullptr;
        TopoDS_Shape shape = BRepAlgoAPI_Fuse(*a->shape, *b->shape);
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_fillet_all(const mk_shape* s, double r) {
    try {
        if (!s || !s->shape) return nullptr;

        BRepFilletAPI_MakeFillet filler(*s->shape);

        // Iterate through all edges using TopExp_Explorer
        TopExp_Explorer exp(*s->shape, TopAbs_EDGE);
        for (; exp.More(); exp.Next()) {
            const TopoDS_Edge& edge = TopoDS::Edge(exp.Current());
            filler.Add(r, edge);
        }

        TopoDS_Shape shape = filler.Shape();
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

double mk_volume(const mk_shape* s) {
    try {
        if (!s || !s->shape) return -1.0;

        GProp_GProps gprops;
        BRepGProp::VolumeProperties(*s->shape, gprops);
        return gprops.Mass();
    } catch (...) {
        return std::numeric_limits<double>::quiet_NaN();
    }
}

int mk_count(const mk_shape* s, int kind) {
    try {
        if (!s || !s->shape) return -1;

        TopAbs_ShapeEnum type;
        if (kind == 0) type = TopAbs_FACE;
        else if (kind == 1) type = TopAbs_EDGE;
        else if (kind == 2) type = TopAbs_VERTEX;
        else return -1;

        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> map;
        TopExp::MapShapes(*s->shape, type, map);
        return map.Extent();
    } catch (...) {
        return -1;
    }
}

int mk_is_valid(const mk_shape* s) {
    try {
        if (!s || !s->shape) return 0;

        BRepCheck_Analyzer analyzer(*s->shape);
        return analyzer.IsValid() ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

void mk_free(mk_shape* s) {
    if (s) {
        delete s->shape;
        delete s;
    }
}

} // extern "C"
