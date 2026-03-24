// /home/emporas/repos/freecad/rust/roy/primitives/src/geometry.rs
use nalgebra::{Isometry3, Point3, Translation3, UnitQuaternion, Vector3};

use crate::price;
use crate::types::{RoyParams, ShelfItemSpec, ShelfSpec, SlabSpec, WallSpec};

// ═══════════════════════════════════════════════════════════════════════════════
// § 1  Role
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
pub enum Role { Fill, Refill, Wall }

impl Role {
    pub fn color(self) -> (f32, f32, f32) {
        match self {
            Self::Fill   => (0.22, 0.78, 0.22),
            Self::Refill => (0.22, 0.42, 0.90),
            Self::Wall   => (0.90, 0.22, 0.22),
        }
    }
    pub fn name(self) -> &'static str {
        match self { Self::Fill => "Fill", Self::Refill => "Refill", Self::Wall => "Wall" }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2  Quadrant
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
pub enum Quad { Kip, Ydr, Ele, Toi }

impl Quad {
    pub const ALL: [Self; 4] = [Self::Kip, Self::Ydr, Self::Ele, Self::Toi];

    pub fn prefix(self) -> &'static str {
        match self {
            Self::Kip => "Kip",
            Self::Ydr => "Ydr",
            Self::Ele => "Ele",
            Self::Toi => "Toi",
        }
    }

    pub fn isometry(self, half: f64, offset_x: f64, offset_y: f64, z: f64) -> Isometry3<f64> {
        let (tx, ty) = match self {
            Self::Kip => (offset_x,        offset_y       ),
            Self::Ydr => (offset_x + half, offset_y       ),
            Self::Ele => (offset_x,        offset_y + half),
            Self::Toi => (offset_x + half, offset_y + half),
        };
        Isometry3::from_parts(
            Translation3::new(tx, ty, z),
            UnitQuaternion::identity(),
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3  Slab / Wall factories
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_slabs(p: &RoyParams, store: &str, offset_x: f64, offset_y: f64) -> Vec<SlabSpec> {
    let (s, t, fh) = (p.side, p.slab_thickness, p.floor_height);
    vec![
        SlabSpec {
            label: format!("{store}_Slab_Ground"),
            store: store.into(),
            x: offset_x, y: offset_y, z: 0.0,
            length: s, width: s, thickness: t,
        },
        SlabSpec {
            label: format!("{store}_Slab_Floor2"),
            store: store.into(),
            x: offset_x, y: offset_y, z: fh,
            length: s, width: s, thickness: t,
        },
        SlabSpec {
            label: format!("{store}_Slab_Roof"),
            store: store.into(),
            x: offset_x, y: offset_y, z: fh * 2.0,
            length: s, width: s, thickness: t,
        },
    ]
}

pub fn make_walls(p: &RoyParams, store: &str, offset_x: f64, offset_y: f64) -> Vec<WallSpec> {
    let (s, t, fh, st) = (p.side, p.wall_thickness, p.floor_height, p.slab_thickness);
    let ht = t / 2.0;
    let wh = fh - st;
    let mut walls = Vec::new();
    for floor in 0u32..2 {
        let z  = st + floor as f64 * fh;
        let fl = format!("F{}", floor + 1);
        macro_rules! w {
            ($id:expr, $x1:expr, $y1:expr, $x2:expr, $y2:expr) => {
                WallSpec {
                    label: format!("{store}_Wall_{fl}_{}", $id),
                    store: store.into(),
                    x1: offset_x + $x1,
                    y1: offset_y + $y1,
                    x2: offset_x + $x2,
                    y2: offset_y + $y2,
                    z, height: wh, width: t, align: "Center".into(),
                }
            };
        }
        walls.push(w!("South", 0.0,  ht,    s,    ht   ));
        walls.push(w!("North", 0.0,  s-ht,  s,    s-ht ));
        walls.push(w!("West",  ht,   0.0,   ht,   s    ));
        walls.push(w!("East",  s-ht, 0.0,   s-ht, s    ));
    }
    walls
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4  QuadBuilder
// ═══════════════════════════════════════════════════════════════════════════════

pub struct QuadBuilder<'a> {
    p:        &'a RoyParams,
    iso:      Isometry3<f64>,
    pfx:      &'static str,
    store:    String,
    sh:       f64,
    lv:       u32,
    lvh:      f64,
    row_num:  u32,
    shelves:  Vec<ShelfSpec>,
    items:    Vec<ShelfItemSpec>,
}

impl<'a> QuadBuilder<'a> {
    pub fn new(
        p:        &'a RoyParams,
        q:        Quad,
        store:    &str,
        offset_x: f64,
        offset_y: f64,
    ) -> Self {
        let half    = p.side / 2.0;
        let z_shelf = p.floor_height + p.slab_thickness;
        let iso     = q.isometry(half, offset_x, offset_y, z_shelf);
        let sh      = p.shelf_height;
        let lv      = p.shelf_levels;
        let lvh     = sh / lv as f64;
        Self {
            p, iso, pfx: q.prefix(),
            store: store.to_string(),
            sh, lv, lvh, row_num: 0,
            shelves: Vec::new(),
            items:   Vec::new(),
        }
    }

    #[inline]
    fn world(&self, local: Point3<f64>) -> Point3<f64> { self.iso * local }

    fn add_run(
        &mut self,
        role:  Role,
        start: Point3<f64>,
        step:  Vector3<f64>,
        count: u32,
        bx:    f64,
        by:    f64,
    ) {
        self.row_num += 1;
        let row = self.row_num;
        let (cr, cg, cb) = role.color();

        for c in 0..count {
            let col          = c + 1;
            let local_corner = start + step * (c as f64);
            let wp           = self.world(local_corner);

            let label = format!(
                "{}.{}_R{:02}_C{:02}",
                self.store, self.pfx, row, col
            );

            self.shelves.push(ShelfSpec {
                label:    label.clone(),
                store:    self.store.clone(),
                x: wp.x, y: wp.y, z: wp.z,
                sx: bx, sy: by, sz: self.sh,
                role:     role.name().into(),
                color:    (cr, cg, cb),
                quadrant: self.pfx.into(),
                row, col,
            });

            if self.p.compute_items {
                for lv_i in 1..=self.lv {
                    let coord = format!(
                        "{}.{}.{}.{}.{}",
                        self.store, self.pfx, row, col, lv_i
                    );
                    let slot_local = Point3::new(
                        local_corner.x + bx / 2.0,
                        local_corner.y + by / 2.0,
                        (lv_i as f64 - 0.5) * self.lvh,
                    );
                    let sp      = self.world(slot_local);
                    let pricing = price::resolve_slot(
                        &coord, &self.store, self.pfx, row, col, lv_i,
                    );
                    self.items.push(ShelfItemSpec {
                        coord_label: coord,
                        shelf_label: label.clone(),
                        price:    pricing.price,
                        quantity: pricing.quantity,
                        world_x:  sp.x,
                        world_y:  sp.y,
                        world_z:  sp.z,
                        level:    lv_i,
                        // Newly generated items are always white (no highlight).
                        // The CLI overwrites this via update_item_color().
                        color: (1.0, 1.0, 1.0),
                    });
                }
            }
        }
    }

    pub fn build(mut self) -> (Vec<ShelfSpec>, Vec<ShelfItemSpec>) {
        let (sw, sd) = (self.p.shelf_width, self.p.shelf_depth);
        let half     = self.p.side / 2.0;
        let ir       = self.p.internal_rows;
        let n_x      = ((half - 2.0 * sd) / sw).floor() as u32;
        let n_y      = n_x;
        let row_step = (half - 2.0 * sd) / (ir + 1) as f64;

        for i in 0..ir {
            let role    = if i % 2 == 0 { Role::Fill } else { Role::Refill };
            let cy      = sd + row_step * (i as f64 + 1.0);
            let start_y = cy - sd / 2.0;
            self.add_run(
                role,
                Point3::new(sd, start_y, 0.0),
                Vector3::new(sw, 0.0, 0.0),
                n_x, sw, sd,
            );
        }
        self.add_run(Role::Wall, Point3::new(sd,       0.0,     0.0), Vector3::new(sw,  0.0, 0.0), n_x, sw, sd);
        self.add_run(Role::Wall, Point3::new(sd,       half-sd, 0.0), Vector3::new(sw,  0.0, 0.0), n_x, sw, sd);
        self.add_run(Role::Wall, Point3::new(0.0,      sd,      0.0), Vector3::new(0.0, sw,  0.0), n_y, sd, sw);
        self.add_run(Role::Wall, Point3::new(half-sd,  sd,      0.0), Vector3::new(0.0, sw,  0.0), n_y, sd, sw);

        (self.shelves, self.items)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5  Public factory
// ═══════════════════════════════════════════════════════════════════════════════

pub fn make_all_shelves_for_store(
    p:        &RoyParams,
    store:    &str,
    offset_x: f64,
    offset_y: f64,
) -> (Vec<ShelfSpec>, Vec<ShelfItemSpec>) {
    Quad::ALL.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut ss, mut is), q| {
            let (s, i) = QuadBuilder::new(p, q, store, offset_x, offset_y).build();
            ss.extend(s);
            is.extend(i);
            (ss, is)
        },
    )
}
