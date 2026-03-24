# pump_macro/pump.py
# exec(open("/home/emporas/repos/freecad/rust/pump/py_macro/pump.py").read())
import pathlib
import shutil
import time
import importlib.util

import FreeCAD
import Part

# ═══════════════════════════════════════════════════════════════════════════════
# § 0  Module hot-reload  (identical dance to shard.py)
# ═══════════════════════════════════════════════════════════════════════════════
#
# FreeCAD keeps a file handle open on any .so it has imported.  Recompiling
# the crate would therefore fail on Linux (EBUSY) and silently use the old
# binary on macOS.  The fix: copy the canonical .so to a fresh timestamped
# shadow file before each import, then import from the shadow.  Old shadows
# from previous loads are cleaned up first.

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")


def _load_fresh():
    # Find the compiled extension (e.g. pump.cpython-312-x86_64-linux-gnu.so)
    src = next((_SP / "pump").glob("pump.cpython-*.so"))

    # Remove previous shadow copies
    for old in (_SP / "pump").glob("_pm_*.so"):
        old.unlink(missing_ok=True)

    # Create a fresh timestamped shadow
    tmp = _SP / "pump" / f"_pm_{int(time.time())}.so"
    shutil.copy(src, tmp)

    # Import the shadow as module "pump"
    spec = importlib.util.spec_from_file_location("pump", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


pm = _load_fresh()

import pump as pm   # noqa: F811  — re-import for IDE type hints


# ═══════════════════════════════════════════════════════════════════════════════
# § 1  Low-level FreeCAD primitive helper
# ═══════════════════════════════════════════════════════════════════════════════

def _safe_name(label: str) -> str:
    return label.replace(".", "_").replace(" ", "_").replace("/", "_")


def _cylinder(doc, spec):
    """Create a Part::Cylinder from a CylinderSpec.

    FreeCAD Part::Cylinder parameters
    ──────────────────────────────────
      Radius   — cylinder radius
      Height   — axial extent along the cylinder's local +Z
      Placement — position of the **base face centre**

    The spec's (x, y, z) is already the base-face centre (geometry.rs § 3).
    """
    obj = doc.addObject("Part::Cylinder", _safe_name(spec.label))
    obj.Label  = spec.label
    obj.Radius = spec.radius
    obj.Height = spec.height
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(spec.x, spec.y, spec.z),
        FreeCAD.Rotation(),      # no rotation — all parts are coaxial with Z
    )
    obj.ViewObject.ShapeColor    = spec.color
    obj.ViewObject.Transparency  = int(spec.transparency)
    return obj


# ═══════════════════════════════════════════════════════════════════════════════
# § 2  Group helpers  (identical API to shard.py)
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
# § 3  Part-type → group dispatcher
# ═══════════════════════════════════════════════════════════════════════════════

# Maps every part_type string to the name of the FreeCAD group it belongs in.
_GROUP_MAP = {
    "barrel_wall":    "Barrel",
    "barrel_bore":    "Barrel",
    "cap_left":       "Caps",
    "cap_right":      "Caps",
    "cap_bore_left":  "Caps",
    "cap_bore_right": "Caps",
    "rod":            "Rod",
    "piston":         "Piston",
    "thumb_press":    "Terminals",
    "nozzle_body":    "Terminals",
    "nozzle_tip":     "Terminals",
    "nozzle_flange":  "Terminals",
}


# ═══════════════════════════════════════════════════════════════════════════════
# § 4  Scene builder
# ═══════════════════════════════════════════════════════════════════════════════

def build_pump(params=None):
    """Build or rebuild the complete coaxial syringe pump assembly.

    Every call wipes the active document and rebuilds from scratch.

    Assembly layout (Z-axis, origin = barrel centre)
    ────────────────────────────────────────────────
      −Z  ←  [flange][tip][nozzle body] | left cap | barrel | right cap | [thumb press]  → +Z
                                                                                 ↑
                                                          rod spans entire length + overhangs

    The barrel and bore are rendered with Transparency set so the internal rod
    and piston are visible in the FreeCAD viewport without boolean operations.

    Parameters
    ──────────
    params : pump.PumpParams, optional
        Override any default dimension.  Examples::

            build_pump()
            build_pump(pm.PumpParams(stroke=80.0))
            build_pump(pm.PumpParams(barrel_length=300.0, show_bore=False))
            build_pump(pm.PumpParams(show_piston=False, show_nozzle=False))
    """
    doc = FreeCAD.ActiveDocument or FreeCAD.newDocument("Pump")

    for obj in list(doc.Objects):
        try:
            doc.removeObject(obj.Name)
        except Exception:
            pass

    print("[pump_macro] Building coaxial syringe pump assembly...")

    specs = pm.build_all_specs(params)

    # ── Print clearance report ────────────────────────────────────────────────
    cr = specs.clearances
    print(f"  Piston radial gap : {cr.piston_radial_gap:.3f} mm")
    print(f"  Rod bore gap      : {cr.rod_bore_gap:.3f} mm")
    print(f"  Piston in barrel  : {'OK' if cr.piston_in_barrel and cr.piston_top_in_barrel else 'FAIL'}")
    print(f"  Total axial length: {cr.total_axial_length:.1f} mm")
    for w in cr.warnings:
        print(f"  ⚠  {w}")

    # ── Create top-level groups ───────────────────────────────────────────────
    grp_barrel    = _grp(doc, "Barrel")
    grp_caps      = _grp(doc, "Caps")
    grp_rod       = _grp(doc, "Rod")
    grp_piston    = _grp(doc, "Piston")
    grp_terminals = _grp(doc, "Terminals")   # thumb press + nozzle assembly

    group_map = {
        "Barrel":    grp_barrel,
        "Caps":      grp_caps,
        "Rod":       grp_rod,
        "Piston":    grp_piston,
        "Terminals": grp_terminals,
    }

    # ── Instantiate all Part::Cylinder primitives ─────────────────────────────
    for spec in specs.cylinders:
        obj        = _cylinder(doc, spec)
        group_name = _GROUP_MAP.get(spec.part_type, "Barrel")
        group_map[group_name].addObject(obj)

    doc.recompute()

    n = specs.n_total
    print(f"[pump_macro] Done — {n} Part::Cylinder objects created.")
    print()
    print("  build_pump()                              – full rebuild, defaults")
    print("  build_pump(pm.PumpParams(...))            – custom params")
    print()
    print("  Quick recipes:")
    print("    pm.PumpParams(stroke=80.0)              – piston near top of barrel")
    print("    pm.PumpParams(stroke=5.0)               – piston near bottom")
    print("    pm.PumpParams(barrel_length=300.0)      – longer barrel")
    print("    pm.PumpParams(show_bore=False)          – hide bore ghost")
    print("    pm.PumpParams(show_piston=False)        – hide piston (empty barrel)")
    print("    pm.PumpParams(show_nozzle=False)        – stub ends only")

    return doc


# ═══════════════════════════════════════════════════════════════════════════════
# § 5  Run on load
# ═══════════════════════════════════════════════════════════════════════════════

build_pump()
