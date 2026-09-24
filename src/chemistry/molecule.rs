//! Molecule representation: a persistent collection of bonded element bodies.

use std::collections::HashSet;

use ultraviolet::Vec3;

/// A molecule formed from one or more bonded element particles.
#[derive(Clone, Debug)]
pub struct Molecule {
    /// Stable molecule id.
    pub id: u64,
    /// Indices into the simulation body list.
    pub body_indices: Vec<usize>,
    /// Sum formula as plain text, e.g. "H2O".
    pub formula: String,
    /// Total molecular mass (sum of body masses).
    pub mass: f32,
    /// Polarity in `[0, 1]`; 0 for perfectly nonpolar, 1 for ionic-like.
    pub polarity: f32,
    /// Aggregatest state at simulation temperature analogy: 0=solid, 1=liquid, 2=gas.
    pub state: u8,
    /// Name of the compound if it matches a known natural molecule.
    pub name: String,
}

impl Molecule {
    pub fn new(id: u64, body_indices: Vec<usize>) -> Self {
        Self {
            id,
            body_indices,
            formula: String::new(),
            mass: 0.0,
            polarity: 0.0,
            state: 2,
            name: String::new(),
        }
    }

    /// Computes centre of mass from the provided bodies.
    pub fn center_of_mass(&self, bodies: &[crate::body::Body]) -> Vec3 {
        if self.body_indices.is_empty() || self.mass <= 0.0 {
            return Vec3::zero();
        }
        let mut sum = Vec3::zero();
        for &idx in &self.body_indices {
            if let Some(body) = bodies.get(idx) {
                sum += body.pos * body.mass;
            }
        }
        sum / self.mass
    }

    /// Computes total linear momentum of the molecule.
    pub fn momentum(&self, bodies: &[crate::body::Body]) -> Vec3 {
        let mut p = Vec3::zero();
        for &idx in &self.body_indices {
            if let Some(body) = bodies.get(idx) {
                p += body.vel * body.mass;
            }
        }
        p
    }

    /// Returns true if the molecule contains a body index.
    pub fn contains(&self, body_index: usize) -> bool {
        self.body_indices.contains(&body_index)
    }

    /// Merges another molecule's body indices into this one, removing duplicates.
    pub fn merge(&mut self, other: &Molecule) {
        let set: HashSet<usize> = self.body_indices.iter().copied().collect();
        for &idx in &other.body_indices {
            if !set.contains(&idx) {
                self.body_indices.push(idx);
            }
        }
    }
}
