# /home/emporas/repos/freecad/rust/a7/py_macro/a7.py
# exec(open("/home/emporas/repos/freecad/rust/a7/py_macro/a7.py").read())
import math
import pathlib
import shutil
import time
import importlib.util

import FreeCAD
import Part

# ═══════════════════════════════════════════════════════════════════════════════
# § 0  Module hot-reload
# ═══════════════════════════════════════════════════════════════════════════════

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")

def _load_fresh():
    src = next((_SP / "a7").glob("a7.cpython-*.so"))
    for old in (_SP / "a7").glob("_sm_*.so"):
        old.unlink(missing_ok=True)
    tmp  = _SP / "a7" / f"_sm_{int(time.time())}.so"
    shutil.copy(src, tmp)
    spec = importlib.util.spec_from_file_location("a7", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

sm = _load_fresh()

import a7 as sm

# ═══════════════════════════════════════════════════════════════════════════════
# § 1  Low-level FreeCAD primitive helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _safe_name(label: str) -> str:
    """Return a FreeCAD-safe internal name (no dots, no spaces)."""
    return label.replace(".", "_").replace(" ", "_")


def _box(doc, label, x, y, z, length, width, height, color=(0.70, 0.70, 0.70)):
    obj = doc.addObject("Part::Box", _safe_name(label))
    obj.Label  = label
    obj.Length = length
    obj.Width  = width
    obj.Height = height
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(x, y, z),
        FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj


def _cylinder(doc, label, x, y, z, radius, height,
              color=(0.55, 0.55, 0.55), rotation=None):
    obj = doc.addObject("Part::Cylinder", _safe_name(label))
    obj.Label  = label
    obj.Radius = radius
    obj.Height = height
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(x, y, z),
        rotation if rotation is not None else FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj


def _torus(doc, label, x, y, z, r_major, r_minor,
           color=(0.20, 0.20, 0.20), rotation=None):
    obj = doc.addObject("Part::Torus", _safe_name(label))
    obj.Label   = label
    obj.Radius1 = r_major   # major (ring) radius
    obj.Radius2 = r_minor   # minor (tube) radius
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(x, y, z),
        rotation if rotation is not None else FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj

# ═══════════════════════════════════════════════════════════════════════════════
# § 2  Group helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _find_or_create_group(doc, label: str, parent=None):
    existing = [o for o in doc.Objects if o.Label == label]
    if existing:
        return existing[0]
    g = doc.addObject("App::DocumentObjectGroup", _safe_name(label))
    g.Label = label
    if parent is not None:
        parent.addObject(g)
    return g


def _grp(doc, label, parent=None):
    return _find_or_create_group(doc, label, parent)

# ═══════════════════════════════════════════════════════════════════════════════
# § 3  Part builders  (Spec → FreeCAD objects)
# ═══════════════════════════════════════════════════════════════════════════════

def _make_chassis_part(doc, spec):
    """Axle beams, rails and cross-members → Part::Box."""
    color = {
        "axle_beam":    (0.50, 0.50, 0.52),
        "rail":         (0.45, 0.45, 0.48),
        "cross_member": (0.55, 0.55, 0.58),
    }.get(spec.part_type, (0.50, 0.50, 0.50))
    return _box(doc, spec.label,
                spec.x, spec.y, spec.z,
                spec.length, spec.width, spec.height,
                color)


def _make_body_part(doc, spec):
    obj = _box(doc, spec.label,
               spec.x, spec.y, spec.z,
               spec.length, spec.width, spec.height,
               spec.color)
    # Glass-like transparency for the windshield
    obj.ViewObject.Transparency = 60 if spec.part_type == "windshield" else 10
    try:
        obj.addProperty(
            "App::PropertyFloat", "FilletRadius", "A7",
            "Design fillet radius (mm)",
        )
        obj.FilletRadius = spec.fillet_radius
    except Exception:
        pass
    return obj

# ── Rotation helpers used by _make_wheel and _make_mechanical ─────────────────

def _rot_x90():
    """Rotate 90° around X: aligns default-Z cylinder/torus axis with Y."""
    return FreeCAD.Rotation(FreeCAD.Vector(1, 0, 0), 90)


def _rot_lean(angle_deg: float):
    """Lean `angle_deg` from Z toward −X (rotation around −Y).

    Used for the steering column and steering wheel so both share the same
    tilt angle.  At angle_deg = 0 this is the identity rotation.
    """
    if abs(angle_deg) < 1e-9:
        return FreeCAD.Rotation()
    return FreeCAD.Rotation(FreeCAD.Vector(0, -1, 0), angle_deg)

def _make_wheel(doc, spec, parent_grp):
    """
    Build one wheel assembly: tyre torus + hub cylinder + wire spokes.

    Coordinate convention
    ─────────────────────
    The axle runs along Y.  The wheel rolls in the XZ plane.  The default
    Part::Torus and Part::Cylinder axes are along Z, so we rotate 90° around X
    to put them on the Y axis.
    """
    grp = _grp(doc, spec.label, parent_grp)

    # ── Tyre ──────────────────────────────────────────────────────────────────
    # The tyre ring radius is outer_radius minus one tire_section so the outer
    # surface of the torus reaches outer_radius.
    tyre = _torus(
        doc, f"{spec.label}_Tyre",
        spec.cx, spec.cy, spec.cz,
        spec.outer_radius - spec.tire_section, spec.tire_section,
        (0.15, 0.15, 0.15), _rot_x90(),
    )
    grp.addObject(tyre)

    # ── Hub ───────────────────────────────────────────────────────────────────
    # Cylinder along Y: base placed at cy - hub_half, height = hub_width.
    hub_w = spec.rim_width + 40.0
    hub = _cylinder(
        doc, f"{spec.label}_Hub",
        spec.cx - spec.hub_radius,          # cylinder base is a circle in XZ
        spec.cy - hub_w / 2.0,             # Y offset so hub is centred at cy
        spec.cz - spec.hub_radius,
        spec.hub_radius, hub_w,
        (0.65, 0.65, 0.65), _rot_x90(),
    )
    grp.addObject(hub)

    # ── Spokes ────────────────────────────────────────────────────────────────
    # Each spoke is a thin cylinder radiating from the hub perimeter to the rim
    # in the XZ plane at y = cy.
    spoke_r   = 4.0
    spoke_len = (spec.outer_radius - spec.tire_section) - spec.hub_radius

    for i in range(spec.spoke_count):
        theta = 2.0 * math.pi * i / spec.spoke_count
        dx    = math.sin(theta)
        dz    = math.cos(theta)

        # Base of the spoke at the hub perimeter
        base_x = spec.cx + spec.hub_radius * dx
        base_z = spec.cz + spec.hub_radius * dz

        # Rotation: tilt the default-Z cylinder axis to point radially (dx, 0, dz).
        # Cross product Z × d = (0,0,1) × (dx, 0, dz) = (0, dx, 0).
        # Normalised rotation axis = (0, 1, 0) (or its negative for dx < 0).
        # Angle = acos(dz) = theta.  We always use (0, -1, 0) and negate theta
        # to get the same result without the sign branch.
        rot = FreeCAD.Rotation(FreeCAD.Vector(0, -1, 0), math.degrees(theta))

        spoke = _cylinder(
            doc, f"{spec.label}_Spoke{i + 1:02d}",
            base_x, spec.cy - spoke_r, base_z,
            spoke_r, spoke_len,
            (0.75, 0.70, 0.55), rot,
        )
        grp.addObject(spoke)

    return grp

def _make_mechanical_part(doc, spec):
    """
    Dispatch on spec.part_type and create the appropriate FreeCAD primitive.

    Dimension conventions (see types.rs § 2 doc comment):
      engine_block / engine_fin / seat → Box  (length, width, height)
      spark_plug                       → Cylinder (length = radius, height = h)
      steering_column                  → Cylinder (length = h, width = radius,
                                                   angle_deg from Z toward -X)
      steering_wheel                   → Torus (height = major r, width = minor r,
                                                same tilt as column)
    """
    t = spec.part_type

    if t in ("engine_block", "engine_fin", "seat"):
        return _box(doc, spec.label,
                    spec.x, spec.y, spec.z,
                    spec.length, spec.width, spec.height,
                    spec.color)

    if t == "spark_plug":
        return _cylinder(doc, spec.label,
                         spec.x, spec.y, spec.z,
                         spec.length,   # radius
                         spec.height,   # cylinder height
                         spec.color)

    if t == "steering_column":
        # Cylinder angled `angle_deg` from Z toward -X.
        rot = _rot_lean(spec.angle_deg)
        return _cylinder(doc, spec.label,
                         spec.x, spec.y, spec.z,
                         spec.width,    # radius
                         spec.length,   # height along column axis
                         spec.color, rot)

    if t == "steering_wheel":
        # Torus tilted the same way as the column so it sits perpendicular to it.
        rot = _rot_lean(spec.angle_deg)
        return _torus(doc, spec.label,
                      spec.x, spec.y, spec.z,
                      spec.height,   # major radius
                      spec.width,    # minor (tube) radius
                      spec.color, rot)

    # Fallback: render unknown types as a labelled box so nothing is silently dropped.
    print(f"[a7_macro] Unknown part_type '{t}' for '{spec.label}' — rendering as box.")
    return _box(doc, spec.label,
                spec.x, spec.y, spec.z,
                spec.length, spec.width, spec.height,
                spec.color)

# ═══════════════════════════════════════════════════════════════════════════════
# § 4  Scene builder
# ═══════════════════════════════════════════════════════════════════════════════

def build_a7(params=None):
    """
    Build or rebuild the complete Austin Seven scene.

    Every call wipes the document and rebuilds from scratch — there is no
    incremental diff because there is no backing database.

    Parameters
    ----------
    params : a7.A7Params, optional
        Override the default geometry parameters.  Pass ``sm.A7Params(...)``
        with keyword arguments to change dimensions.
    """
    doc = FreeCAD.ActiveDocument or FreeCAD.newDocument("A7")

    # Full rebuild: clear all existing objects.
    for obj in list(doc.Objects):
        doc.removeObject(obj.Name)

    print("[a7_macro] Building Austin Seven scene...")

    specs = sm.build_all_specs(params)

    # ── Top-level groups ──────────────────────────────────────────────────────
    grp_chassis = _grp(doc, "Chassis")
    grp_body    = _grp(doc, "Bodywork")
    grp_wheels  = _grp(doc, "Wheels")
    grp_mech    = _grp(doc, "Mechanical")

    # ── Chassis ───────────────────────────────────────────────────────────────
    for spec in specs.chassis_parts:
        grp_chassis.addObject(_make_chassis_part(doc, spec))

    # ── Bodywork ──────────────────────────────────────────────────────────────
    for spec in specs.body_parts:
        grp_body.addObject(_make_body_part(doc, spec))

    # ── Wheels ────────────────────────────────────────────────────────────────
    for spec in specs.wheels:
        _make_wheel(doc, spec, grp_wheels)

    # ── Mechanical ────────────────────────────────────────────────────────────
    for spec in specs.mechanical_parts:
        grp_mech.addObject(_make_mechanical_part(doc, spec))

    doc.recompute()

    n = specs.n_total
    print(f"[a7_macro] Done — {n} objects created.")
    print()
    print("  build_a7()               – full rebuild with default params")
    print("  build_a7(sm.A7Params())  – full rebuild with custom params")
    print("  sm.A7Params(compute_engine=False)  – chassis + body only")

    return doc

# ═══════════════════════════════════════════════════════════════════════════════
# § 5  Run on load
# ═══════════════════════════════════════════════════════════════════════════════

build_a7()
