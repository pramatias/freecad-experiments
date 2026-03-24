// /home/emporas/repos/freecad/rust/a7/primitives/src/lib.rs
mod geometry;
mod types;

pub use geometry::make_all_parts;
pub use geometry::make_body_parts;
pub use geometry::make_chassis_parts;
pub use geometry::make_mechanical_parts;
pub use geometry::make_wheels;

pub use types::A7Params;
pub use types::BodyPartSpec;
pub use types::ChassisPartSpec;
pub use types::MechanicalPartSpec;
pub use types::WheelSpec;
