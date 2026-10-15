#include <Standard_TypeDef.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepGProp.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepTools_History.hxx>
#include <BRepTools.hxx>
#include <BRep_Tool.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Vertex.hxx>
#include <Geom_Surface.hxx>
#include <GProp_GProps.hxx>
#include <GeomLProp_SLProps.hxx>
#include <gp_Pnt.hxx>
#include <gp_Vec.hxx>
#include <gp_Ax2.hxx>
#include <gp_Trsf.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Interface_Static.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <fstream>
#include <regex>
#include <limits>
#include <cmath>
#include <vector>

extern "C" {

struct mk_shape {
    TopoDS_Shape* shape;
};

struct mk_op {
    TopoDS_Shape* result;
    std::vector<TopoDS_Shape>* operands;
    Handle(BRepTools_History)* hist;
};

// Helper: map shape to 0-based index
int shape_index_helper(const TopoDS_Shape& s, const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& map) {
    int idx = map.FindIndex(s);
    return idx > 0 ? idx - 1 : -1;  // Convert 1-based to 0-based
}

// Helper: get shape at 0-based index
TopoDS_Shape shape_at_index_helper(const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& map, int idx) {
    if (idx < 0 || idx >= map.Extent()) return TopoDS_Shape();
    return map(idx + 1);  // Convert 0-based to 1-based
}

#define shape_index shape_index_helper
#define shape_at_index shape_at_index_helper

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

mk_shape* mk_translate(const mk_shape* s, double dx, double dy, double dz) {
    try {
        if (!s || !s->shape) return nullptr;
        gp_Trsf trsf;
        trsf.SetTranslation(gp_Vec(dx, dy, dz));
        BRepBuilderAPI_Transform transformer(*s->shape, trsf, true);  // copy=true
        TopoDS_Shape shape = transformer.Shape();
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_cylinder_at(double x, double y, double z, double r, double h) {
    try {
        gp_Ax2 axis(gp_Pnt(x, y, z), gp_Vec(0.0, 0.0, 1.0));
        TopoDS_Shape shape = BRepPrimAPI_MakeCylinder(axis, r, h);
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_prism_polygon(const double* xy, int n, double h) {
    try {
        if (!xy || n < 3) return nullptr;

        BRepBuilderAPI_MakePolygon polygon;
        for (int i = 0; i < n; i++) {
            polygon.Add(gp_Pnt(xy[2*i], xy[2*i+1], 0.0));
        }
        polygon.Close();
        if (!polygon.IsDone()) return nullptr;

        TopoDS_Wire wire = polygon.Wire();
        BRepBuilderAPI_MakeFace face(wire);
        if (!face.IsDone()) return nullptr;

        TopoDS_Face faceShape = face.Face();
        gp_Vec prismDir(0.0, 0.0, h);
        TopoDS_Shape shape = BRepPrimAPI_MakePrism(faceShape, prismDir);

        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(shape);
        return result;
    } catch (...) {
        return nullptr;
    }
}

mk_op* mk_op_cut(const mk_shape* a, const mk_shape* b) {
    try {
        if (!a || !a->shape || !b || !b->shape) return nullptr;

        BRepAlgoAPI_Cut cut(*a->shape, *b->shape);
        if (!cut.IsDone()) return nullptr;

        mk_op* op = new mk_op;
        op->result = new TopoDS_Shape(cut.Shape());
        op->operands = new std::vector<TopoDS_Shape>{*a->shape, *b->shape};
        op->hist = new Handle(BRepTools_History)(cut.History());
        return op;
    } catch (...) {
        return nullptr;
    }
}

mk_op* mk_op_fuse(const mk_shape* a, const mk_shape* b) {
    try {
        if (!a || !a->shape || !b || !b->shape) return nullptr;

        BRepAlgoAPI_Fuse fuse(*a->shape, *b->shape);
        if (!fuse.IsDone()) return nullptr;

        mk_op* op = new mk_op;
        op->result = new TopoDS_Shape(fuse.Shape());
        op->operands = new std::vector<TopoDS_Shape>{*a->shape, *b->shape};
        op->hist = new Handle(BRepTools_History)(fuse.History());
        return op;
    } catch (...) {
        return nullptr;
    }
}

mk_op* mk_op_fillet(const mk_shape* s, const int* edge_idx, int n, double r) {
    try {
        if (!s || !s->shape || !edge_idx || n <= 0) return nullptr;

        BRepFilletAPI_MakeFillet filler(*s->shape);

        // Map edges to get proper indexing
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edgeMap;
        TopExp::MapShapes(*s->shape, TopAbs_EDGE, edgeMap);

        // Add selected edges
        for (int i = 0; i < n; i++) {
            TopoDS_Shape edgeShape = shape_at_index(edgeMap, edge_idx[i]);
            if (edgeShape.IsNull()) return nullptr;
            const TopoDS_Edge& edge = TopoDS::Edge(edgeShape);
            filler.Add(r, edge);
        }

        // Build the fillet
        filler.Build();
        if (!filler.IsDone()) return nullptr;

        TopoDS_Shape shape = filler.Shape();

        mk_op* op = new mk_op;
        op->result = new TopoDS_Shape(shape);
        op->operands = new std::vector<TopoDS_Shape>{*s->shape};
        // BRepFilletAPI_MakeFillet answers Modified/Generated/IsDeleted per input sub-shape; collect them
        // into a BRepTools_History so the history queries work as for booleans.
        Handle(BRepTools_History) hist = new BRepTools_History();
        const TopAbs_ShapeEnum kinds[3] = {TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX};
        for (TopAbs_ShapeEnum k : kinds) {
            NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> m;
            TopExp::MapShapes(*s->shape, k, m);
            for (int i = 1; i <= m.Extent(); i++) {
                const TopoDS_Shape& sub = m(i);
                // The filleted edge itself is deleted but still generates the fillet face: record both.
                for (const TopoDS_Shape& g : filler.Generated(sub)) {
                    hist->AddGenerated(sub, g);
                }
                if (filler.IsDeleted(sub)) {
                    hist->Remove(sub);
                    continue;
                }
                for (const TopoDS_Shape& mod : filler.Modified(sub)) {
                    if (!mod.IsSame(sub)) hist->AddModified(sub, mod);
                }
            }
        }
        op->hist = new Handle(BRepTools_History)(hist);
        return op;
    } catch (...) {
        return nullptr;
    }
}

mk_op* mk_op_prism_polygon(const double* xy, int n, double h) {
    try {
        if (!xy || n < 3) return nullptr;

        BRepBuilderAPI_MakePolygon polygon;
        for (int i = 0; i < n; i++) {
            polygon.Add(gp_Pnt(xy[2*i], xy[2*i+1], 0.0));
        }
        polygon.Close();
        if (!polygon.IsDone()) return nullptr;

        TopoDS_Wire wire = polygon.Wire();
        BRepBuilderAPI_MakeFace face(wire);
        if (!face.IsDone()) return nullptr;

        TopoDS_Face faceShape = face.Face();
        gp_Vec prismDir(0.0, 0.0, h);
        BRepPrimAPI_MakePrism prism(faceShape, prismDir);
        if (!prism.IsDone()) return nullptr;

        mk_op* op = new mk_op;
        op->result = new TopoDS_Shape(prism.Shape());
        op->operands = new std::vector<TopoDS_Shape>{faceShape};

        // Build history manually for prism using Generated()
        Handle(BRepTools_History) hist = new BRepTools_History();
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edgeMap;
        TopExp::MapShapes(faceShape, TopAbs_EDGE, edgeMap);
        for (int i = 1; i <= edgeMap.Extent(); i++) {
            const TopoDS_Shape& edge = edgeMap(i);
            const NCollection_List<TopoDS_Shape>& genShapes = prism.Generated(edge);
            for (const TopoDS_Shape& genShape : genShapes) {
                hist->AddGenerated(edge, genShape);
            }
        }
        op->hist = new Handle(BRepTools_History)(hist);
        return op;
    } catch (...) {
        return nullptr;
    }
}

mk_shape* mk_op_shape(const mk_op* op) {
    try {
        if (!op || !op->result) return nullptr;
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(*op->result);
        return result;
    } catch (...) {
        return nullptr;
    }
}

void mk_op_free(mk_op* op) {
    if (op) {
        if (op->result) delete op->result;
        if (op->operands) delete op->operands;
        if (op->hist) delete op->hist;
        delete op;
    }
}

int mk_op_modified(const mk_op* op, int operand, int kind, int index, int* out, int cap) {
    try {
        if (!op || !op->result || !op->operands || operand < 0 || operand >= (int)op->operands->size()) return -1;
        if (!out || cap <= 0) return -1;

        TopAbs_ShapeEnum type;
        if (kind == 0) type = TopAbs_FACE;
        else if (kind == 1) type = TopAbs_EDGE;
        else if (kind == 2) type = TopAbs_VERTEX;
        else return -1;

        // Map operand sub-shapes
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> operandMap;
        TopExp::MapShapes((*op->operands)[operand], type, operandMap);
        if (index < 0 || index >= operandMap.Extent()) return -1;

        TopoDS_Shape subShape = operandMap(index + 1);  // 1-based in OCCT

        // Map result shapes
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> resultMap;
        TopExp::MapShapes(*op->result, type, resultMap);

        // Check history if available
        if (op->hist && !op->hist->IsNull()) {
            Handle(BRepTools_History) hist = *op->hist;

            // Check if deleted
            if (hist->IsRemoved(subShape)) {
                return 0;  // Deleted, no modifications
            }

            // Get modified shapes
            const NCollection_List<TopoDS_Shape>& modList = hist->Modified(subShape);
            int count = 0;
            for (const TopoDS_Shape& mod : modList) {
                int idx = shape_index(mod, resultMap);
                if (idx >= 0 && count < cap) {
                    out[count++] = idx;
                }
            }
            if (count > 0) return count;
        }

        // If no history or empty modified list, check if sub-shape exists unchanged in result
        int idx = shape_index(subShape, resultMap);
        if (idx >= 0) {
            if (cap > 0) out[0] = idx;
            return 1;
        }

        return 0;
    } catch (...) {
        return -1;
    }
}

int mk_op_generated(const mk_op* op, int operand, int kind, int index, int* out_kind, int* out, int cap) {
    try {
        if (!op || !op->result || !op->operands || operand < 0 || operand >= (int)op->operands->size()) return -1;
        if (!out || !out_kind || cap <= 0) return -1;

        TopAbs_ShapeEnum type;
        if (kind == 0) type = TopAbs_FACE;
        else if (kind == 1) type = TopAbs_EDGE;
        else if (kind == 2) type = TopAbs_VERTEX;
        else return -1;

        // Map operand sub-shapes
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> operandMap;
        TopExp::MapShapes((*op->operands)[operand], type, operandMap);
        if (index < 0 || index >= operandMap.Extent()) return -1;

        TopoDS_Shape subShape = operandMap(index + 1);

        if (!op->hist || op->hist->IsNull()) {
            return 0;  // No history available
        }

        Handle(BRepTools_History) hist = *op->hist;
        const NCollection_List<TopoDS_Shape>& genList = hist->Generated(subShape);

        if (genList.IsEmpty()) return 0;

        // Determine generated shape kind and map indices
        TopAbs_ShapeEnum genType = TopAbs_COMPOUND;
        if (!genList.IsEmpty()) {
            genType = genList.First().ShapeType();
        }

        int genKind;
        if (genType == TopAbs_FACE) genKind = 0;
        else if (genType == TopAbs_EDGE) genKind = 1;
        else if (genType == TopAbs_VERTEX) genKind = 2;
        else return -1;

        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> resultMap;
        TopExp::MapShapes(*op->result, genType, resultMap);

        int count = 0;
        for (const TopoDS_Shape& gen : genList) {
            int idx = shape_index(gen, resultMap);
            if (idx >= 0 && count < cap) {
                out[count] = idx;
                out_kind[count] = genKind;
                count++;
            }
        }
        return count;
    } catch (...) {
        return -1;
    }
}

int mk_op_is_deleted(const mk_op* op, int operand, int kind, int index) {
    try {
        if (!op || !op->operands || operand < 0 || operand >= (int)op->operands->size()) return -1;

        TopAbs_ShapeEnum type;
        if (kind == 0) type = TopAbs_FACE;
        else if (kind == 1) type = TopAbs_EDGE;
        else if (kind == 2) type = TopAbs_VERTEX;
        else return -1;

        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> operandMap;
        TopExp::MapShapes((*op->operands)[operand], type, operandMap);
        if (index < 0 || index >= operandMap.Extent()) return -1;

        TopoDS_Shape subShape = operandMap(index + 1);

        if (!op->hist || op->hist->IsNull()) {
            return 0;  // No history, assume not deleted
        }

        Handle(BRepTools_History) hist = *op->hist;
        return hist->IsRemoved(subShape) ? 1 : 0;
    } catch (...) {
        return -1;
    }
}

int mk_face_info(const mk_shape* s, int index, double* out7) {
    try {
        if (!s || !s->shape || !out7) return -1;

        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> faceMap;
        TopExp::MapShapes(*s->shape, TopAbs_FACE, faceMap);
        if (index < 0 || index >= faceMap.Extent()) return -1;

        TopoDS_Face face = TopoDS::Face(faceMap(index + 1));

        // Compute surface properties (area and centroid)
        GProp_GProps gprops;
        BRepGProp::SurfaceProperties(face, gprops);

        gp_Pnt centroid = gprops.CentreOfMass();
        out7[0] = gprops.Mass();  // Area for surfaces
        out7[1] = centroid.X();
        out7[2] = centroid.Y();
        out7[3] = centroid.Z();

        // Compute normal at centroid using surface geometry
        double u1, u2, v1, v2;
        BRepTools::UVBounds(face, u1, u2, v1, v2);

        double uMid = (u1 + u2) / 2.0;
        double vMid = (v1 + v2) / 2.0;

        // Create property evaluator at the midpoint
        Handle(Geom_Surface) geomSurf = BRep_Tool::Surface(face);
        if (geomSurf.IsNull()) return -1;

        GeomLProp_SLProps props(geomSurf, 1, 1e-7);
        props.SetParameters(uMid, vMid);

        // Compute normal from first derivatives
        const gp_Vec& d1u = props.D1U();
        const gp_Vec& d1v = props.D1V();
        gp_Vec normal = d1u.Crossed(d1v);

        if (normal.Magnitude() < 1e-7) {
            return -1;  // Cannot compute normal
        }

        if (face.Orientation() == TopAbs_REVERSED) {
            normal.Reverse();
        }

        normal.Normalize();
        out7[4] = normal.X();
        out7[5] = normal.Y();
        out7[6] = normal.Z();

        return 0;
    } catch (...) {
        return -1;
    }
}

// STEP export/import functions
int mk_export_step(const mk_shape* s, const char* path) {
    try {
        if (!s || !s->shape || !path) return -1;
        STEPControl_Writer w;
        if (w.Transfer(*s->shape, STEPControl_AsIs) != IFSelect_RetDone) return -1;
        if (w.Write(path) != IFSelect_RetDone) return -1;
        return 0;
    } catch (...) {
        return -1;
    }
}

int mk_export_step_schema(const mk_shape* s, const char* path, const char* schema) {
    try {
        if (!s || !s->shape || !path || !schema) return -1;
        STEPControl_Writer w;  // registers the STEP parameters (Interface_Static) on construction
        if (!Interface_Static::SetCVal("write.step.schema", schema)) return -1;
        Interface_Static::SetCVal("write.step.unit", "MM");
        // Without this the product name carries a per-process export counter ("... 8.0 2", "... 8.0 6").
        Interface_Static::SetCVal("write.step.product.name", "mkcad-part");
        if (w.Transfer(*s->shape, STEPControl_AsIs) != IFSelect_RetDone) return -1;
        if (w.Write(path) != IFSelect_RetDone) return -1;
        // OCCT appends a per-process export counter to the product name ("mkcad-part 3"); strip it so that the
        // DATA section depends on the shape only (K-4).
        std::string text;
        {
            std::ifstream in(path, std::ios::binary);
            if (!in) return -1;
            text.assign(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
        }
        text = std::regex_replace(text, std::regex("'mkcad-part [0-9]+'"), "'mkcad-part'");
        std::ofstream out(path, std::ios::binary | std::ios::trunc);
        if (!out) return -1;
        out << text;
        return 0;
    } catch (...) {
        return -1;
    }
}

mk_shape* mk_import_step(const char* path) {
    try {
        if (!path) return nullptr;
        STEPControl_Reader rd;
        if (rd.ReadFile(path) != IFSelect_RetDone) return nullptr;
        if (rd.TransferRoots() <= 0) return nullptr;
        mk_shape* result = new mk_shape;
        result->shape = new TopoDS_Shape(rd.OneShape());
        return result;
    } catch (...) {
        return nullptr;
    }
}

} // extern "C"
