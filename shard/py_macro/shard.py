# /home/emporas/repos/freecad/rust/shard/py_macro/shard.py
# exec(open("/home/emporas/repos/freecad/rust/shard/py_macro/shard.py").read())
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
    src = next((_SP / "shard").glob("shard.cpython-*.so"))
    for old in (_SP / "shard").glob("_sm_*.so"):
        old.unlink(missing_ok=True)
    tmp  = _SP / "shard" / f"_sm_{int(time.time())}.so"
    shutil.copy(src, tmp)
    spec = importlib.util.spec_from_file_location("shard", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


sm = _load_fresh()

import shard as sm   # noqa: F811


# ═══════════════════════════════════════════════════════════════════════════════
# § 1  Low-level FreeCAD primitive helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _safe_name(label: str) -> str:
    return label.replace(".", "_").replace(" ", "_").replace("/", "_")


def _box(doc, label, x, y, z, length, width, height,
         color=(0.65, 0.68, 0.72), rotation=None):
    obj = doc.addObject("Part::Box", _safe_name(label))
    obj.Label  = label
    obj.Length = length
    obj.Width  = width
    obj.Height = height
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(x, y, z),
        rotation if rotation is not None else FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj


def _centered_box(doc, label, cx, cy, cz, length, width, height,
                  color=(0.65, 0.68, 0.72), rotation=None):
    """Part::Box centred at (cx, cy, cz); rotation applied around the centre."""
    rot = rotation if rotation is not None else FreeCAD.Rotation()
    pl  = FreeCAD.Placement(
        FreeCAD.Vector(cx - length / 2.0, cy - width / 2.0, cz - height / 2.0),
        rot,
    )
    obj = doc.addObject("Part::Box", _safe_name(label))
    obj.Label     = label
    obj.Length    = length
    obj.Width     = width
    obj.Height    = height
    obj.Placement = pl
    obj.ViewObject.ShapeColor = color
    return obj


def _cone(doc, label, x, y, z, base_r, tip_r, height,
          color=(0.55, 0.60, 0.67), rotation=None):
    obj = doc.addObject("Part::Cone", _safe_name(label))
    obj.Label   = label
    obj.Radius1 = base_r
    obj.Radius2 = tip_r
    obj.Height  = height
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(x, y, z),
        rotation if rotation is not None else FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj


def _ellipsoid(doc, label, cx, cy, cz, r_z, r_y, r_x,
               color=(0.62, 0.68, 0.75)):
    """Part::Ellipsoid centred at (cx, cy, cz) — kept for optional debug use."""
    obj = doc.addObject("Part::Ellipsoid", _safe_name(label))
    obj.Label   = label
    obj.Radius1 = r_z
    obj.Radius2 = r_y
    obj.Radius3 = r_x
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(cx, cy, cz),
        FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor = color
    return obj


# ═══════════════════════════════════════════════════════════════════════════════
# § 2  Rotation helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _rot_euler(rx_deg: float, ry_deg: float, rz_deg: float) -> "FreeCAD.Rotation":
    """Intrinsic ZYX Euler rotation: Z first, then Y, then X."""
    rx = FreeCAD.Rotation(FreeCAD.Vector(1, 0, 0), rx_deg) if abs(rx_deg) > 1e-9 else FreeCAD.Rotation()
    ry = FreeCAD.Rotation(FreeCAD.Vector(0, 1, 0), ry_deg) if abs(ry_deg) > 1e-9 else FreeCAD.Rotation()
    rz = FreeCAD.Rotation(FreeCAD.Vector(0, 0, 1), rz_deg) if abs(rz_deg) > 1e-9 else FreeCAD.Rotation()
    return rz.multiply(ry).multiply(rx)


def _rot_y(deg: float) -> "FreeCAD.Rotation":
    if abs(deg) < 1e-9:
        return FreeCAD.Rotation()
    return FreeCAD.Rotation(FreeCAD.Vector(0, 1, 0), deg)


def _rot_x(deg: float) -> "FreeCAD.Rotation":
    if abs(deg) < 1e-9:
        return FreeCAD.Rotation()
    return FreeCAD.Rotation(FreeCAD.Vector(1, 0, 0), deg)


# ═══════════════════════════════════════════════════════════════════════════════
# § 3  Group helpers
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
# § 4  Spec → FreeCAD dispatchers
# ═══════════════════════════════════════════════════════════════════════════════

def _make_shard(doc, spec):
    """Rotated centred Part::Box.

    Handles all ShardSpec part_types:
      frame_facet  — outer ring facet (tangentially oriented)
      frame_inner  — inner accent ring facet
      inner_blade  — horizontal cutting blade
      inner_shard  — random inner facet
    """
    rot = _rot_euler(spec.rot_x, spec.rot_y, spec.rot_z)
    return _centered_box(
        doc, spec.label,
        spec.x, spec.y, spec.z,
        spec.length, spec.width, spec.height,
        spec.color, rot,
    )


def _make_spike(doc, spec):
    """Pyramidal spike as Part::Cone.

    Primary and secondary spikes arrive as clusters: one main SpikeSpec plus
    several flanking SpikeSpec objects (all with the same part_type).
    Each is rendered as its own Cone; FreeCAD groups them under the same
    sub-group so they appear as a unified polyhedral shape.
    """
    rot = _rot_y(spec.rot_y)
    if abs(spec.rot_x) > 1e-9:
        rot = rot.multiply(_rot_x(spec.rot_x))

    return _cone(
        doc, spec.label,
        spec.x, spec.y, spec.z,
        spec.base_radius,
        spec.tip_radius,
        spec.length,
        spec.color,
        rot,
    )


def _make_foundation_part(doc, spec):
    """V-plate arm, cap bar or debris block as rotated Part::Box."""
    rot = _rot_euler(spec.rot_x, spec.rot_y, spec.rot_z)
    return _centered_box(
        doc, spec.label,
        spec.x, spec.y, spec.z,
        spec.length, spec.width, spec.height,
        spec.color, rot,
    )


# ═══════════════════════════════════════════════════════════════════════════════
# § 5  Scene builder
# ═══════════════════════════════════════════════════════════════════════════════

def build_shard(params=None):
    """Build or rebuild the complete shattered-crystal starburst scene.

    Every call wipes the active document and rebuilds from scratch.

    What changed in v2
    ------------------
    • Frame is now a ring of 36 + 24 faceted Part::Box segments (copper/teal
      colour-blocked) instead of a single ellipsoid boolean cut.
    • Core has 40 inner shards (up from 20), denser diamond facet rings.
    • Each primary/secondary spike is a polyhedral cluster of 3-5 cones.
    • Foundation V-plates are larger and properly horizontal; debris formation
      is wider and uses larger crystalline pieces with full 3-D rotation.

    Parameters
    ----------
    params : shard.ShardParams, optional
        Override default geometry.  Examples::

            build_shard(sm.ShardParams(primary_len=1800.0, inner_shards=60))
            build_shard(sm.ShardParams(compute_foundation=False))
    """
    doc = FreeCAD.ActiveDocument or FreeCAD.newDocument("Shard")

    for obj in list(doc.Objects):
        try:
            doc.removeObject(obj.Name)
        except Exception:
            pass

    print("[shard_macro] Building shattered crystal composition (v2)...")

    specs = sm.build_all_specs(params)

    # ── Top-level groups ──────────────────────────────────────────────────────
    grp_frame  = _grp(doc, "Frame")
    grp_core   = _grp(doc, "Core")
    grp_spikes = _grp(doc, "Spikes")
    grp_found  = _grp(doc, "Foundation")

    # Core sub-groups
    grp_diamond = _grp(doc, "Core_Diamond", grp_core)
    grp_blades  = _grp(doc, "Core_Blades",  grp_core)
    grp_shards  = _grp(doc, "Core_Shards",  grp_core)

    # Frame sub-groups (NEW: outer facets + inner accent ring)
    grp_frame_outer = _grp(doc, "Frame_Outer", grp_frame)
    grp_frame_inner = _grp(doc, "Frame_Inner", grp_frame)

    # Spike sub-groups
    grp_primary   = _grp(doc, "Spikes_Primary",   grp_spikes)
    grp_secondary = _grp(doc, "Spikes_Secondary", grp_spikes)
    grp_minor     = _grp(doc, "Spikes_Minor",     grp_spikes)

    # Foundation sub-groups
    grp_tiers  = _grp(doc, "Foundation_Tiers",  grp_found)
    grp_debris = _grp(doc, "Foundation_Debris", grp_found)

    # ── Frame (v2: ShardSpec objects, NOT EllipseFrameSpec) ───────────────────
    #
    # frame_specs now contains ShardSpec objects with part_type:
    #   "frame_facet"  → outer ring facets (36 segments, copper + teal)
    #   "frame_inner"  → inner accent ring (24 segments, copper)
    for spec in specs.frame_specs:
        obj = _make_shard(doc, spec)
        if spec.part_type == "frame_inner":
            grp_frame_inner.addObject(obj)
        else:
            grp_frame_outer.addObject(obj)

    # ── Spikes: diamond cones + polyhedral outer spike clusters ───────────────
    for spec in specs.spike_specs:
        obj = _make_spike(doc, spec)
        t = spec.part_type
        if t in ("diamond_up", "diamond_down"):
            grp_diamond.addObject(obj)
        elif t == "primary":
            grp_primary.addObject(obj)
        elif t == "secondary":
            grp_secondary.addObject(obj)
        else:
            grp_minor.addObject(obj)

    # ── Core shards: blades + inner facets ────────────────────────────────────
    for spec in specs.shard_specs:
        obj = _make_shard(doc, spec)
        if spec.part_type == "inner_blade":
            grp_blades.addObject(obj)
        else:
            grp_shards.addObject(obj)

    # ── Foundation: tiered V-plates + large debris formation ─────────────────
    for spec in specs.foundation_specs:
        obj = _make_foundation_part(doc, spec)
        if spec.part_type == "debris":
            grp_debris.addObject(obj)
        else:
            grp_tiers.addObject(obj)

    doc.recompute()

    n = specs.n_total
    print(f"[shard_macro] Done — {n} objects created.")
    print()
    print("  build_shard()                          – full rebuild, default params")
    print("  build_shard(sm.ShardParams(...))       – custom params")
    print()
    print("  Quick recipes:")
    print("    sm.ShardParams(compute_spikes=False)           – frame+core+foundation")
    print("    sm.ShardParams(compute_foundation=False)       – frame+core+spikes")
    print("    sm.ShardParams(inner_shards=60, blade_pairs=6) – ultra-dense core")
    print("    sm.ShardParams(primary_len=1800)               – longer corner spikes")

    return doc


# ═══════════════════════════════════════════════════════════════════════════════
# § 6  Run on load
# ═══════════════════════════════════════════════════════════════════════════════

build_shard()
