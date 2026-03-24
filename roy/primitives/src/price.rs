// /home/emporas/repos/freecad/rust/roy/src/price.rs
pub struct SlotPricing {
    pub price:    f64,
    pub quantity: u32,
}

/// Look up price and quantity for a shelf slot.
///
/// `store`    — e.g. "Kifisos"
/// `quadrant` — e.g. "Kip"
pub fn resolve_slot(
    _coord_label: &str,
    _store:       &str,
    _quadrant:    &str,
    _row:         u32,
    _col:         u32,
    _level:       u32,
) -> SlotPricing {
    SlotPricing { price: 1.00, quantity: 100 }
}
