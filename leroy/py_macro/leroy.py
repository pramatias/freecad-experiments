# exec(open("/home/emporas/repos/freecad/rust/leroy/py_macro/leroy.py").read())
import importlib.util
import pathlib
import shutil
import time

import FreeCAD
import Part

# ═══════════════════════════════════════════════════════════════════════════
# § 0  Hot-reload  (same shadow-copy dance as pump.py)
# ═══════════════════════════════════════════════════════════════════════════

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")

def _load_fresh():
    pkg = _SP / "leroy"
    src = next(pkg.glob("leroy.cpython-*.so"))
    for old in pkg.glob("_lr_*.so"):
        old.unlink(missing_ok=True)
    tmp  = pkg / f"_lr_{int(time.time())}.so"
    shutil.copy(src, tmp)
    spec = importlib.util.spec_from_file_location("leroy", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

lr = _load_fresh()
import leroy as lr   # noqa: F811  — re-import for IDE hints

# ═══════════════════════════════════════════════════════════════════════════
# § 1  Font path  – adjust if needed
# ═══════════════════════════════════════════════════════════════════════════
# _FONT = str(pathlib.Path("/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf"))

# _FONT = str(pathlib.Path.home() / ".local/share/fonts/LiberationSansNarrow-BoldItalic.ttf")
# _FONT = str(pathlib.Path.home() / "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf")
# _FONT = str(pathlib.Path.home() / ".local/share/fonts/NotoSans-Bold.ttf")
_FONT = str(pathlib.Path.home() / ".local/share/fonts/DejaVuSans-Bold.ttf")

# ═══════════════════════════════════════════════════════════════════════════
# § 2  FreeCAD primitive builders
# ═══════════════════════════════════════════════════════════════════════════

def _safe(label: str) -> str:
    return label.replace(" ", "_").replace("/", "_").replace(".", "_")


def _color(c) -> tuple:
    """Accept list or tuple [r, g, b] → Python tuple."""
    return tuple(float(x) for x in c)


def _make_prism(doc, spec):
    """Extruded triangular prism from a FaceExtrudeSpec."""
    verts = [FreeCAD.Vector(v[0], v[1], spec.z_base) for v in spec.vertices_2d]
    verts.append(verts[0])                       # close the loop
    wire  = Part.makePolygon(verts)
    face  = Part.Face(wire)
    solid = face.extrude(FreeCAD.Vector(0.0, 0.0, spec.extrude_height))

    obj = doc.addObject("Part::Feature", _safe(spec.label))
    obj.Label = spec.label
    obj.Shape = solid
    obj.ViewObject.ShapeColor   = _color(spec.color)
    obj.ViewObject.Transparency = spec.transparency
    return obj


def _make_line(doc, spec):
    """Wire edge for a grid line."""
    p1   = FreeCAD.Vector(spec.x1, spec.y1, spec.z1)
    p2   = FreeCAD.Vector(spec.x2, spec.y2, spec.z2)
    edge = Part.makeLine(p1, p2)
    obj  = doc.addObject("Part::Feature", _safe(spec.label))
    obj.Label  = spec.label
    obj.Shape  = edge
    obj.ViewObject.LineColor = _color(spec.color)
    obj.ViewObject.LineWidth = 2.0
    return obj


def _anchor_local(bb, anchor: str):
    if anchor == "start_peak":
        return FreeCAD.Vector(bb.XMin, bb.YMax, 0.0)
    if anchor == "end_peak":
        return FreeCAD.Vector(bb.XMax, bb.YMax, 0.0)
    if anchor == "start_base":
        return FreeCAD.Vector(bb.XMin, bb.YMin, 0.0)
    if anchor == "end_base":
        return FreeCAD.Vector(bb.XMax, bb.YMin, 0.0)
    raise ValueError(f"Unknown text anchor: {anchor}")


def _make_text(doc, spec):
    """ShapeString + Part::Extrusion for relief lettering."""
    try:
        import Draft
    except ImportError:
        print(f"  [leroy] Draft workbench not available – skipping '{spec.text}'")
        return None

    # Create at origin first so we can inspect the real bounding box.
    ss = Draft.make_shapestring(
        String=spec.text,
        FontFile=_FONT,
        Size=spec.font_height,
    )
    doc.recompute()

    bb = ss.Shape.BoundBox
    rot = FreeCAD.Rotation(FreeCAD.Vector(0, 0, 1), spec.rotation_deg)

    # Pick the visible corner that should sit on the target point.
    local_anchor = _anchor_local(bb, spec.anchor)
    anchor_rotated = rot.multVec(local_anchor)

    target = FreeCAD.Vector(spec.x, spec.y, spec.z)
    base = target.sub(anchor_rotated)

    ss.Placement = FreeCAD.Placement(base, rot)
    doc.recompute()

    # Extrude the ShapeString into a solid
    extrude = doc.addObject("Part::Extrusion", _safe(spec.label))
    extrude.Label = spec.label
    extrude.Base = ss
    extrude.DirMode = "Custom"
    extrude.Dir = FreeCAD.Vector(0.0, 0.0, 1.0)
    extrude.LengthFwd = spec.extrude_depth
    extrude.Solid = True
    extrude.ViewObject.ShapeColor = _color(spec.color)

    ss.ViewObject.Visibility = False
    return extrude

def _make_hollow_shell(doc, spec):
    outer_pts = [FreeCAD.Vector(x, y, spec.z_back) for x, y in spec.outer_vertices_2d]
    outer_pts.append(outer_pts[0])
    inner_pts = [FreeCAD.Vector(x, y, spec.z_back) for x, y in spec.inner_vertices_2d]
    inner_pts.append(inner_pts[0])

    outer_face = Part.Face(Part.makePolygon(outer_pts))
    inner_face = Part.Face(Part.makePolygon(inner_pts))

    outer_solid = outer_face.extrude(FreeCAD.Vector(0.0, 0.0, spec.body_depth))
    inner_solid = inner_face.extrude(FreeCAD.Vector(0.0, 0.0, spec.pocket_depth))

    shell = outer_solid.cut(inner_solid)

    obj = doc.addObject("Part::Feature", _safe(spec.label))
    obj.Label = spec.label
    obj.Shape = shell
    obj.ViewObject.ShapeColor = _color(spec.color)
    obj.ViewObject.Transparency = spec.transparency
    return obj

# ═══════════════════════════════════════════════════════════════════════════
# § 3  Group helpers
# ═══════════════════════════════════════════════════════════════════════════

def _grp(doc, label, parent=None):
    existing = [o for o in doc.Objects if o.Label == label]
    if existing:
        return existing[0]
    g = doc.addObject("App::DocumentObjectGroup", _safe(label))
    g.Label = label
    if parent is not None:
        parent.addObject(g)
    return g

# ═══════════════════════════════════════════════════════════════════════════
# § 4  Scene builder
# ═══════════════════════════════════════════════════════════════════════════

def build_logo(
    letter_spacing=4.5,
    leroy_down_offset=0.0,
    merlin_down_offset=0.0,
):
    """Build or rebuild the Leroy Merlin logo assembly."""

    doc = FreeCAD.ActiveDocument or FreeCAD.newDocument("LeroyMerlin")

    for obj in list(doc.Objects):
        try:
            doc.removeObject(obj.Name)
        except Exception:
            pass

    print(
        "[leroy_macro] Computing geometry in Rust … "
        f"(letter_spacing={letter_spacing}, "
        f"leroy_down_offset={leroy_down_offset}, "
        f"merlin_down_offset={merlin_down_offset})"
    )

    logo_specs = lr.build_logo_specs(leroy_down_offset, merlin_down_offset)
    support_spec = lr.build_support_back_spec()

    grp_logo    = _grp(doc, "Logo")
    grp_support = _grp(doc, "Support")
    grp_grid    = _grp(doc, "Grid")
    grp_text    = _grp(doc, "Text")

    for spec in logo_specs.faces:
        obj = _make_prism(doc, spec)
        (grp_logo if spec.label == "Logo" else grp_support).addObject(obj)

    for spec in logo_specs.lines:
        grp_grid.addObject(_make_line(doc, spec))

    for spec in logo_specs.texts:
        obj = _make_text(doc, spec)
        if obj:
            grp_text.addObject(obj)

    support_obj = _make_hollow_shell(doc, support_spec)
    grp_support.addObject(support_obj)

    for rib_spec in support_spec.corner_ribs:
        rib_obj = _make_prism(doc, rib_spec)
        grp_support.addObject(rib_obj)

    center_square_obj = _make_prism(doc, support_spec.center_square)
    hole = support_spec.bolt_hole
    hole_cyl = Part.makeCylinder(
        hole.radius,
        hole.depth,
        FreeCAD.Vector(hole.x, hole.y, hole.z),
        FreeCAD.Vector(0.0, 0.0, 1.0),
    )
    center_square_obj.Shape = center_square_obj.Shape.cut(hole_cyl)
    grp_support.addObject(center_square_obj)

    doc.recompute()

    print(
        f"[leroy_macro] Done – "
        f"{len(logo_specs.faces)} prisms, "
        f"{len(logo_specs.lines)} grid edges, "
        f"{len(logo_specs.texts)} text letters, "
        f"1 support shell, "
        f"{len(support_spec.corner_ribs)} ribs."
    )

    return doc

# ═══════════════════════════════════════════════════════════════════════════
# § 5  Run on load
# ═══════════════════════════════════════════════════════════════════════════

build_logo(leroy_down_offset=1.2, merlin_down_offset=1.14)
