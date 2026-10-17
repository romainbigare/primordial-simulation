//! The viewer's names of chemistry 0 (`library/chem0.toml`) match chemistry 0: one name for each element, substance
//! and reaction.

use std::path::Path;

#[test]
fn the_library_names_every_element_substance_and_reaction_of_chemistry_0() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../library/chem0.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let names: toml::Table = toml::from_str(&text).expect("the library file is TOML");
    let count = |key: &str| names.get(key).and_then(|v| v.as_array()).map_or(0, |v| v.len());
    let (chem, _) = worldgen::chemistry0();
    assert_eq!(count("elements"), chem.n_elements, "element names");
    assert_eq!(count("substances"), chem.ns(), "substance names");
    assert_eq!(count("reactions"), chem.reactions.len(), "reaction names");
}
