//! Chemistry subsystem for element-particle binding and molecule formation.
//!
//! This module extends the galactic-atom analogy with persistent molecular
//! bonds. It is intentionally an analog model: it uses real periodic trends
//! (electronegativity, valence, oxidation states) to drive bonding behaviour,
//! but it does not attempt ab-initio quantum chemistry.

pub mod bond;
pub mod compounds;
pub mod molecule;

pub use compounds::CompoundLibrary;
pub use molecule::Molecule;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn compound_library_identifies_water_and_salt() {
        let library = CompoundLibrary::new();

        let mut water = HashMap::new();
        water.insert(1, 2);
        water.insert(8, 1);
        let identified = library.identify(&water).expect("H2O should be known");
        assert_eq!(identified.formula, "H2O");
        assert_eq!(identified.name, "Water");

        let mut salt = HashMap::new();
        salt.insert(11, 1);
        salt.insert(17, 1);
        let identified = library.identify(&salt).expect("NaCl should be known");
        assert_eq!(identified.formula, "NaCl");
        assert_eq!(identified.name, "Sodium chloride");
    }

    #[test]
    fn compound_library_returns_none_for_unknown_mixture() {
        let library = CompoundLibrary::new();
        let mut counts = HashMap::new();
        counts.insert(1, 7);
        counts.insert(8, 2);
        assert!(library.identify(&counts).is_none());
    }

    #[test]
    fn molecule_tracks_membership_and_merges() {
        let mut m1 = Molecule::new(1, vec![0, 1, 2]);
        let m2 = Molecule::new(2, vec![2, 3]);
        m1.merge(&m2);
        assert_eq!(m1.body_indices.len(), 4);
        assert!(m1.contains(3));
        assert!(!m1.contains(99));
    }
}
