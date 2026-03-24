# /home/emporas/repos/freecad/rust/roy/py_macro/roy.py
# exec(open("/home/emporas/repos/freecad/rust/roy/py_macro/roy.py").read())
import os
import pathlib
import shutil
import time
import importlib.util

import FreeCAD
import Part
import Draft
import Arch

# ═══════════════════════════════════════════════════════════════════════════════
# § 0  Module hot-reload
# ═══════════════════════════════════════════════════════════════════════════════

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")

def _load_fresh():
    src = next((_SP / "roy").glob("roy.cpython-*.so"))
    for old in (_SP / "roy").glob("_sm_*.so"):
        old.unlink(missing_ok=True)
    tmp  = _SP / "roy" / f"_sm_{int(time.time())}.so"
    shutil.copy(src, tmp)
    spec = importlib.util.spec_from_file_location("roy", tmp)
    mod  = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


sm = _load_fresh()

# ═══════════════════════════════════════════════════════════════════════════════
# § 1  Module-level scene state
# ═══════════════════════════════════════════════════════════════════════════════

# Four separate per-entity fingerprint caches.  Keeping them separate is
# critical: a single combined map caused every key from *other* tables to
# appear as a deletion in each get_* call, wiping the entire building on the
# second exec().
#
# Each cache is persisted across exec() re-runs via globals().  We only treat
# them as valid when *all four* are present together with a non-None _sync_at;
# if any piece is missing the whole set is reset so the next run does a clean
# full diff rather than an inconsistent incremental one.

def _load_fp_state():
    slabs   = globals().get("_fp_slabs",   None)
    walls   = globals().get("_fp_walls",   None)
    shelves = globals().get("_fp_shelves", None)
    items   = globals().get("_fp_items",   None)
    sync_at = globals().get("_sync_at",    None)
    # All five must be present and non-None for the state to be valid.
    if slabs is not None and walls is not None and \
       shelves is not None and items is not None and sync_at is not None:
        return slabs, walls, shelves, items, sync_at
    return {}, {}, {}, {}, None

_fp_slabs, _fp_walls, _fp_shelves, _fp_items, _sync_at = _load_fp_state()

# ═══════════════════════════════════════════════════════════════════════════════
# § 2  Spec → FreeCAD object builders
# ═══════════════════════════════════════════════════════════════════════════════

def _make_slab(doc, spec):
    obj = Arch.makeStructure(
        None,
        length = spec.length,
        width  = spec.width,
        height = spec.thickness,
        name   = spec.label,
    )
    obj.Label   = spec.label
    obj.IfcType = "Slab"
    obj.Placement = FreeCAD.Placement(
        FreeCAD.Vector(
            spec.x + spec.length / 2,
            spec.y + spec.width  / 2,
            spec.z,
        ),
        FreeCAD.Rotation(),
    )
    obj.ViewObject.ShapeColor   = (0.85, 0.84, 0.80)
    obj.ViewObject.Transparency = 20
    return obj


def _make_wall(doc, spec):
    line = Draft.make_line(
        FreeCAD.Vector(spec.x1, spec.y1, spec.z),
        FreeCAD.Vector(spec.x2, spec.y2, spec.z),
    )
    line.Label      = f"{spec.label}_baseline"
    line.Visibility = False
    doc.recompute()
    wall = Arch.makeWall(
        line,
        width  = spec.width,
        height = spec.height,
        align  = spec.align,
        name   = spec.label,
    )
    wall.Label = spec.label
    wall.ViewObject.ShapeColor = (0.85, 0.80, 0.72)
    return wall


def _make_shelf(doc, spec):
    safe = spec.label.replace(".", "_")
    box  = doc.addObject("Part::Box", safe)
    box.Label  = spec.label
    box.Length = spec.sx
    box.Width  = spec.sy
    box.Height = spec.sz
    box.Placement = FreeCAD.Placement(
        FreeCAD.Vector(spec.x, spec.y, spec.z),
        FreeCAD.Rotation(),
    )
    r, g, b = spec.color
    box.ViewObject.ShapeColor   = (r, g, b)
    box.ViewObject.Transparency = 10
    return box


def _item_text_color(item):
    """
    Return the FreeCAD text colour for a shelf-item label.

    White (1, 1, 1) is the "no highlight" sentinel — fall back to dark grey
    so the label is still readable against the shelf background.
    Any other colour stored in the DB is used directly as the text colour,
    giving warehouse operators a clear visual signal.
    """
    r, g, b = item.color
    if r == 1.0 and g == 1.0 and b == 1.0:
        return (0.05, 0.05, 0.05)   # default: dark grey, no highlight
    return (r, g, b)


def _apply_item_color_to_shelf(doc, item):
    """
    Push the item's highlight colour onto the parent shelf Part::Box.

    The shelf box is the only always-visible object that represents an
    inventory slot — the Draft text labels are hidden by default.

    White (1, 1, 1) is the "no highlight" sentinel: restore a neutral
    shelf colour and transparency so the box doesn't stay tinted after a
    colour reset (--white).

    The lookup uses item.shelf_label directly against obj.Label because
    _make_shelf() sets box.Label = spec.label (the human-readable dotted
    name), which is different from obj.Name (the safe underscored internal
    FreeCAD identifier).
    """
    r, g, b = item.color
    shelf_objs = [o for o in doc.Objects if o.Label == item.shelf_label]
    if not shelf_objs:
        return
    box = shelf_objs[0]
    if (r, g, b) == (1.0, 1.0, 1.0):
        # Reset to the neutral shelf colour written by _make_shelf().
        # We don't know the original role colour here, so use a safe
        # mid-grey that is clearly "no highlight".
        box.ViewObject.ShapeColor   = (0.70, 0.70, 0.70)
        box.ViewObject.Transparency = 10
    else:
        box.ViewObject.ShapeColor   = (r, g, b)
        box.ViewObject.Transparency = 0


# Built once, reused for every dispatch lookup.
_DISPATCH: dict | None = None


def _get_dispatch() -> dict:
    global _DISPATCH
    if _DISPATCH is None:
        _DISPATCH = {
            sm.SlabSpec:  _make_slab,
            sm.WallSpec:  _make_wall,
            sm.ShelfSpec: _make_shelf,
        }
    return _DISPATCH

# ═══════════════════════════════════════════════════════════════════════════════
# § 3  Group-management helpers  (store-aware)
# ═══════════════════════════════════════════════════════════════════════════════

_TAG_GROUP = "ItemPriceTags"


def _find_or_create_group(doc, label: str, parent=None):
    existing = [o for o in doc.Objects if o.Label == label]
    if existing:
        return existing[0]
    g = doc.addObject("App::DocumentObjectGroup", label)
    g.Label = label
    if parent is not None:
        parent.addObject(g)
    return g


def _ensure_store_group(doc, store: str):
    return _find_or_create_group(doc, f"Store_{store}")


def _ensure_store_struct_group(doc, store: str):
    parent = _ensure_store_group(doc, store)
    return _find_or_create_group(doc, f"{store}_Structure", parent)


def _ensure_store_quad_group(doc, store: str, quad: str):
    parent = _ensure_store_group(doc, store)
    return _find_or_create_group(doc, f"{store}_Quad_{quad}", parent)


def _ensure_store_row_group(doc, store: str, quad: str, row: int):
    parent = _ensure_store_quad_group(doc, store, quad)
    return _find_or_create_group(doc, f"{store}_{quad}_Row{row:02d}", parent)


def _ensure_tag_group(doc):
    return _find_or_create_group(doc, _TAG_GROUP)


def _remove_by_label(doc, label: str):
    for obj in list(doc.Objects):
        if obj.Label in (label, f"{label}_baseline"):
            doc.removeObject(obj.Name)


def _place_building_obj(doc, spec, fc_obj):
    if isinstance(spec, sm.ShelfSpec):
        _ensure_store_row_group(doc, spec.store, spec.quadrant, spec.row).addObject(fc_obj)
    else:
        _ensure_store_struct_group(doc, spec.store).addObject(fc_obj)

# ═══════════════════════════════════════════════════════════════════════════════
# § 4  Apply a DiffResult to the live FreeCAD document
# ═══════════════════════════════════════════════════════════════════════════════

def _apply_diff(doc, diff) -> None:
    dispatch = _get_dispatch()
    tag_grp  = _ensure_tag_group(doc)

    # ── Building: removals first ──────────────────────────────────────────────
    for label in diff.building_remove:
        _remove_by_label(doc, label)

    # ── Building: additions ───────────────────────────────────────────────────
    for spec in diff.building_add:
        fc_obj = dispatch[type(spec)](doc, spec)
        _place_building_obj(doc, spec, fc_obj)

    # ── Building: updates (delete old → create replacement) ───────────────────
    for spec in diff.building_update:
        _remove_by_label(doc, spec.label)
        fc_obj = dispatch[type(spec)](doc, spec)
        _place_building_obj(doc, spec, fc_obj)

    # ── Items: removals ───────────────────────────────────────────────────────
    for tag_label in diff.item_remove:
        _remove_by_label(doc, tag_label)

    # ── Items: additions ──────────────────────────────────────────────────────
    for item in diff.item_add:
        tag_grp.addObject(_make_item_label(doc, item))
        # Propagate any non-white highlight colour to the shelf box so that
        # items inserted with a colour flag are immediately visible.
        if item.color != (1.0, 1.0, 1.0):
            _apply_item_color_to_shelf(doc, item)

    # ── Items: in-place text + colour patch ───────────────────────────────────
    for item in diff.item_update:
        tag_lbl = "Tag_" + item.coord_label.replace(".", "_")
        objs    = [o for o in doc.Objects if o.Label == tag_lbl]
        if objs:
            obj      = objs[0]
            lines    = list(obj.Text)
            lines[1] = f"\u20ac{item.price:.2f}"
            lines[2] = f"Qty: {item.quantity}"
            obj.Text = lines

            obj.ViewObject.TextColor = _item_text_color(item)

            new_pos = FreeCAD.Vector(item.world_x, item.world_y, item.world_z)
            if obj.Placement.Base != new_pos:
                obj.Placement = FreeCAD.Placement(
                    new_pos,
                    FreeCAD.Rotation(FreeCAD.Vector(1, 0, 0), 90),
                )
        else:
            # Tag was missing from the scene — recreate it.
            tag_grp.addObject(_make_item_label(doc, item))

        # Always push the new colour (or reset) onto the visible shelf box.
        # This is the primary fix: the Draft text label is hidden by default,
        # so the box is the only thing the operator actually sees.
        _apply_item_color_to_shelf(doc, item)

# ═══════════════════════════════════════════════════════════════════════════════
# § 5  Public entry points
# ═══════════════════════════════════════════════════════════════════════════════

def build_roy(*, force: bool = False, db_path: str | None = None):
    """
    Full or incremental scene build.

    Pass ``force=True`` to wipe the document and rebuild from scratch.
    On a normal re-exec the scene is updated incrementally.

    A hard reset is also forced whenever any of the four fingerprint caches or
    _sync_at is missing (e.g. one survived a namespace refresh and another did
    not), because an incremental diff against a partial cache can produce
    spurious additions or deletions in the FreeCAD scene.
    """
    global _fp_slabs, _fp_walls, _fp_shelves, _fp_items, _sync_at
    doc = FreeCAD.ActiveDocument

    # Force a hard reset if:
    #   • caller asked for it explicitly, OR
    #   • there is no active document, OR
    #   • any fingerprint cache is empty (first run), OR
    #   • _sync_at is None (inconsistent state — safer to rebuild from scratch).
    any_fp_empty = not (_fp_slabs and _fp_walls and _fp_shelves and _fp_items)
    need_hard_reset = (
        force
        or doc is None
        or any_fp_empty
        or _sync_at is None
    )

    if need_hard_reset:
        doc = doc or FreeCAD.newDocument("Roy")
        for obj in doc.Objects:
            doc.removeObject(obj.Name)
        _fp_slabs   = {}
        _fp_walls   = {}
        _fp_shelves = {}
        _fp_items   = {}
        _sync_at    = None

    return _run_diff(doc, full=need_hard_reset, db_path=db_path)


def update_roy(*, db_path: str | None = None):
    """Alias for ``build_roy()`` — kept for backward compatibility."""
    return build_roy(db_path=db_path)


def _run_diff(doc, *, full: bool, db_path: str | None = None):
    global _fp_slabs, _fp_walls, _fp_shelves, _fp_items, _sync_at

    mode = "Full build" if full else "Incremental update"
    print(f"[roy_macro] {mode} — computing diff in Rust...")

    # On a full build pass empty maps and since=None so every row is fetched.
    # On an incremental pass hand back the four per-entity caches plus the
    # timestamp from the previous sync so the Rust layer only reads rows whose
    # updated_at >= that timestamp.
    since = None if full else _sync_at

    diff = sm.diff_all_specs(
        _fp_slabs,
        _fp_walls,
        _fp_shelves,
        _fp_items,
        db_path,
        since,
    )

    if not full and diff.n_add == 0 and diff.n_update == 0 and diff.n_remove == 0:
        print("[roy_macro] Scene is already up to date — nothing to do.")
        return doc

    _apply_diff(doc, diff)

    # Store each per-entity fingerprint map separately so they can be passed
    # back to the matching prev_* argument on the next call.
    _fp_slabs   = dict(diff.slab_fingerprints)
    _fp_walls   = dict(diff.wall_fingerprints)
    _fp_shelves = dict(diff.shelf_fingerprints)
    _fp_items   = dict(diff.item_fingerprints)
    _sync_at    = diff.sync_at   # timestamp recorded by Rust at query time

    doc.recompute()
    _print_summary(diff, full=full)
    hide_items()
    return doc


def _print_summary(diff, *, full: bool) -> None:
    mode = "Full build" if full else "Incremental update"
    print(f"[roy_macro] {mode} complete.")
    print(f"  Added    : {diff.n_add}")
    print(f"  Updated  : {diff.n_update}")
    print(f"  Removed  : {diff.n_remove}")
    print(f"  Unchanged: {diff.n_unchanged}")
    print()
    print("  show_items()          – reveal price/quantity labels")
    print("  hide_items()          – hide  price/quantity labels")
    print("  toggle_items()        – flip current label visibility")
    print("  update_roy(**kwargs)  – incremental rebuild")

# ═══════════════════════════════════════════════════════════════════════════════
# § 6  Label visibility helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _tag_group():
    doc    = FreeCAD.ActiveDocument
    groups = [o for o in doc.Objects if o.Label == _TAG_GROUP]
    return groups[0] if groups else None


def _set_items_visible(visible: bool) -> None:
    grp = _tag_group()
    if grp is None:
        print("[roy_macro] No item tags found — run build_roy() first.")
        return
    for obj in grp.OutList:
        obj.Visibility = visible
    grp.Visibility = visible
    FreeCAD.ActiveDocument.recompute()
    state = "shown" if visible else "hidden"
    print(f"[roy_macro] Price/quantity labels {state} ({len(grp.OutList)} labels).")


def show_items():
    _set_items_visible(True)


def hide_items():
    _set_items_visible(False)


def toggle_items():
    grp = _tag_group()
    if grp and grp.OutList:
        _set_items_visible(not grp.OutList[0].Visibility)
    else:
        print("[roy_macro] No item tags found — run build_roy() first.")


def _make_item_label(doc, item):
    lines = [
        item.coord_label,
        f"\u20ac{item.price:.2f}",
        f"Qty: {item.quantity}",
    ]
    txt = Draft.make_text(lines)
    txt.ViewObject.DisplayMode = "Screen"
    txt.Placement = FreeCAD.Placement(
        FreeCAD.Vector(item.world_x, item.world_y, item.world_z),
        FreeCAD.Rotation(FreeCAD.Vector(1, 0, 0), 90),
    )
    txt.Label                = "Tag_" + item.coord_label.replace(".", "_")
    txt.ViewObject.FontSize  = 80
    txt.ViewObject.TextColor = _item_text_color(item)
    return txt


def update_item(coord_label: str, price: float = None, quantity: int = None) -> None:
    """Patch the Draft text for a single shelf slot without a full rebuild."""
    doc     = FreeCAD.ActiveDocument
    tag_lbl = "Tag_" + coord_label.replace(".", "_")
    objs    = [o for o in doc.Objects if o.Label == tag_lbl]
    if not objs:
        print(f"[roy_macro] Label '{tag_lbl}' not found.")
        return
    obj   = objs[0]
    lines = list(obj.Text)
    if price    is not None: lines[1] = f"\u20ac{price:.2f}"
    if quantity is not None: lines[2] = f"Qty: {quantity}"
    obj.Text = lines
    doc.recompute()
    print(f"[roy_macro] Updated {coord_label}.")

# ═══════════════════════════════════════════════════════════════════════════════
# § 7  Run on load
# ═══════════════════════════════════════════════════════════════════════════════

build_roy()
