use boxes_geo::{GeoPoint, Direction, NORTH, UP};

#[derive(Debug, Clone)]
pub struct ProductSpec {
    pub name: String,
    pub barcode: String,
    pub boxes: u32,
    pub spacing: f64,
}

impl ProductSpec {
    pub fn new(name: impl Into<String>, barcode: impl Into<String>, boxes: u32, spacing: f64) -> Self {
        Self {
            name: name.into(),
            barcode: barcode.into(),
            boxes,
            spacing,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProductOrigin {
    pub start: GeoPoint,
}

impl ProductOrigin {
    pub const fn new(start: GeoPoint) -> Self {
        Self { start }
    }
}

#[derive(Debug, Clone)]
pub struct ProductRun {
    pub spec: ProductSpec,
    pub origin: GeoPoint,
    pub step_up: f64,
    pub step_north: f64,
}

impl ProductRun {
    pub fn new(spec: ProductSpec, origin: GeoPoint, step_up: f64, step_north: f64) -> Self {
        Self {
            spec,
            origin,
            step_up,
            step_north,
        }
    }

    pub fn default_north_and_up(spec: ProductSpec, origin: GeoPoint) -> Self {
        Self::new(spec, origin, 1.0, 1.0)
    }

    pub fn positions(&self) -> impl Iterator<Item = GeoPoint> + '_ {
        (0..self.spec.boxes).map(move |i| {
            let p = self.origin;
            p.moved(UP, self.step_up * i as f64)
             .moved(NORTH, self.step_north * i as f64)
        })
    }
}
