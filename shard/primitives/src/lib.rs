// primitives/src/lib.rs
mod geometry;
mod types;

// geometry exports — make_frame now returns Vec<ShardSpec>
pub use geometry::make_all_parts;
pub use geometry::make_core;
pub use geometry::make_foundation;
pub use geometry::make_frame;
pub use geometry::make_spikes;

// type exports — EllipseFrameSpec is kept for backwards compat / optional use
pub use types::EllipseFrameSpec;
pub use types::FoundationSpec;
pub use types::ShardParams;
pub use types::ShardSpec;
pub use types::SpikeSpec;
