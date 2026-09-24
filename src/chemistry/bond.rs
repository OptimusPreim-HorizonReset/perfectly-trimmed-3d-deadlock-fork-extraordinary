//! Persistent bond representation for molecular dynamics.

pub use crate::elements::BondKind;

/// A bond between two element particles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    /// Index of the first body in the simulation body list.
    pub body_a: usize,
    /// Index of the second body in the simulation body list.
    pub body_b: usize,
    /// Classification of the bond.
    pub kind: BondKind,
    /// Normalized bond strength in `[0, 1]`.
    pub strength: f32,
    /// Polarity of the bond in `[0, 1]`.
    pub polarity: f32,
    /// Preferred equilibrium distance between the two bodies.
    pub equilibrium_distance: f32,
    /// Energy required to break the bond (analog, arbitrary units).
    pub dissociation_energy: f32,
}

impl Bond {
    /// Creates a bond from a binding tendency, using the two body radii to set
    /// a reasonable equilibrium distance.
    pub fn from_tendency(
        body_a: usize,
        body_b: usize,
        radius_a: f32,
        radius_b: f32,
        tendency: crate::elements::BindingTendency,
    ) -> Self {
        let equilibrium_distance = (radius_a + radius_b) * (1.05 + tendency.strength * 0.25);
        let dissociation_energy = tendency.strength * (0.5 + tendency.polarity * 0.5);
        Self {
            body_a,
            body_b,
            kind: tendency.kind,
            strength: tendency.strength,
            polarity: tendency.polarity,
            equilibrium_distance,
            dissociation_energy,
        }
    }

    /// Returns true if the bond is strong enough to survive mild perturbations.
    pub fn is_stable(&self) -> bool {
        self.strength > 0.15 && self.dissociation_energy > 0.05
    }
}
