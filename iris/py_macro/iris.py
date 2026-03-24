#!/usr/bin/env python3
# exec(open("/home/emporas/repos/freecad/py_macros/iris.py").read())

import pathlib, shutil, time, importlib.util
import FreeCAD, Part

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")

def _load_fresh():
    src = next((_SP / "iris").glob("iris.cpython-*.so"))
    for old in (_SP / "iris").glob("_iris_*.so"):
        old.unlink(missing_ok=True)
    tmp = _SP / "iris" / f"_iris_{int(time.time())}.so"
    shutil.copy(src, tmp)
    spec = importlib.util.spec_from_file_location("iris", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

iris = _load_fresh()

def _clear_doc(doc):
    for obj in doc.Objects:
        doc.removeObject(obj.Name)

def _make_sphere(doc, spec):
    obj = doc.addObject("Part::Feature", spec.label)
    obj.Shape = Part.makeSphere(spec.radius)
    obj.Label = spec.label
    return obj

def _make_torus(doc, spec):
    obj = doc.addObject("Part::Feature", spec.label)
    obj.Shape = Part.makeTorus(spec.radius1, spec.radius2)
    obj.Label = spec.label
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(0, 0, spec.z),
        FreeCAD.Rotation(),
    )
    return obj

def _make_fin(doc, spec):
    # Box centred at local origin, rotated then translated by Placement.
    half_l, half_w = spec.length / 2, spec.width / 2
    box = Part.makeBox(
        spec.length, spec.width, spec.height,
        FreeCAD.Vector(-half_l, -half_w, 0),
    )
    obj = doc.addObject("Part::Feature", spec.label)
    obj.Shape = box
    obj.Label = spec.label
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(spec.cx, spec.cy, spec.z_base),
        FreeCAD.Rotation(FreeCAD.Vector(0, 0, 1), spec.angle_deg),
    )
    return obj

def _make_polygon_extrude(doc, spec):
    vecs = [FreeCAD.Vector(x, y, spec.z_base + z) for x, y, z in spec.pts]
    vecs.append(vecs[0])                    # close the polygon
    wire = Part.makePolygon(vecs)
    try:
        solid = Part.Face(wire).extrude(FreeCAD.Vector(0, 0, spec.height))
    except Exception as exc:
        print(f"[iris_macro] {spec.label}: extrude failed ({exc}), storing wire")
        solid = wire
    obj = doc.addObject("Part::Feature", spec.label)
    obj.Shape = solid
    obj.Label = spec.label
    return obj

_DISPATCH = None   # populated after iris is loaded

def build_iris(**kwargs):
    global _DISPATCH
    _DISPATCH = {
        iris.SphereSpec:         _make_sphere,
        iris.TorusSpec:          _make_torus,
        iris.FinSpec:            _make_fin,
        iris.PolygonExtrudeSpec: _make_polygon_extrude,
    }

    doc = FreeCAD.ActiveDocument or FreeCAD.newDocument("Iris")
    _clear_doc(doc)

    for spec in iris.build_specs(iris.IrisParams(**kwargs)):
        _DISPATCH[type(spec)](doc, spec)

    doc.recompute()
    print("[iris_macro] Done:", [o.Label for o in doc.Objects])

build_iris()
