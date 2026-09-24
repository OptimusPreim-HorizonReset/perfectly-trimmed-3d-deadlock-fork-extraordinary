//! Curated library of natural molecules and compounds.

use std::collections::HashMap;

/// A known chemical compound with stoichiometric composition.
#[derive(Clone, Debug)]
pub struct Compound {
    pub name: &'static str,
    pub formula: &'static str,
    /// Map from atomic number to required atom count.
    pub composition: HashMap<u8, usize>,
    /// Polarity estimate in `[0, 1]`.
    pub polarity: f32,
    /// Aggregatest state at room temperature analogy: 0=solid, 1=liquid, 2=gas.
    pub state: u8,
}

impl Compound {
    fn new(
        name: &'static str,
        formula: &'static str,
        composition: &[(u8, usize)],
        polarity: f32,
        state: u8,
    ) -> Self {
        Self {
            name,
            formula,
            composition: composition.iter().copied().collect(),
            polarity,
            state,
        }
    }
}

/// Library of fundamental natural molecules.
pub struct CompoundLibrary {
    compounds: Vec<Compound>,
    by_formula: HashMap<String, usize>,
}

impl CompoundLibrary {
    pub fn new() -> Self {
        let compounds = Self::build_compounds();
        let by_formula = compounds
            .iter()
            .enumerate()
            .map(|(i, c)| (c.formula.to_string(), i))
            .collect();
        Self {
            compounds,
            by_formula,
        }
    }

    pub fn compounds(&self) -> &[Compound] {
        &self.compounds
    }

    pub fn by_formula(&self, formula: &str) -> Option<&Compound> {
        self.by_formula.get(formula).map(|&i| &self.compounds[i])
    }

    /// Tries to identify a compound from an element count map.
    pub fn identify(&self, counts: &HashMap<u8, usize>) -> Option<&Compound> {
        for compound in &self.compounds {
            if compound.composition.len() == counts.len()
                && compound
                    .composition
                    .iter()
                    .all(|(z, count)| counts.get(z).copied().unwrap_or(0) == *count)
            {
                return Some(compound);
            }
        }
        None
    }

    fn build_compounds() -> Vec<Compound> {
        vec![
            // Diatomic elements
            Compound::new("Hydrogen", "H2", &[(1, 2)], 0.0, 2),
            Compound::new("Nitrogen", "N2", &[(7, 2)], 0.0, 2),
            Compound::new("Oxygen", "O2", &[(8, 2)], 0.0, 2),
            Compound::new("Fluorine", "F2", &[(9, 2)], 0.0, 2),
            Compound::new("Chlorine", "Cl2", &[(17, 2)], 0.0, 2),
            Compound::new("Bromine", "Br2", &[(35, 2)], 0.0, 1),
            Compound::new("Iodine", "I2", &[(53, 2)], 0.0, 0),
            // Simple inorganics
            Compound::new("Water", "H2O", &[(1, 2), (8, 1)], 0.85, 1),
            Compound::new("Carbon dioxide", "CO2", &[(6, 1), (8, 2)], 0.25, 2),
            Compound::new("Carbon monoxide", "CO", &[(6, 1), (8, 1)], 0.35, 2),
            Compound::new("Ammonia", "NH3", &[(7, 1), (1, 3)], 0.75, 2),
            Compound::new("Methane", "CH4", &[(6, 1), (1, 4)], 0.0, 2),
            // Acids / bases / salts
            Compound::new("Hydrogen chloride", "HCl", &[(1, 1), (17, 1)], 0.95, 2),
            Compound::new("Sodium chloride", "NaCl", &[(11, 1), (17, 1)], 1.0, 0),
            Compound::new("Potassium chloride", "KCl", &[(19, 1), (17, 1)], 1.0, 0),
            Compound::new("Calcium chloride", "CaCl2", &[(20, 1), (17, 2)], 0.95, 0),
            Compound::new("Sodium hydroxide", "NaOH", &[(11, 1), (8, 1), (1, 1)], 0.95, 0),
            Compound::new("Potassium hydroxide", "KOH", &[(19, 1), (8, 1), (1, 1)], 0.95, 0),
            Compound::new("Sulfuric acid", "H2SO4", &[(1, 2), (16, 1), (8, 4)], 0.85, 1),
            Compound::new("Nitric acid", "HNO3", &[(1, 1), (7, 1), (8, 3)], 0.8, 1),
            Compound::new("Carbonic acid", "H2CO3", &[(1, 2), (6, 1), (8, 3)], 0.75, 1),
            // Oxides
            Compound::new("Calcium oxide", "CaO", &[(20, 1), (8, 1)], 0.95, 0),
            Compound::new("Magnesium oxide", "MgO", &[(12, 1), (8, 1)], 0.95, 0),
            Compound::new("Iron(III) oxide", "Fe2O3", &[(26, 2), (8, 3)], 0.9, 0),
            Compound::new("Silicon dioxide", "SiO2", &[(14, 1), (8, 2)], 0.7, 0),
            Compound::new("Aluminium oxide", "Al2O3", &[(13, 2), (8, 3)], 0.85, 0),
            // Carbonates / sulfates / nitrates
            Compound::new("Calcium carbonate", "CaCO3", &[(20, 1), (6, 1), (8, 3)], 0.85, 0),
            Compound::new("Sodium carbonate", "Na2CO3", &[(11, 2), (6, 1), (8, 3)], 0.9, 0),
            Compound::new("Calcium sulfate", "CaSO4", &[(20, 1), (16, 1), (8, 4)], 0.85, 0),
            Compound::new("Sodium nitrate", "NaNO3", &[(11, 1), (7, 1), (8, 3)], 0.85, 0),
            Compound::new("Potassium nitrate", "KNO3", &[(19, 1), (7, 1), (8, 3)], 0.85, 0),
            // Simple organic molecules
            Compound::new("Methanol", "CH3OH", &[(6, 1), (1, 4), (8, 1)], 0.7, 1),
            Compound::new("Ethane", "C2H6", &[(6, 2), (1, 6)], 0.05, 2),
            Compound::new("Ethene", "C2H4", &[(6, 2), (1, 4)], 0.1, 2),
            Compound::new("Ethyne", "C2H2", &[(6, 2), (1, 2)], 0.15, 2),
            Compound::new("Formaldehyde", "CH2O", &[(6, 1), (1, 2), (8, 1)], 0.55, 2),
            Compound::new("Acetic acid", "CH3COOH", &[(6, 2), (1, 4), (8, 2)], 0.65, 1),
            Compound::new("Glucose", "C6H12O6", &[(6, 6), (1, 12), (8, 6)], 0.75, 0),
            Compound::new("Ethanol", "C2H5OH", &[(6, 2), (1, 6), (8, 1)], 0.6, 1),
            // Biological building blocks (simplified)
            Compound::new("Ammonium", "NH4", &[(7, 1), (1, 4)], 0.7, 2),
            Compound::new("Hydroxide", "OH", &[(8, 1), (1, 1)], 0.9, 1),
            Compound::new("Hydronium", "H3O", &[(8, 1), (1, 3)], 0.9, 1),
            // Minerals / crystal prototypes
            Compound::new("Quartz", "SiO2", &[(14, 1), (8, 2)], 0.7, 0),
            Compound::new("Calcite", "CaCO3", &[(20, 1), (6, 1), (8, 3)], 0.85, 0),
            Compound::new("Halite", "NaCl", &[(11, 1), (17, 1)], 1.0, 0),
        ]
    }
}

impl Default for CompoundLibrary {
    fn default() -> Self {
        Self::new()
    }
}
