// /home/emporas/repos/freecad/rust/roy/primitives/src/lib.rs
mod geometry;
mod price;
mod types;

pub use geometry::Role;
pub use geometry::Quad;
pub use geometry::QuadBuilder;
pub use geometry::make_slabs;
pub use geometry::make_walls;
pub use geometry::make_all_shelves_for_store;

pub use price::SlotPricing;
pub use price::resolve_slot;

pub use types::RoyParams;
pub use types::SlabSpec;
pub use types::WallSpec;
pub use types::ShelfSpec;
pub use types::ShelfItemSpec;
pub use types::CurrentSlabState;
pub use types::CurrentWallState;
pub use types::CurrentShelfState;
pub use types::CurrentItemState;
pub use types::BuildingDiff;
pub use types::ItemDiff;
