use super::*;

#[test]
fn pins_on_grid_and_keys_unique() {
    let lib = library();
    let mut keys: Vec<_> = lib.iter().map(|d| d.key).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), lib.len());
    for d in lib {
        for pin in &d.pins {
            assert_eq!(pin.pos.x % crate::geom::GRID, 0.0, "{} pin x off grid", d.key);
            assert_eq!(pin.pos.y % crate::geom::GRID, 0.0, "{} pin y off grid", d.key);
        }
    }
}
