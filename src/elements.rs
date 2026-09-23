use crate::body::Body;
use ultraviolet::Vec3;

/// Periodic table definitions and galactic-atom generation helpers for the galaxy-as-atom simulation mode.
#[allow(dead_code)]
pub struct ElementDefinition {
    pub atomic_number: u8,
    pub symbol: &'static str,
    pub name: &'static str,
    pub atomic_weight: f32,
}

pub enum GalaxyMorphology {
    Dwarf,
    Spiral,
    Elliptical,
    AGN,
}

impl GalaxyMorphology {
    pub fn central_spin(&self) -> f32 {
        match self {
            GalaxyMorphology::Dwarf => 0.08,
            GalaxyMorphology::Spiral => 0.18,
            GalaxyMorphology::Elliptical => 0.10,
            GalaxyMorphology::AGN => 0.35,
        }
    }

    pub fn bulge_spin(&self) -> f32 {
        match self {
            GalaxyMorphology::Dwarf => 0.04,
            GalaxyMorphology::Spiral => 0.08,
            GalaxyMorphology::Elliptical => 0.03,
            GalaxyMorphology::AGN => 0.12,
        }
    }

    pub fn disk_spin_scale(&self) -> f32 {
        match self {
            GalaxyMorphology::Dwarf => 0.8,
            GalaxyMorphology::Spiral => 1.0,
            GalaxyMorphology::Elliptical => 0.4,
            GalaxyMorphology::AGN => 1.3,
        }
    }
}

static PRIME_DIMENSION_SEQUENCE: [u32; 120] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29,
    31, 37, 41, 43, 47, 53, 59, 61, 67, 71,
    73, 79, 83, 89, 97, 101, 103, 107, 109, 113,
    127, 131, 137, 139, 149, 151, 157, 163, 167, 173,
    179, 181, 191, 193, 197, 199, 211, 223, 227, 229,
    233, 239, 241, 251, 257, 263, 269, 271, 277, 281,
    283, 293, 307, 311, 313, 317, 331, 337, 347, 349,
    353, 359, 367, 373, 379, 383, 389, 397, 401, 409,
    419, 421, 431, 433, 439, 443, 449, 457, 461, 463,
    467, 479, 487, 491, 499, 503, 509, 521, 523, 541,
    547, 557, 563, 569, 571, 577, 587, 593, 599, 601,
    607, 613, 617, 619, 631, 641, 643, 647, 653, 659,
];

impl ElementDefinition {
    pub fn galaxy_morphology(&self) -> GalaxyMorphology {
        match self.atomic_number {
            1 | 2 => GalaxyMorphology::Dwarf,
            10 | 18 | 36 | 54 | 86 => GalaxyMorphology::Elliptical,
            z if z >= 87 => GalaxyMorphology::AGN,
            _ => GalaxyMorphology::Spiral,
        }
    }

    pub fn prime_dimension(&self) -> u32 {
        PRIME_DIMENSION_SEQUENCE
            .get(self.atomic_number as usize)
            .copied()
            .unwrap_or(2)
    }

    pub fn prime_gap(&self) -> u32 {
        let index = self.atomic_number as usize;
        if let (Some(current), Some(next)) = (
            PRIME_DIMENSION_SEQUENCE.get(index),
            PRIME_DIMENSION_SEQUENCE.get(index + 1),
        ) {
            next.saturating_sub(*current)
        } else {
            1
        }
    }

    pub fn mass_dimension(&self) -> f32 {
        (self.prime_dimension() as f32).ln().max(1.0)
    }

    pub fn light_signature(&self, light_energy: f32) -> f32 {
        light_energy * self.mass_dimension()
    }

    pub fn total_galactic_mass(&self, mass_unit: f32) -> f32 {
        (self.atomic_number as f32 + self.atomic_weight * 0.1) * mass_unit
    }

    pub fn bulge_mass_fraction(&self) -> f32 {
        match self.galaxy_morphology() {
            GalaxyMorphology::Dwarf => 0.14,
            GalaxyMorphology::Spiral => 0.22,
            GalaxyMorphology::Elliptical => 0.34,
            GalaxyMorphology::AGN => 0.28,
        }
    }

    pub fn halo_mass_fraction(&self) -> f32 {
        match self.galaxy_morphology() {
            GalaxyMorphology::Dwarf => 0.08,
            GalaxyMorphology::Spiral => 0.12,
            GalaxyMorphology::Elliptical => 0.18,
            GalaxyMorphology::AGN => 0.10,
        }
    }

    pub fn smbh_mass(&self, total_mass: f32) -> f32 {
        // In the galaxy-as-atom analogy, the central nucleus mass is balanced
        // with the total mass of orbiting structure, similar to a hydrogen atom
        // where the proton's central mass is of the same order as the orbital system.
        total_mass * 0.5
    }

    pub fn disk_particle_count(&self) -> usize {
        ((self.atomic_number as usize).max(1) * 4).saturating_add(12)
    }

    pub fn halo_particle_count(&self) -> usize {
        match self.galaxy_morphology() {
            GalaxyMorphology::Dwarf => 4,
            GalaxyMorphology::Spiral => 8,
            GalaxyMorphology::Elliptical => 10,
            GalaxyMorphology::AGN => 12,
        }
    }

    pub fn disk_radius_factor(&self) -> f32 {
        match self.galaxy_morphology() {
            GalaxyMorphology::Dwarf => 1.2,
            GalaxyMorphology::Spiral => 1.7,
            GalaxyMorphology::Elliptical => 1.3,
            GalaxyMorphology::AGN => 2.0,
        }
    }

    pub fn magnetism(&self, alpha: f32) -> f32 {
        alpha * self.mass_dimension()
    }

    pub fn gravitation(&self, beta: f32) -> f32 {
        let reactivity = self.prime_gap().max(1) as f32;
        beta * self.mass_dimension() / reactivity
    }
}

pub fn elements() -> &'static [ElementDefinition] {
    &ELEMENTS
}

pub fn element_by_atomic_number(atomic_number: u8) -> Option<&'static ElementDefinition> {
    if atomic_number == 0 || atomic_number as usize > ELEMENTS.len() {
        None
    } else {
        Some(&ELEMENTS[(atomic_number - 1) as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_dimension_lookup_matches_expected_primes() {
        let hydrogen = element_by_atomic_number(1).unwrap();
        assert_eq!(hydrogen.prime_dimension(), 3);
        assert_eq!(hydrogen.prime_gap(), 2);
        assert!(hydrogen.mass_dimension() > 0.0);

        let helium = element_by_atomic_number(2).unwrap();
        assert_eq!(helium.prime_dimension(), 5);
        assert_eq!(helium.prime_gap(), 2);

        let oganesson = element_by_atomic_number(118).unwrap();
        assert_eq!(oganesson.prime_dimension(), 653);
        assert_eq!(oganesson.prime_gap(), 6);
    }
}

pub fn generate_galactic_atom_system(
    element: &ElementDefinition,
    center: Vec3,
    rotation_axis: Vec3,
    mass_unit: f32,
    orbital_mass_unit: f32,
    radius_scale: f32,
    alpha: f32,
    beta: f32,
    light_energy: f32,
) -> Vec<Body> {
    let total_mass = element.total_galactic_mass(mass_unit);
    let mut smbh_mass = element.smbh_mass(total_mass);
    let mut orbit_mass = (total_mass - smbh_mass).max(total_mass * 0.1);
    orbit_mass *= orbital_mass_unit;
    let total_mass = smbh_mass + orbit_mass;
    if orbit_mass < 0.001 || smbh_mass <= 0.0 {
        smbh_mass = total_mass * 0.5;
        orbit_mass = total_mass - smbh_mass;
    }
    let bulge_mass = orbit_mass * element.bulge_mass_fraction();
    let halo_mass = orbit_mass * element.halo_mass_fraction();
    let disk_mass = (orbit_mass - bulge_mass - halo_mass).max(orbit_mass * 0.05);
    let disk_count = element.disk_particle_count();
    let halo_count = element.halo_particle_count();
    let morphology = element.galaxy_morphology();

    let prime_dimension = element.prime_dimension();
    let mass_dimension = element.mass_dimension();
    let reactivity = element.prime_gap() as f32;
    let magnetism = element.magnetism(alpha);
    let gravitation = element.gravitation(beta);
    let light_interaction = element.light_signature(light_energy);

    let rotation_axis = if rotation_axis == Vec3::zero() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        rotation_axis.normalized()
    };

    let mut bodies = Vec::with_capacity(2 + disk_count + halo_count);
    let center_radius = (smbh_mass.max(0.1)).cbrt() * 0.6 * radius_scale;

    bodies.push(Body::new_element(
        center,
        Vec3::zero(),
        smbh_mass,
        center_radius,
        morphology.central_spin(),
        rotation_axis,
        element.atomic_number,
        prime_dimension as f32,
        reactivity,
        magnetism,
        gravitation,
        light_interaction,
        mass_dimension,
    ));

    let bulge_radius = (bulge_mass.max(0.1)).cbrt() * 1.2 * radius_scale;
    bodies.push(Body::new_element(
        center + rotation_axis * (center_radius * 0.4),
        Vec3::zero(),
        bulge_mass,
        bulge_radius,
        morphology.bulge_spin(),
        rotation_axis,
        element.atomic_number,
        prime_dimension as f32,
        reactivity,
        magnetism,
        gravitation,
        light_interaction,
        mass_dimension,
    ));

    let (u, v) = plane_basis(rotation_axis);
    let mut accumulated_mass = smbh_mass + bulge_mass;
    let disk_radius = radius_scale * element.disk_radius_factor();

    for i in 0..disk_count {
        let fraction = (i as f32 + 0.5) / disk_count as f32;
        let angle = fraction * std::f32::consts::TAU + fastrand::f32() * 0.18;
        let cos = angle.cos();
        let sin = angle.sin();
        let radius = disk_radius * (0.25 + 0.75 * fraction.sqrt());
        let z = (fastrand::f32() - 0.5) * disk_radius * 0.06;
        let position = center + u * (cos * radius) + v * (sin * radius) + rotation_axis * z;
        let offset = position - center;
        let tangent = if offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(offset).normalized()
        } else {
            (u * -sin + v * cos).normalized()
        };
        let orbital_speed = ((accumulated_mass / radius.max(1.0)).max(0.01)).sqrt();
        let velocity = tangent * orbital_speed;
        let mass = (disk_mass / disk_count as f32).max(0.0005);
        let radius = (mass.cbrt() * 0.8 + 0.05) * radius_scale * 0.6;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            orbital_speed * 0.15 * morphology.disk_spin_scale(),
            rotation_axis,
            element.atomic_number,
            prime_dimension as f32,
            reactivity,
            magnetism,
            gravitation,
            light_interaction,
            mass_dimension,
        ));
        accumulated_mass += mass;
    }

    let halo_radius = disk_radius * 1.8;
    for _i in 0..halo_count {
        let theta = fastrand::f32() * std::f32::consts::PI;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let offset = Vec3::new(
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        ) * halo_radius;
        let position = center + offset;
        let tangent = if offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(offset).normalized()
        } else {
            u
        };
        let orbital_speed = ((accumulated_mass / halo_radius.max(1.0)).max(0.005)).sqrt();
        let velocity = tangent * orbital_speed * 0.5;
        let mass = (halo_mass / halo_count as f32).max(0.0002);
        let radius = (mass.cbrt() * 0.7 + 0.04) * radius_scale * 0.5;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            orbital_speed * 0.08,
            rotation_axis,
            element.atomic_number,
            prime_dimension as f32,
            reactivity,
            magnetism,
            gravitation,
            light_interaction,
            mass_dimension,
        ));
        accumulated_mass += mass;
    }

    bodies
}

pub fn generate_atomic_system(
    element: &ElementDefinition,
    center: Vec3,
    rotation_axis: Vec3,
    center_mass_unit: f32,
    orbital_mass_unit: f32,
    radius_scale: f32,
    alpha: f32,
    beta: f32,
    light_energy: f32,
) -> Vec<Body> {
    generate_galactic_atom_system(
        element,
        center,
        rotation_axis,
        center_mass_unit,
        orbital_mass_unit,
        radius_scale,
        alpha,
        beta,
        light_energy,
    )
}

fn plane_basis(axis: Vec3) -> (Vec3, Vec3) {
    let reference = if axis.abs().x < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(reference).normalized();
    let v = axis.cross(u).normalized();
    (u, v)
}

static ELEMENTS: [ElementDefinition; 118] = [
    ElementDefinition {
        atomic_number: 1,
        symbol: "H",
        name: "Hydrogen",
        atomic_weight: 1.008,
    },
    ElementDefinition {
        atomic_number: 2,
        symbol: "He",
        name: "Helium",
        atomic_weight: 4.0026,
    },
    ElementDefinition {
        atomic_number: 3,
        symbol: "Li",
        name: "Lithium",
        atomic_weight: 6.941,
    },
    ElementDefinition {
        atomic_number: 4,
        symbol: "Be",
        name: "Beryllium",
        atomic_weight: 9.0122,
    },
    ElementDefinition {
        atomic_number: 5,
        symbol: "B",
        name: "Boron",
        atomic_weight: 10.81,
    },
    ElementDefinition {
        atomic_number: 6,
        symbol: "C",
        name: "Carbon",
        atomic_weight: 12.011,
    },
    ElementDefinition {
        atomic_number: 7,
        symbol: "N",
        name: "Nitrogen",
        atomic_weight: 14.007,
    },
    ElementDefinition {
        atomic_number: 8,
        symbol: "O",
        name: "Oxygen",
        atomic_weight: 15.999,
    },
    ElementDefinition {
        atomic_number: 9,
        symbol: "F",
        name: "Fluorine",
        atomic_weight: 18.998,
    },
    ElementDefinition {
        atomic_number: 10,
        symbol: "Ne",
        name: "Neon",
        atomic_weight: 20.180,
    },
    ElementDefinition {
        atomic_number: 11,
        symbol: "Na",
        name: "Sodium",
        atomic_weight: 22.990,
    },
    ElementDefinition {
        atomic_number: 12,
        symbol: "Mg",
        name: "Magnesium",
        atomic_weight: 24.305,
    },
    ElementDefinition {
        atomic_number: 13,
        symbol: "Al",
        name: "Aluminium",
        atomic_weight: 26.982,
    },
    ElementDefinition {
        atomic_number: 14,
        symbol: "Si",
        name: "Silicon",
        atomic_weight: 28.085,
    },
    ElementDefinition {
        atomic_number: 15,
        symbol: "P",
        name: "Phosphorus",
        atomic_weight: 30.974,
    },
    ElementDefinition {
        atomic_number: 16,
        symbol: "S",
        name: "Sulfur",
        atomic_weight: 32.06,
    },
    ElementDefinition {
        atomic_number: 17,
        symbol: "Cl",
        name: "Chlorine",
        atomic_weight: 35.45,
    },
    ElementDefinition {
        atomic_number: 18,
        symbol: "Ar",
        name: "Argon",
        atomic_weight: 39.948,
    },
    ElementDefinition {
        atomic_number: 19,
        symbol: "K",
        name: "Potassium",
        atomic_weight: 39.098,
    },
    ElementDefinition {
        atomic_number: 20,
        symbol: "Ca",
        name: "Calcium",
        atomic_weight: 40.078,
    },
    ElementDefinition {
        atomic_number: 21,
        symbol: "Sc",
        name: "Scandium",
        atomic_weight: 44.956,
    },
    ElementDefinition {
        atomic_number: 22,
        symbol: "Ti",
        name: "Titanium",
        atomic_weight: 47.867,
    },
    ElementDefinition {
        atomic_number: 23,
        symbol: "V",
        name: "Vanadium",
        atomic_weight: 50.942,
    },
    ElementDefinition {
        atomic_number: 24,
        symbol: "Cr",
        name: "Chromium",
        atomic_weight: 51.996,
    },
    ElementDefinition {
        atomic_number: 25,
        symbol: "Mn",
        name: "Manganese",
        atomic_weight: 54.938,
    },
    ElementDefinition {
        atomic_number: 26,
        symbol: "Fe",
        name: "Iron",
        atomic_weight: 55.845,
    },
    ElementDefinition {
        atomic_number: 27,
        symbol: "Co",
        name: "Cobalt",
        atomic_weight: 58.933,
    },
    ElementDefinition {
        atomic_number: 28,
        symbol: "Ni",
        name: "Nickel",
        atomic_weight: 58.693,
    },
    ElementDefinition {
        atomic_number: 29,
        symbol: "Cu",
        name: "Copper",
        atomic_weight: 63.546,
    },
    ElementDefinition {
        atomic_number: 30,
        symbol: "Zn",
        name: "Zinc",
        atomic_weight: 65.38,
    },
    ElementDefinition {
        atomic_number: 31,
        symbol: "Ga",
        name: "Gallium",
        atomic_weight: 69.723,
    },
    ElementDefinition {
        atomic_number: 32,
        symbol: "Ge",
        name: "Germanium",
        atomic_weight: 72.630,
    },
    ElementDefinition {
        atomic_number: 33,
        symbol: "As",
        name: "Arsenic",
        atomic_weight: 74.922,
    },
    ElementDefinition {
        atomic_number: 34,
        symbol: "Se",
        name: "Selenium",
        atomic_weight: 78.971,
    },
    ElementDefinition {
        atomic_number: 35,
        symbol: "Br",
        name: "Bromine",
        atomic_weight: 79.904,
    },
    ElementDefinition {
        atomic_number: 36,
        symbol: "Kr",
        name: "Krypton",
        atomic_weight: 83.798,
    },
    ElementDefinition {
        atomic_number: 37,
        symbol: "Rb",
        name: "Rubidium",
        atomic_weight: 85.468,
    },
    ElementDefinition {
        atomic_number: 38,
        symbol: "Sr",
        name: "Strontium",
        atomic_weight: 87.62,
    },
    ElementDefinition {
        atomic_number: 39,
        symbol: "Y",
        name: "Yttrium",
        atomic_weight: 88.906,
    },
    ElementDefinition {
        atomic_number: 40,
        symbol: "Zr",
        name: "Zirconium",
        atomic_weight: 91.224,
    },
    ElementDefinition {
        atomic_number: 41,
        symbol: "Nb",
        name: "Niobium",
        atomic_weight: 92.906,
    },
    ElementDefinition {
        atomic_number: 42,
        symbol: "Mo",
        name: "Molybdenum",
        atomic_weight: 95.95,
    },
    ElementDefinition {
        atomic_number: 43,
        symbol: "Tc",
        name: "Technetium",
        atomic_weight: 98.0,
    },
    ElementDefinition {
        atomic_number: 44,
        symbol: "Ru",
        name: "Ruthenium",
        atomic_weight: 101.07,
    },
    ElementDefinition {
        atomic_number: 45,
        symbol: "Rh",
        name: "Rhodium",
        atomic_weight: 102.91,
    },
    ElementDefinition {
        atomic_number: 46,
        symbol: "Pd",
        name: "Palladium",
        atomic_weight: 106.42,
    },
    ElementDefinition {
        atomic_number: 47,
        symbol: "Ag",
        name: "Silver",
        atomic_weight: 107.87,
    },
    ElementDefinition {
        atomic_number: 48,
        symbol: "Cd",
        name: "Cadmium",
        atomic_weight: 112.41,
    },
    ElementDefinition {
        atomic_number: 49,
        symbol: "In",
        name: "Indium",
        atomic_weight: 114.82,
    },
    ElementDefinition {
        atomic_number: 50,
        symbol: "Sn",
        name: "Tin",
        atomic_weight: 118.71,
    },
    ElementDefinition {
        atomic_number: 51,
        symbol: "Sb",
        name: "Antimony",
        atomic_weight: 121.76,
    },
    ElementDefinition {
        atomic_number: 52,
        symbol: "Te",
        name: "Tellurium",
        atomic_weight: 127.60,
    },
    ElementDefinition {
        atomic_number: 53,
        symbol: "I",
        name: "Iodine",
        atomic_weight: 126.90,
    },
    ElementDefinition {
        atomic_number: 54,
        symbol: "Xe",
        name: "Xenon",
        atomic_weight: 131.29,
    },
    ElementDefinition {
        atomic_number: 55,
        symbol: "Cs",
        name: "Caesium",
        atomic_weight: 132.91,
    },
    ElementDefinition {
        atomic_number: 56,
        symbol: "Ba",
        name: "Barium",
        atomic_weight: 137.33,
    },
    ElementDefinition {
        atomic_number: 57,
        symbol: "La",
        name: "Lanthanum",
        atomic_weight: 138.91,
    },
    ElementDefinition {
        atomic_number: 58,
        symbol: "Ce",
        name: "Cerium",
        atomic_weight: 140.12,
    },
    ElementDefinition {
        atomic_number: 59,
        symbol: "Pr",
        name: "Praseodymium",
        atomic_weight: 140.91,
    },
    ElementDefinition {
        atomic_number: 60,
        symbol: "Nd",
        name: "Neodymium",
        atomic_weight: 144.24,
    },
    ElementDefinition {
        atomic_number: 61,
        symbol: "Pm",
        name: "Promethium",
        atomic_weight: 145.0,
    },
    ElementDefinition {
        atomic_number: 62,
        symbol: "Sm",
        name: "Samarium",
        atomic_weight: 150.36,
    },
    ElementDefinition {
        atomic_number: 63,
        symbol: "Eu",
        name: "Europium",
        atomic_weight: 151.96,
    },
    ElementDefinition {
        atomic_number: 64,
        symbol: "Gd",
        name: "Gadolinium",
        atomic_weight: 157.25,
    },
    ElementDefinition {
        atomic_number: 65,
        symbol: "Tb",
        name: "Terbium",
        atomic_weight: 158.93,
    },
    ElementDefinition {
        atomic_number: 66,
        symbol: "Dy",
        name: "Dysprosium",
        atomic_weight: 162.50,
    },
    ElementDefinition {
        atomic_number: 67,
        symbol: "Ho",
        name: "Holmium",
        atomic_weight: 164.93,
    },
    ElementDefinition {
        atomic_number: 68,
        symbol: "Er",
        name: "Erbium",
        atomic_weight: 167.26,
    },
    ElementDefinition {
        atomic_number: 69,
        symbol: "Tm",
        name: "Thulium",
        atomic_weight: 168.93,
    },
    ElementDefinition {
        atomic_number: 70,
        symbol: "Yb",
        name: "Ytterbium",
        atomic_weight: 173.05,
    },
    ElementDefinition {
        atomic_number: 71,
        symbol: "Lu",
        name: "Lutetium",
        atomic_weight: 174.97,
    },
    ElementDefinition {
        atomic_number: 72,
        symbol: "Hf",
        name: "Hafnium",
        atomic_weight: 178.49,
    },
    ElementDefinition {
        atomic_number: 73,
        symbol: "Ta",
        name: "Tantalum",
        atomic_weight: 180.95,
    },
    ElementDefinition {
        atomic_number: 74,
        symbol: "W",
        name: "Tungsten",
        atomic_weight: 183.84,
    },
    ElementDefinition {
        atomic_number: 75,
        symbol: "Re",
        name: "Rhenium",
        atomic_weight: 186.21,
    },
    ElementDefinition {
        atomic_number: 76,
        symbol: "Os",
        name: "Osmium",
        atomic_weight: 190.23,
    },
    ElementDefinition {
        atomic_number: 77,
        symbol: "Ir",
        name: "Iridium",
        atomic_weight: 192.22,
    },
    ElementDefinition {
        atomic_number: 78,
        symbol: "Pt",
        name: "Platinum",
        atomic_weight: 195.08,
    },
    ElementDefinition {
        atomic_number: 79,
        symbol: "Au",
        name: "Gold",
        atomic_weight: 196.97,
    },
    ElementDefinition {
        atomic_number: 80,
        symbol: "Hg",
        name: "Mercury",
        atomic_weight: 200.59,
    },
    ElementDefinition {
        atomic_number: 81,
        symbol: "Tl",
        name: "Thallium",
        atomic_weight: 204.38,
    },
    ElementDefinition {
        atomic_number: 82,
        symbol: "Pb",
        name: "Lead",
        atomic_weight: 207.2,
    },
    ElementDefinition {
        atomic_number: 83,
        symbol: "Bi",
        name: "Bismuth",
        atomic_weight: 208.98,
    },
    ElementDefinition {
        atomic_number: 84,
        symbol: "Po",
        name: "Polonium",
        atomic_weight: 209.0,
    },
    ElementDefinition {
        atomic_number: 85,
        symbol: "At",
        name: "Astatine",
        atomic_weight: 210.0,
    },
    ElementDefinition {
        atomic_number: 86,
        symbol: "Rn",
        name: "Radon",
        atomic_weight: 222.0,
    },
    ElementDefinition {
        atomic_number: 87,
        symbol: "Fr",
        name: "Francium",
        atomic_weight: 223.0,
    },
    ElementDefinition {
        atomic_number: 88,
        symbol: "Ra",
        name: "Radium",
        atomic_weight: 226.0,
    },
    ElementDefinition {
        atomic_number: 89,
        symbol: "Ac",
        name: "Actinium",
        atomic_weight: 227.0,
    },
    ElementDefinition {
        atomic_number: 90,
        symbol: "Th",
        name: "Thorium",
        atomic_weight: 232.04,
    },
    ElementDefinition {
        atomic_number: 91,
        symbol: "Pa",
        name: "Protactinium",
        atomic_weight: 231.04,
    },
    ElementDefinition {
        atomic_number: 92,
        symbol: "U",
        name: "Uranium",
        atomic_weight: 238.03,
    },
    ElementDefinition {
        atomic_number: 93,
        symbol: "Np",
        name: "Neptunium",
        atomic_weight: 237.0,
    },
    ElementDefinition {
        atomic_number: 94,
        symbol: "Pu",
        name: "Plutonium",
        atomic_weight: 244.0,
    },
    ElementDefinition {
        atomic_number: 95,
        symbol: "Am",
        name: "Americium",
        atomic_weight: 243.0,
    },
    ElementDefinition {
        atomic_number: 96,
        symbol: "Cm",
        name: "Curium",
        atomic_weight: 247.0,
    },
    ElementDefinition {
        atomic_number: 97,
        symbol: "Bk",
        name: "Berkelium",
        atomic_weight: 247.0,
    },
    ElementDefinition {
        atomic_number: 98,
        symbol: "Cf",
        name: "Californium",
        atomic_weight: 251.0,
    },
    ElementDefinition {
        atomic_number: 99,
        symbol: "Es",
        name: "Einsteinium",
        atomic_weight: 252.0,
    },
    ElementDefinition {
        atomic_number: 100,
        symbol: "Fm",
        name: "Fermium",
        atomic_weight: 257.0,
    },
    ElementDefinition {
        atomic_number: 101,
        symbol: "Md",
        name: "Mendelevium",
        atomic_weight: 258.0,
    },
    ElementDefinition {
        atomic_number: 102,
        symbol: "No",
        name: "Nobelium",
        atomic_weight: 259.0,
    },
    ElementDefinition {
        atomic_number: 103,
        symbol: "Lr",
        name: "Lawrencium",
        atomic_weight: 262.0,
    },
    ElementDefinition {
        atomic_number: 104,
        symbol: "Rf",
        name: "Rutherfordium",
        atomic_weight: 267.0,
    },
    ElementDefinition {
        atomic_number: 105,
        symbol: "Db",
        name: "Dubnium",
        atomic_weight: 270.0,
    },
    ElementDefinition {
        atomic_number: 106,
        symbol: "Sg",
        name: "Seaborgium",
        atomic_weight: 271.0,
    },
    ElementDefinition {
        atomic_number: 107,
        symbol: "Bh",
        name: "Bohrium",
        atomic_weight: 270.0,
    },
    ElementDefinition {
        atomic_number: 108,
        symbol: "Hs",
        name: "Hassium",
        atomic_weight: 277.0,
    },
    ElementDefinition {
        atomic_number: 109,
        symbol: "Mt",
        name: "Meitnerium",
        atomic_weight: 278.0,
    },
    ElementDefinition {
        atomic_number: 110,
        symbol: "Ds",
        name: "Darmstadtium",
        atomic_weight: 281.0,
    },
    ElementDefinition {
        atomic_number: 111,
        symbol: "Rg",
        name: "Roentgenium",
        atomic_weight: 282.0,
    },
    ElementDefinition {
        atomic_number: 112,
        symbol: "Cn",
        name: "Copernicium",
        atomic_weight: 285.0,
    },
    ElementDefinition {
        atomic_number: 113,
        symbol: "Nh",
        name: "Nihonium",
        atomic_weight: 286.0,
    },
    ElementDefinition {
        atomic_number: 114,
        symbol: "Fl",
        name: "Flerovium",
        atomic_weight: 289.0,
    },
    ElementDefinition {
        atomic_number: 115,
        symbol: "Mc",
        name: "Moscovium",
        atomic_weight: 290.0,
    },
    ElementDefinition {
        atomic_number: 116,
        symbol: "Lv",
        name: "Livermorium",
        atomic_weight: 293.0,
    },
    ElementDefinition {
        atomic_number: 117,
        symbol: "Ts",
        name: "Tennessine",
        atomic_weight: 294.0,
    },
    ElementDefinition {
        atomic_number: 118,
        symbol: "Og",
        name: "Oganesson",
        atomic_weight: 294.0,
    },
];
