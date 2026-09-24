use crate::body::Body;
use ultraviolet::Vec3;

/// Periodic-table definitions and galactic-atom generation helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChemicalFamily {
    AlkaliMetal,
    AlkalineEarthMetal,
    TransitionMetal,
    PostTransitionMetal,
    Metalloid,
    ReactiveNonmetal,
    Halogen,
    NobleGas,
    Lanthanide,
    Actinide,
}

impl ChemicalFamily {
    pub fn is_metal(self) -> bool {
        matches!(
            self,
            Self::AlkaliMetal
                | Self::AlkalineEarthMetal
                | Self::TransitionMetal
                | Self::PostTransitionMetal
                | Self::Lanthanide
                | Self::Actinide
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChemistryColorGroup {
    RedOrange,
    Yellow,
    Green,
    Turquoise,
    Blue,
    Indigo,
    Violet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectronegativityCategory {
    StrongDonor,
    ModerateDonor,
    Metallic,
    Polar,
    Acceptor,
    StrongAcceptor,
    InertOrUndefined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BondKind {
    Inert,
    Metallic,
    NonpolarCovalent,
    PolarCovalent,
    Ionic,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementColor {
    pub hue_degrees: f32,
    pub saturation: f32,
    pub lightness: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BindingTendency {
    pub kind: BondKind,
    pub strength: f32,
    pub polarity: f32,
}

#[derive(Clone, Copy, Debug)]
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
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307,
    311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421,
    431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547,
    557, 563, 569, 571, 577, 587, 593, 599, 601, 607, 613, 617, 619, 631, 641, 643, 647, 653, 659,
];

impl ElementDefinition {
    pub fn group(&self) -> u8 {
        match self.atomic_number {
            1 => 1,
            2 => 18,
            3..=4 => self.atomic_number - 2,
            5..=10 => self.atomic_number + 8,
            11..=12 => self.atomic_number - 10,
            13..=18 => self.atomic_number,
            19..=36 => self.atomic_number - 18,
            37..=54 => self.atomic_number - 36,
            55 => 1,
            56 => 2,
            57..=71 => 3,
            72..=86 => self.atomic_number - 68,
            87 => 1,
            88 => 2,
            89..=103 => 3,
            104..=118 => self.atomic_number - 100,
            _ => 0,
        }
    }

    pub fn chemical_family(&self) -> ChemicalFamily {
        match self.atomic_number {
            3 | 11 | 19 | 37 | 55 | 87 => ChemicalFamily::AlkaliMetal,
            4 | 12 | 20 | 38 | 56 | 88 => ChemicalFamily::AlkalineEarthMetal,
            2 | 10 | 18 | 36 | 54 | 86 | 118 => ChemicalFamily::NobleGas,
            9 | 17 | 35 | 53 | 85 | 117 => ChemicalFamily::Halogen,
            5 | 14 | 32 | 33 | 51 | 52 => ChemicalFamily::Metalloid,
            1 | 6 | 7 | 8 | 15 | 16 | 34 => ChemicalFamily::ReactiveNonmetal,
            57..=71 => ChemicalFamily::Lanthanide,
            89..=103 => ChemicalFamily::Actinide,
            13 | 31 | 49 | 50 | 81 | 82 | 83 | 84 | 113 | 114 | 115 | 116 => {
                ChemicalFamily::PostTransitionMetal
            }
            _ => ChemicalFamily::TransitionMetal,
        }
    }

    pub fn chemistry_color_group(&self) -> ChemistryColorGroup {
        match self.chemical_family() {
            ChemicalFamily::AlkaliMetal => ChemistryColorGroup::RedOrange,
            ChemicalFamily::AlkalineEarthMetal => ChemistryColorGroup::Yellow,
            ChemicalFamily::TransitionMetal
            | ChemicalFamily::PostTransitionMetal
            | ChemicalFamily::Lanthanide
            | ChemicalFamily::Actinide => ChemistryColorGroup::Green,
            ChemicalFamily::Metalloid => ChemistryColorGroup::Turquoise,
            ChemicalFamily::ReactiveNonmetal => ChemistryColorGroup::Blue,
            ChemicalFamily::Halogen => ChemistryColorGroup::Indigo,
            ChemicalFamily::NobleGas => ChemistryColorGroup::Violet,
        }
    }

    /// Established Pauling electronegativity where a meaningful value is available.
    /// `None` is retained for noble gases and insufficiently characterized superheavy elements.
    pub fn pauling_electronegativity(&self) -> Option<f32> {
        const PERIOD_1: [Option<f32>; 2] = [Some(2.20), None];
        const PERIOD_2: [Option<f32>; 8] = [
            Some(0.98),
            Some(1.57),
            Some(2.04),
            Some(2.55),
            Some(3.04),
            Some(3.44),
            Some(3.98),
            None,
        ];
        const PERIOD_3: [Option<f32>; 8] = [
            Some(0.93),
            Some(1.31),
            Some(1.61),
            Some(1.90),
            Some(2.19),
            Some(2.58),
            Some(3.16),
            None,
        ];
        const PERIOD_4: [Option<f32>; 18] = [
            Some(0.82),
            Some(1.00),
            Some(1.36),
            Some(1.54),
            Some(1.63),
            Some(1.66),
            Some(1.55),
            Some(1.83),
            Some(1.88),
            Some(1.91),
            Some(1.90),
            Some(1.65),
            Some(1.81),
            Some(2.01),
            Some(2.18),
            Some(2.55),
            Some(2.96),
            Some(3.00),
        ];
        const PERIOD_5: [Option<f32>; 18] = [
            Some(0.82),
            Some(0.95),
            Some(1.22),
            Some(1.33),
            Some(1.60),
            Some(2.16),
            Some(1.90),
            Some(2.20),
            Some(2.28),
            Some(2.20),
            Some(1.93),
            Some(1.69),
            Some(1.78),
            Some(1.96),
            Some(2.05),
            Some(2.10),
            Some(2.66),
            Some(2.60),
        ];
        const PERIOD_6: [Option<f32>; 32] = [
            Some(0.79),
            Some(0.89),
            Some(1.10),
            Some(1.12),
            Some(1.13),
            Some(1.14),
            Some(1.13),
            Some(1.17),
            Some(1.20),
            Some(1.20),
            Some(1.10),
            Some(1.22),
            Some(1.23),
            Some(1.24),
            Some(1.25),
            Some(1.10),
            Some(1.27),
            Some(1.30),
            Some(1.50),
            Some(2.36),
            Some(1.90),
            Some(2.20),
            Some(2.20),
            Some(2.28),
            Some(2.54),
            Some(2.00),
            Some(1.62),
            Some(2.33),
            Some(2.02),
            Some(2.00),
            Some(2.20),
            None,
        ];
        const PERIOD_7: [Option<f32>; 32] = [
            Some(0.70),
            Some(0.90),
            Some(1.10),
            Some(1.30),
            Some(1.50),
            Some(1.38),
            Some(1.36),
            Some(1.28),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            Some(1.30),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ];

        match self.atomic_number {
            1..=2 => PERIOD_1[(self.atomic_number - 1) as usize],
            3..=10 => PERIOD_2[(self.atomic_number - 3) as usize],
            11..=18 => PERIOD_3[(self.atomic_number - 11) as usize],
            19..=36 => PERIOD_4[(self.atomic_number - 19) as usize],
            37..=54 => PERIOD_5[(self.atomic_number - 37) as usize],
            55..=86 => PERIOD_6[(self.atomic_number - 55) as usize],
            87..=118 => PERIOD_7[(self.atomic_number - 87) as usize],
            _ => None,
        }
    }

    fn estimated_electronegativity(&self) -> f32 {
        let period_drop = (self.period().saturating_sub(2) as f32) * 0.045;
        let group_estimate = match self.group() {
            1 => 0.85,
            2 => 1.10,
            3 => 1.25,
            4 => 1.40,
            5 => 1.55,
            6 => 1.75,
            7 => 1.85,
            8 => 1.95,
            9 => 2.05,
            10 => 2.10,
            11 => 2.00,
            12 => 1.75,
            13 => 1.80,
            14 => 2.05,
            15 => 2.30,
            16 => 2.65,
            17 => 3.10,
            18 => 3.55,
            _ => 1.50,
        };
        (group_estimate - period_drop).clamp(0.70, 3.90)
    }

    /// A total value for rendering and analog interactions. Measured Pauling values
    /// are preferred; missing values use a bounded group/period estimate.
    pub fn effective_electronegativity(&self) -> f32 {
        self.pauling_electronegativity()
            .unwrap_or_else(|| self.estimated_electronegativity())
    }

    pub fn electronegativity_is_estimated(&self) -> bool {
        self.pauling_electronegativity().is_none()
    }

    pub fn electronegativity_category(&self) -> ElectronegativityCategory {
        if self.chemical_family() == ChemicalFamily::NobleGas {
            return ElectronegativityCategory::InertOrUndefined;
        }

        match self.effective_electronegativity() {
            value if value <= 1.0 => ElectronegativityCategory::StrongDonor,
            value if value <= 1.5 => ElectronegativityCategory::ModerateDonor,
            value if value <= 2.0 => ElectronegativityCategory::Metallic,
            value if value <= 2.5 => ElectronegativityCategory::Polar,
            value if value <= 3.0 => ElectronegativityCategory::Acceptor,
            _ => ElectronegativityCategory::StrongAcceptor,
        }
    }

    pub fn reactivity_score(&self) -> f32 {
        let period = self.period() as f32;
        let base = match self.chemical_family() {
            ChemicalFamily::AlkaliMetal => 0.66 + period * 0.045,
            ChemicalFamily::AlkalineEarthMetal => 0.42 + period * 0.050,
            ChemicalFamily::TransitionMetal => {
                0.32 + (self.group() as f32 - 7.0).abs() * 0.012 + period * 0.010
            }
            ChemicalFamily::PostTransitionMetal => 0.42 + period * 0.018,
            ChemicalFamily::Metalloid => 0.48 + period * 0.012,
            ChemicalFamily::ReactiveNonmetal => match self.atomic_number {
                1 => 0.82,
                6 => 0.52,
                7 => 0.62,
                8 => 0.90,
                15 => 0.70,
                16 => 0.74,
                34 => 0.68,
                _ => 0.60,
            },
            ChemicalFamily::Halogen => 1.04 - period * 0.055,
            ChemicalFamily::NobleGas => match self.atomic_number {
                54 => 0.10,
                86 => 0.12,
                118 => 0.16,
                _ => 0.02,
            },
            ChemicalFamily::Lanthanide => 0.58 + (self.atomic_number - 57) as f32 * 0.002,
            ChemicalFamily::Actinide => 0.69 + (self.atomic_number - 89) as f32 * 0.003,
        };

        // The tiny identity term keeps template values deterministic and distinct
        // without changing the family-level chemical trend.
        (base + self.atomic_number as f32 * 0.0001).clamp(0.0, 1.0)
    }

    pub fn core_reactivity_score(&self) -> f32 {
        (self.reactivity_score() * 0.35).clamp(0.0, 1.0)
    }

    pub fn core_suitability(&self) -> f32 {
        let stability = 1.0 - self.reactivity_score();
        let mass_factor = (self.atomic_weight / 294.0).sqrt();
        (0.15 + stability * 0.65 + mass_factor * 0.20).clamp(0.05, 1.0)
    }

    pub fn element_color(&self) -> ElementColor {
        let (base_hue, saturation, lightness): (f32, f32, f32) = match self.chemistry_color_group()
        {
            ChemistryColorGroup::RedOrange => (22.0, 94.0, 58.0),
            ChemistryColorGroup::Yellow => (72.0, 92.0, 72.0),
            ChemistryColorGroup::Green => (132.0, 72.0, 58.0),
            ChemistryColorGroup::Turquoise => (184.0, 78.0, 62.0),
            ChemistryColorGroup::Blue => (232.0, 82.0, 64.0),
            ChemistryColorGroup::Indigo => (274.0, 86.0, 65.0),
            ChemistryColorGroup::Violet => (310.0, 72.0, 70.0),
        };
        let identity_shift = ((self.atomic_number as f32 * 0.618_034).fract() - 0.5) * 8.0;
        let electronegativity_shift = (self.effective_electronegativity() - 2.0) * 1.5;

        ElementColor {
            hue_degrees: (base_hue + identity_shift + electronegativity_shift).rem_euclid(360.0),
            saturation: (saturation + self.reactivity_score() * 6.0).clamp(0.0, 100.0),
            lightness: lightness.clamp(0.0, 100.0),
        }
    }

    pub fn binding_tendency(&self, other: &ElementDefinition) -> BindingTendency {
        if self.chemical_family() == ChemicalFamily::NobleGas
            || other.chemical_family() == ChemicalFamily::NobleGas
        {
            return BindingTendency {
                kind: BondKind::Inert,
                strength: (self.reactivity_score() * other.reactivity_score()).sqrt() * 0.15,
                polarity: 0.0,
            };
        }

        let delta =
            (self.effective_electronegativity() - other.effective_electronegativity()).abs();
        let polarity = (delta / 3.28).clamp(0.0, 1.0);
        let both_metals = self.chemical_family().is_metal() && other.chemical_family().is_metal();
        let kind = if both_metals {
            BondKind::Metallic
        } else if delta >= 1.7 {
            BondKind::Ionic
        } else if delta >= 0.4 {
            BondKind::PolarCovalent
        } else {
            BondKind::NonpolarCovalent
        };
        let kind_factor = match kind {
            BondKind::Ionic => 1.0,
            BondKind::PolarCovalent => 0.82,
            BondKind::NonpolarCovalent => 0.68,
            BondKind::Metallic => 0.62,
            BondKind::Inert => 0.15,
        };
        let reactivity = (self.reactivity_score() * other.reactivity_score())
            .sqrt()
            .clamp(0.0, 1.0);

        BindingTendency {
            kind,
            strength: (reactivity * (0.45 + polarity * 0.55) * kind_factor).clamp(0.0, 1.0),
            polarity,
        }
    }

    pub fn is_complementary_pair(&self, other: &ElementDefinition) -> bool {
        self.binding_tendency(other).kind == BondKind::Ionic
            && self.chemical_family().is_metal() != other.chemical_family().is_metal()
    }

    /// Returns true for elements that commonly form stable diatomic molecules
    /// under standard conditions (H, N, O, F, Cl, Br, I).
    pub fn forms_diatomic_molecule(&self) -> bool {
        matches!(self.atomic_number, 1 | 7 | 8 | 9 | 17 | 35 | 53)
    }

    /// Covalent radius in picometres. Values for short-lived heavy elements are
    /// bounded theoretical estimates and are used only for relative body volume.
    pub fn atomic_radius_pm(&self) -> f32 {
        const PERIOD_1: [f32; 2] = [31.0, 28.0];
        const PERIOD_2: [f32; 8] = [128.0, 96.0, 84.0, 76.0, 71.0, 66.0, 57.0, 58.0];
        const PERIOD_3: [f32; 8] = [166.0, 141.0, 121.0, 111.0, 107.0, 105.0, 102.0, 106.0];
        const PERIOD_4: [f32; 18] = [
            203.0, 176.0, 170.0, 160.0, 153.0, 139.0, 139.0, 132.0, 126.0, 124.0, 132.0, 122.0,
            122.0, 120.0, 119.0, 120.0, 120.0, 116.0,
        ];
        const PERIOD_5: [f32; 18] = [
            220.0, 195.0, 190.0, 175.0, 164.0, 154.0, 147.0, 146.0, 142.0, 139.0, 145.0, 144.0,
            142.0, 139.0, 139.0, 138.0, 139.0, 140.0,
        ];
        const PERIOD_6: [f32; 32] = [
            244.0, 215.0, 207.0, 204.0, 203.0, 201.0, 199.0, 198.0, 198.0, 196.0, 194.0, 192.0,
            192.0, 189.0, 190.0, 187.0, 187.0, 175.0, 170.0, 162.0, 151.0, 144.0, 141.0, 136.0,
            136.0, 132.0, 145.0, 146.0, 148.0, 140.0, 150.0, 150.0,
        ];
        const PERIOD_7: [f32; 32] = [
            260.0, 221.0, 215.0, 206.0, 200.0, 196.0, 190.0, 187.0, 180.0, 169.0, 168.0, 168.0,
            165.0, 167.0, 173.0, 176.0, 161.0, 157.0, 149.0, 143.0, 141.0, 134.0, 129.0, 128.0,
            121.0, 122.0, 136.0, 143.0, 162.0, 175.0, 165.0, 157.0,
        ];

        match self.atomic_number {
            1..=2 => PERIOD_1[(self.atomic_number - 1) as usize],
            3..=10 => PERIOD_2[(self.atomic_number - 3) as usize],
            11..=18 => PERIOD_3[(self.atomic_number - 11) as usize],
            19..=36 => PERIOD_4[(self.atomic_number - 19) as usize],
            37..=54 => PERIOD_5[(self.atomic_number - 37) as usize],
            55..=86 => PERIOD_6[(self.atomic_number - 55) as usize],
            87..=118 => PERIOD_7[(self.atomic_number - 87) as usize],
            _ => 100.0,
        }
    }

    pub fn atomic_radius_scale(&self) -> f32 {
        (self.atomic_radius_pm() / 100.0).clamp(0.28, 2.60)
    }

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

    pub fn period(&self) -> u8 {
        match self.atomic_number {
            1..=2 => 1,
            3..=10 => 2,
            11..=18 => 3,
            19..=36 => 4,
            37..=54 => 5,
            55..=86 => 6,
            _ => 7,
        }
    }

    pub fn mass_dimension(&self) -> f32 {
        (self.prime_dimension() as f32).ln().max(1.0)
    }

    pub fn prime_stability_frequency(&self) -> f32 {
        1.0 / (self.prime_gap().max(1) as f32)
    }

    pub fn size_dimension(&self) -> f32 {
        self.atomic_radius_scale() * (1.0 + self.prime_dimension_scale() * 0.02)
    }

    pub fn relative_velocity_scale(&self) -> f32 {
        1.0 + self.prime_stability_frequency() * 0.12
            + self.reactivity_score() * 0.08
            + (self.period() as f32 - 1.0) * 0.02
    }

    pub fn prime_dimension_scale(&self) -> f32 {
        (self.prime_dimension() as f32).ln().max(1.0) * 0.012
            + self.prime_stability_frequency() * 0.08
    }

    pub fn orbital_velocity_scale(&self) -> f32 {
        (1.0 + self.mass_dimension() * 0.02
            + self.prime_stability_frequency() * 0.06
            + self.prime_dimension_scale() * 0.04)
            .clamp(1.0, 1.45)
    }

    pub fn bulge_density_factor(&self) -> f32 {
        (1.0 + self.prime_stability_frequency() * 0.18 + (self.period() as f32 - 1.0) * 0.04)
            .clamp(1.0, 1.6)
    }

    pub fn element_radius_scale(&self, base_radius_scale: f32) -> f32 {
        base_radius_scale * self.size_dimension() * (1.0 + self.prime_dimension_scale() * 0.03)
    }

    pub fn smbh_mass_fraction(&self) -> f32 {
        (0.42 + self.mass_dimension() * 0.03 + (self.period() as f32 - 1.0) * 0.008)
            .clamp(0.35, 0.62)
    }

    pub fn bulge_mass_multiplier(&self) -> f32 {
        (0.95 + self.prime_stability_frequency() * 0.18 + (self.period() as f32 - 1.0) * 0.02)
            .clamp(0.9, 1.4)
    }

    pub fn halo_mass_multiplier(&self) -> f32 {
        (0.92 + (1.0 / (1.0 + self.mass_dimension())) * 0.16).clamp(0.9, 1.15)
    }

    pub fn disk_orbital_velocity_factor(&self) -> f32 {
        (1.0 + self.mass_dimension() * 0.02 + self.prime_stability_frequency() * 0.08)
            .clamp(1.0, 1.35)
    }

    pub fn gas_dispersion_factor(&self) -> f32 {
        (0.7 + self.prime_stability_frequency() * 0.15 + (self.period() as f32 / 7.0) * 0.08)
            .clamp(0.7, 1.2)
    }

    pub fn halo_binding_factor(&self) -> f32 {
        (0.48 + self.mass_dimension() * 0.02 + self.prime_stability_frequency() * 0.02)
            .clamp(0.5, 0.92)
    }

    pub fn light_signature(&self, light_energy: f32) -> f32 {
        light_energy * self.mass_dimension() * (0.85 + self.effective_electronegativity() * 0.075)
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
        let fraction = self.smbh_mass_fraction();
        total_mass * fraction
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

    pub fn bulge_particle_count(&self) -> usize {
        let base = match self.galaxy_morphology() {
            GalaxyMorphology::Dwarf => 6,
            GalaxyMorphology::Spiral => 10,
            GalaxyMorphology::Elliptical => 14,
            GalaxyMorphology::AGN => 18,
        };
        base + (self.period() as usize).saturating_sub(1)
    }

    pub fn gas_particle_count(&self) -> usize {
        (self.disk_particle_count() / 4).max(3)
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
        let prime_gap = self.prime_gap().max(1) as f32;
        beta * self.mass_dimension() / prime_gap
    }

    /// Number of valence electrons based on group and family.
    /// This is a simplified periodic-table estimate used for bond-valency rules.
    pub fn valence_electrons(&self) -> u8 {
        match self.chemical_family() {
            ChemicalFamily::AlkaliMetal => 1,
            ChemicalFamily::AlkalineEarthMetal => 2,
            ChemicalFamily::TransitionMetal | ChemicalFamily::PostTransitionMetal => {
                // For the p-block/post-transition metals, group number minus 10 gives
                // a reasonable valence estimate; for transition metals we keep a low
                // default because their chemistry is more variable.
                match self.group() {
                    3..=12 => 2,
                    g => g.saturating_sub(10).max(1),
                }
            }
            ChemicalFamily::Metalloid => match self.group() {
                13 => 3,
                14 => 4,
                15 => 5,
                16 => 6,
                _ => self.group().saturating_sub(10).max(1),
            },
            ChemicalFamily::ReactiveNonmetal => match self.atomic_number {
                1 => 1,
                _ => match self.group() {
                    14 => 4,
                    15 => 5,
                    16 => 6,
                    _ => self.group().saturating_sub(10).max(1),
                },
            },
            ChemicalFamily::Halogen => 7,
            ChemicalFamily::NobleGas => 8,
            ChemicalFamily::Lanthanide | ChemicalFamily::Actinide => 3,
        }
    }

    /// Typical oxidation states used for simple stoichiometric reactions.
    /// Returns a small set of the most common/stable states.
    pub fn typical_oxidation_states(&self) -> &'static [i8] {
        match self.chemical_family() {
            ChemicalFamily::AlkaliMetal => &[1],
            ChemicalFamily::AlkalineEarthMetal => &[2],
            ChemicalFamily::Halogen => &[-1],
            ChemicalFamily::NobleGas => &[0],
            ChemicalFamily::ReactiveNonmetal => match self.atomic_number {
                1 => &[-1, 1],
                6 => &[-4, -2, 2, 4],
                7 => &[-3, 2, 3, 4, 5],
                8 => &[-2, -1],
                15 => &[-3, 3, 5],
                16 => &[-2, 2, 4, 6],
                34 => &[-2, 4, 6],
                _ => &[-2, 2],
            },
            ChemicalFamily::Metalloid => match self.atomic_number {
                5 => &[3],
                14 => &[-4, 4],
                32 => &[2, 4],
                33 => &[-3, 3, 5],
                51 => &[-3, 3, 5],
                52 => &[-2, 2, 4, 6],
                _ => &[3, 5],
            },
            ChemicalFamily::TransitionMetal => match self.atomic_number {
                22 | 40 | 72 => &[2, 3, 4],
                24 | 42 => &[2, 3, 6],
                25 => &[2, 3, 4, 7],
                26 => &[2, 3],
                27 => &[2, 3],
                28 => &[2, 3],
                29 => &[1, 2],
                30 => &[2],
                _ => &[2, 3],
            },
            ChemicalFamily::PostTransitionMetal => match self.group() {
                13 => &[3],
                14 => &[2, 4],
                15 => &[-3, 3, 5],
                16 => &[-2, 2, 4],
                _ => &[2, 3],
            },
            ChemicalFamily::Lanthanide | ChemicalFamily::Actinide => &[3],
        }
    }

    /// Selects the configured gravity model for this element.
    pub fn effective_gravitation(
        &self,
        beta: f32,
        use_scientific: bool,
        weights: (f32, f32, f32, f32),
    ) -> f32 {
        if use_scientific {
            self.scientific_gravitation(beta, weights.0, weights.1, weights.2, weights.3)
        } else {
            self.gravitation(beta)
        }
    }

    /// A scientifically consistent gravity analogy for individual elements.
    ///
    /// Real gravity depends only on mass, but in this analog model the
    /// "effective gravitational pull" of an element is modulated by properties
    /// that correlate with how strongly it interacts with other matter in a
    /// chemical/astronomical analogy:
    /// - mass: heavier elements have stronger attraction
    /// - electronegativity: stronger electron affinity increases local pull
    /// - atomic radius: larger radius dilutes the effective pull
    /// - reactivity: more reactive elements participate more strongly
    ///
    /// The result is normalized so that hydrogen produces approximately `beta`.
    pub fn scientific_gravitation(
        &self,
        beta: f32,
        mass_weight: f32,
        electronegativity_weight: f32,
        radius_weight: f32,
        reactivity_weight: f32,
    ) -> f32 {
        let hydrogen = element_by_atomic_number(1).expect("hydrogen must exist");
        let mass_ratio = self.atomic_weight / hydrogen.atomic_weight;
        let en_ratio = self.effective_electronegativity() / hydrogen.effective_electronegativity();
        let radius_ratio = self.atomic_radius_pm() / hydrogen.atomic_radius_pm();
        let reactivity = self.reactivity_score();

        let mass_term = mass_ratio.ln_1p();
        let en_term = en_ratio.ln_1p();
        let radius_term = (1.0 / radius_ratio.max(1e-3)).ln_1p();
        let reactivity_term = reactivity;

        let total_weight = mass_weight + electronegativity_weight + radius_weight + reactivity_weight;
        let normalized_weight = total_weight.max(1e-6);

        let raw = (mass_weight * mass_term
            + electronegativity_weight * en_term
            + radius_weight * radius_term
            + reactivity_weight * reactivity_term)
            / normalized_weight;

        // Compute hydrogen's raw value with the same formula to normalize the scale.
        let h_mass_ratio = hydrogen.atomic_weight / hydrogen.atomic_weight;
        let h_en_ratio = hydrogen.effective_electronegativity() / hydrogen.effective_electronegativity();
        let h_radius_ratio = hydrogen.atomic_radius_pm() / hydrogen.atomic_radius_pm();
        let h_reactivity = hydrogen.reactivity_score();
        let h_raw = (mass_weight * h_mass_ratio.ln_1p()
            + electronegativity_weight * h_en_ratio.ln_1p()
            + radius_weight * (1.0 / h_radius_ratio).ln_1p()
            + reactivity_weight * h_reactivity)
            / normalized_weight;

        let scale = beta / h_raw.max(1e-6);
        raw * scale
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
        assert_eq!(hydrogen.period(), 1);

        let helium = element_by_atomic_number(2).unwrap();
        assert_eq!(helium.prime_dimension(), 5);
        assert_eq!(helium.prime_gap(), 2);
        assert_eq!(helium.period(), 1);

        let neon = element_by_atomic_number(10).unwrap();
        assert_eq!(neon.period(), 2);
        assert!(neon.element_radius_scale(1.0) > 0.0);
        assert!(neon.relative_velocity_scale() > 1.0);

        let oganesson = element_by_atomic_number(118).unwrap();
        assert_eq!(oganesson.prime_dimension(), 653);
        assert_eq!(oganesson.prime_gap(), 6);
        assert_eq!(oganesson.period(), 7);
    }

    #[test]
    fn galactic_atom_system_core_includes_smbh_and_bulge_cluster() {
        let carbon = element_by_atomic_number(6).unwrap();
        let bodies = generate_galactic_atom_system(
            carbon,
            Vec3::zero(),
            Vec3::new(0.0, 0.0, 1.0),
            1.0,
            1.0,
            1.0,
            0.1,
            0.1,
            1.0,
            (0.5, 2.0),
        );
        let core_count = bodies
            .iter()
            .filter(|b| b.segment_type == crate::body::ParticleSegmentType::Core)
            .count();
        let bulge_count = bodies
            .iter()
            .filter(|b| b.segment_type == crate::body::ParticleSegmentType::Bulge)
            .count();
        assert_eq!(core_count, 1);
        assert_eq!(bulge_count, carbon.bulge_particle_count());
        assert!(
            bodies.len() > 6,
            "expected a full galactic atom system with disk and halo bodies"
        );
    }

    #[test]
    fn galactic_atom_system_shell_hierarchy_is_preserved() {
        let carbon = element_by_atomic_number(6).unwrap();
        let bodies = generate_galactic_atom_system(
            carbon,
            Vec3::zero(),
            Vec3::new(0.0, 0.0, 1.0),
            1.0,
            1.0,
            1.0,
            0.1,
            0.1,
            1.0,
            (0.5, 2.0),
        );

        let core = bodies
            .iter()
            .find(|b| b.segment_type == crate::body::ParticleSegmentType::Core)
            .expect("core body missing");
        let center = core.pos;

        let average_radius = |segment| {
            let iter = bodies.iter().filter(|b| b.segment_type == segment);
            let coords: Vec<f32> = iter.map(|b| (b.pos - center).mag()).collect();
            assert!(
                !coords.is_empty(),
                "no bodies found for segment {:?}",
                segment
            );
            coords.iter().sum::<f32>() / coords.len() as f32
        };

        let gas_radius = average_radius(crate::body::ParticleSegmentType::Gas);
        let disk_radius = average_radius(crate::body::ParticleSegmentType::Orbital);
        let halo_radius = average_radius(crate::body::ParticleSegmentType::Satellite);

        assert!(gas_radius < disk_radius, "gas should be inner to disk");
        assert!(
            disk_radius < halo_radius,
            "disk should be inner to halo/satellites"
        );
    }

    #[test]
    fn hydrogen_galactic_atom_system_uses_at_least_400_bodies() {
        let hydrogen = element_by_atomic_number(1).unwrap();
        let bodies = generate_galactic_atom_system(
            hydrogen,
            Vec3::zero(),
            Vec3::new(0.0, 0.0, 1.0),
            1.0,
            1.0,
            1.0,
            0.1,
            0.1,
            1.0,
            (0.5, 2.0),
        );

        assert!(
            bodies.len() >= 400,
            "hydrogen should generate at least 400 element bodies"
        );
    }

    #[test]
    fn all_element_templates_have_consistent_chemistry_properties() {
        assert_eq!(elements().len(), 118);
        let mut color_hues = std::collections::HashSet::new();
        let mut reactivity_values = std::collections::HashSet::new();

        for (index, element) in elements().iter().enumerate() {
            assert_eq!(element.atomic_number as usize, index + 1);
            assert!((1..=18).contains(&element.group()));
            assert!((1..=7).contains(&element.period()));
            assert!(element.atomic_weight.is_finite() && element.atomic_weight > 0.0);
            assert!(element.atomic_radius_pm().is_finite() && element.atomic_radius_pm() > 0.0);
            assert!((0.70..=3.98).contains(&element.effective_electronegativity()));
            assert!((0.0..=1.0).contains(&element.reactivity_score()));

            let color = element.element_color();
            assert!((0.0..360.0).contains(&color.hue_degrees));
            assert!((0.0..=100.0).contains(&color.saturation));
            assert!((0.0..=100.0).contains(&color.lightness));
            color_hues.insert(color.hue_degrees.to_bits());
            reactivity_values.insert(element.reactivity_score().to_bits());
        }

        assert_eq!(color_hues.len(), 118);
        assert_eq!(reactivity_values.len(), 118);
    }

    #[test]
    fn chemistry_extremes_match_periodic_trends() {
        let fluorine = element_by_atomic_number(9).unwrap();
        let oxygen = element_by_atomic_number(8).unwrap();
        let caesium = element_by_atomic_number(55).unwrap();
        let francium = element_by_atomic_number(87).unwrap();
        let helium = element_by_atomic_number(2).unwrap();
        let lithium = element_by_atomic_number(3).unwrap();
        let boron = element_by_atomic_number(5).unwrap();
        let iron = element_by_atomic_number(26).unwrap();
        let oganesson = element_by_atomic_number(118).unwrap();

        assert_eq!(
            fluorine.electronegativity_category(),
            ElectronegativityCategory::StrongAcceptor
        );
        assert!(fluorine.effective_electronegativity() > oxygen.effective_electronegativity());
        assert!(caesium.atomic_radius_pm() > oxygen.atomic_radius_pm());
        assert!(francium.reactivity_score() > 0.9);
        assert_eq!(helium.chemical_family(), ChemicalFamily::NobleGas);
        assert!(helium.reactivity_score() < 0.05);
        assert_eq!(lithium.group(), 1);
        assert_eq!(boron.group(), 13);
        assert_eq!(iron.group(), 8);
        assert_eq!(oganesson.group(), 18);
    }

    #[test]
    fn sodium_chloride_is_a_complementary_ionic_pair() {
        let sodium = element_by_atomic_number(11).unwrap();
        let chlorine = element_by_atomic_number(17).unwrap();
        let helium = element_by_atomic_number(2).unwrap();

        let ionic = sodium.binding_tendency(chlorine);
        assert_eq!(ionic.kind, BondKind::Ionic);
        assert!(ionic.polarity > 0.6);
        assert!(sodium.is_complementary_pair(chlorine));
        assert!(
            ionic.strength > sodium.binding_tendency(helium).strength,
            "an inert noble-gas pairing must be weaker"
        );
    }

    #[test]
    fn valence_electrons_follow_periodic_trends() {
        let hydrogen = element_by_atomic_number(1).unwrap();
        let lithium = element_by_atomic_number(3).unwrap();
        let carbon = element_by_atomic_number(6).unwrap();
        let oxygen = element_by_atomic_number(8).unwrap();
        let chlorine = element_by_atomic_number(17).unwrap();
        let argon = element_by_atomic_number(18).unwrap();
        let iron = element_by_atomic_number(26).unwrap();

        assert_eq!(hydrogen.valence_electrons(), 1);
        assert_eq!(lithium.valence_electrons(), 1);
        assert_eq!(carbon.valence_electrons(), 4);
        assert_eq!(oxygen.valence_electrons(), 6);
        assert_eq!(chlorine.valence_electrons(), 7);
        assert_eq!(argon.valence_electrons(), 8);
        assert_eq!(iron.valence_electrons(), 2);
    }

    #[test]
    fn typical_oxidation_states_match_chemical_family() {
        let sodium = element_by_atomic_number(11).unwrap();
        let calcium = element_by_atomic_number(20).unwrap();
        let chlorine = element_by_atomic_number(17).unwrap();
        let oxygen = element_by_atomic_number(8).unwrap();
        let iron = element_by_atomic_number(26).unwrap();
        let argon = element_by_atomic_number(18).unwrap();

        assert!(sodium.typical_oxidation_states().contains(&1));
        assert!(calcium.typical_oxidation_states().contains(&2));
        assert!(chlorine.typical_oxidation_states().contains(&-1));
        assert!(oxygen.typical_oxidation_states().contains(&-2));
        assert!(iron.typical_oxidation_states().contains(&2));
        assert!(argon.typical_oxidation_states().contains(&0));
    }

    #[test]
    fn scientific_gravitation_follows_mass_trend() {
        let hydrogen = element_by_atomic_number(1).unwrap();
        let helium = element_by_atomic_number(2).unwrap();
        let oxygen = element_by_atomic_number(8).unwrap();
        let iron = element_by_atomic_number(26).unwrap();
        let uranium = element_by_atomic_number(92).unwrap();

        let g_h = hydrogen.scientific_gravitation(1.0, 1.0, 0.4, 0.3, 0.2);
        let g_he = helium.scientific_gravitation(1.0, 1.0, 0.4, 0.3, 0.2);
        let g_o = oxygen.scientific_gravitation(1.0, 1.0, 0.4, 0.3, 0.2);
        let g_fe = iron.scientific_gravitation(1.0, 1.0, 0.4, 0.3, 0.2);
        let g_u = uranium.scientific_gravitation(1.0, 1.0, 0.4, 0.3, 0.2);

        assert!(
            g_h < g_he && g_he < g_o && g_o < g_fe && g_fe < g_u,
            "scientific gravity should increase with atomic weight"
        );
    }

    #[test]
    fn effective_gravitation_selects_model_based_on_config_flag() {
        let hydrogen = element_by_atomic_number(1).unwrap();
        let oxygen = element_by_atomic_number(8).unwrap();

        let classic_h = hydrogen.effective_gravitation(1.0, false, (1.0, 0.4, 0.3, 0.2));
        let classic_o = oxygen.effective_gravitation(1.0, false, (1.0, 0.4, 0.3, 0.2));
        assert_eq!(classic_h, hydrogen.gravitation(1.0));
        assert_eq!(classic_o, oxygen.gravitation(1.0));

        let scientific_h = hydrogen.effective_gravitation(1.0, true, (1.0, 0.4, 0.3, 0.2));
        let scientific_o = oxygen.effective_gravitation(1.0, true, (1.0, 0.4, 0.3, 0.2));
        assert_ne!(scientific_h, hydrogen.gravitation(1.0));
        assert_ne!(scientific_o, oxygen.gravitation(1.0));
        assert!(scientific_h > 0.0);
        assert!(scientific_o > scientific_h);
    }

    #[test]
    fn diatomic_elements_are_recognized() {
        assert!(element_by_atomic_number(1).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(7).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(8).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(9).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(17).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(35).unwrap().forms_diatomic_molecule());
        assert!(element_by_atomic_number(53).unwrap().forms_diatomic_molecule());

        assert!(!element_by_atomic_number(2).unwrap().forms_diatomic_molecule());
        assert!(!element_by_atomic_number(6).unwrap().forms_diatomic_molecule());
        assert!(!element_by_atomic_number(26).unwrap().forms_diatomic_molecule());
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
    particle_mass_range: (f32, f32),
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
    let bulge_mass = orbit_mass * element.bulge_mass_fraction() * element.bulge_mass_multiplier();
    let halo_mass = orbit_mass * element.halo_mass_fraction() * element.halo_mass_multiplier();
    let mut disk_mass = (orbit_mass - bulge_mass - halo_mass).max(orbit_mass * 0.05);
    let mut disk_count = element.disk_particle_count();
    let halo_count = element.halo_particle_count();
    let bulge_count = element.bulge_particle_count();
    let mut gas_count = element.gas_particle_count();
    let morphology = element.galaxy_morphology();
    let (mass_min, mass_max) = particle_mass_range;
    let disk_scatter = (mass_max - mass_min) * 0.24;
    let gas_scatter = (mass_max - mass_min) * 0.18;
    let halo_scatter = (mass_max - mass_min) * 0.20;
    let bulge_scatter = (mass_max - mass_min) * 0.35;

    let min_element_particle_count = if element.atomic_number == 1 { 400 } else { 100 };
    let mut current_total_particle_count = 1 + bulge_count + gas_count + disk_count + halo_count;
    while current_total_particle_count < min_element_particle_count {
        disk_count = disk_count.saturating_add(1);
        gas_count = (disk_count / 4).max(3);
        current_total_particle_count = 1 + bulge_count + gas_count + disk_count + halo_count;
    }
    let prime_dimension = element.prime_dimension();
    let mass_dimension = element.mass_dimension();
    let reactivity = element.reactivity_score();
    let magnetism = element.magnetism(alpha);
    let gravitation = element.gravitation(beta);
    let light_interaction = element.light_signature(light_energy);

    let rotation_axis = if rotation_axis == Vec3::zero() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        rotation_axis.normalized()
    };

    let element_radius_scale = element.element_radius_scale(radius_scale);
    let velocity_scale = element.relative_velocity_scale()
        * element.disk_orbital_velocity_factor()
        * element.orbital_velocity_scale();

    let mut bodies = Vec::with_capacity(1 + bulge_count + gas_count + disk_count + halo_count);
    let center_radius = (smbh_mass.max(0.1)).cbrt() * 0.6 * element_radius_scale;

    bodies.push(Body::new_element(
        center,
        Vec3::zero(),
        smbh_mass,
        center_radius,
        morphology.central_spin(),
        rotation_axis,
        element.atomic_number,
        crate::body::ParticleSegmentType::Core,
        prime_dimension as f32,
        element.core_reactivity_score(),
        magnetism,
        gravitation,
        light_interaction,
        mass_dimension,
    ));

    let bulge_body_mass = (bulge_mass / bulge_count as f32).max(0.0004);
    let bulge_mass_min = (bulge_body_mass * 0.65).max(mass_min * 0.4);
    let bulge_mass_max = (bulge_body_mass * 1.25).max(bulge_mass_min);
    let bulge_body_mass = bulge_body_mass.clamp(bulge_mass_min, bulge_mass_max);
    let bulge_body_radius = (bulge_body_mass.cbrt() * 0.85 + 0.06) * element_radius_scale * 0.7;
    let bulge_cloud_radius =
        center_radius * (1.6 + 0.28 / element.bulge_density_factor()) + bulge_body_radius * 1.1;

    for i in 0..bulge_count {
        let fraction = (i as f32 + 0.5) / bulge_count as f32;
        let theta = fastrand::f32() * std::f32::consts::PI;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let mut direction = Vec3::new(
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        )
        .normalized();
        let flatten = 0.5 + 0.15 * (element.period() as f32 - 1.0);
        direction.z *= flatten;
        direction = direction.normalized();

        let radius = bulge_cloud_radius * (0.12 + 0.32 * fraction.sqrt());
        let position = center + direction * radius;
        let offset = position - center;

        let tangent = if offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(offset).normalized()
        } else {
            direction.cross(rotation_axis).normalized()
        };
        let rotation_bias =
            0.24 + 0.18 * (element.period() as f32 / 7.0) + element.bulge_density_factor() * 0.05;
        let orbital_speed =
            ((smbh_mass / offset.mag().max(1.0)).max(0.01)).sqrt() * velocity_scale * rotation_bias;

        let random_dispersion = Vec3::new(
            (fastrand::f32() - 0.5) * 0.3,
            (fastrand::f32() - 0.5) * 0.3,
            (fastrand::f32() - 0.5) * 0.25,
        );
        let velocity = tangent * orbital_speed
            + random_dispersion * orbital_speed * 0.6
            + rotation_axis * ((fastrand::f32() - 0.5) * 0.015);

        let mass =
            (bulge_body_mass + (fastrand::f32() - 0.5) * bulge_scatter * 1.2).max(bulge_mass_min);
        let radius = (mass.cbrt() * 0.95 + 0.08) * element_radius_scale * 0.75;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            morphology.bulge_spin() * 1.25 + element.prime_stability_frequency() * 0.1,
            rotation_axis,
            element.atomic_number,
            crate::body::ParticleSegmentType::Bulge,
            prime_dimension as f32,
            reactivity,
            magnetism * 1.1,
            gravitation,
            light_interaction,
            mass_dimension,
        ));
    }

    let (u, v) = plane_basis(rotation_axis);
    let gas_mass = disk_mass * 0.16 * element.gas_dispersion_factor();
    disk_mass -= gas_mass;

    let mut accumulated_mass = smbh_mass + bulge_mass;
    let disk_radius = element_radius_scale * element.disk_radius_factor();

    for i in 0..gas_count {
        let fraction = (i as f32 + 0.5) / gas_count as f32;
        let angle = fraction * std::f32::consts::TAU + fastrand::f32() * 0.45;
        let cos = angle.cos();
        let sin = angle.sin();
        let radius = disk_radius * (0.10 + 0.35 * fraction.sqrt());
        let z = (fastrand::f32() - 0.5) * disk_radius * 0.12;
        let in_plane_offset = u * (cos * radius) + v * (sin * radius);
        let position = center + in_plane_offset + rotation_axis * z;
        let tangent = if in_plane_offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(in_plane_offset).normalized()
        } else {
            (u * -sin + v * cos).normalized()
        };
        let orbital_speed = ((accumulated_mass / radius.max(1.0)).max(0.01)).sqrt()
            * velocity_scale
            * element.gas_dispersion_factor()
            * (1.0 + element.prime_dimension_scale() * 0.02);
        let velocity = tangent * orbital_speed + rotation_axis * ((fastrand::f32() - 0.5) * 0.03);
        let base_mass = (gas_mass / gas_count as f32).max(0.00015);
        let mass = (base_mass + (fastrand::f32() - 0.5) * gas_scatter * 0.9).max(mass_min * 0.12);
        let radius = (mass.cbrt() * 0.55 + 0.025) * element_radius_scale * 0.35;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            orbital_speed * 0.08 * morphology.disk_spin_scale() * velocity_scale,
            rotation_axis,
            element.atomic_number,
            crate::body::ParticleSegmentType::Gas,
            prime_dimension as f32,
            reactivity,
            magnetism * 0.72,
            gravitation,
            light_interaction,
            mass_dimension,
        ));
        accumulated_mass += mass;
    }

    for i in 0..disk_count {
        let fraction = (i as f32 + 0.5) / disk_count as f32;
        let angle = fraction * std::f32::consts::TAU + fastrand::f32() * 0.18;
        let cos = angle.cos();
        let sin = angle.sin();
        let radius = disk_radius * (0.30 + 0.55 * fraction.sqrt());
        let z = (fastrand::f32() - 0.5) * disk_radius * 0.03;
        let in_plane_offset = u * (cos * radius) + v * (sin * radius);
        let position = center + in_plane_offset + rotation_axis * z;
        let tangent = if in_plane_offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(in_plane_offset).normalized()
        } else {
            (u * -sin + v * cos).normalized()
        };
        let orbital_speed = ((accumulated_mass / radius.max(1.0)).max(0.01)).sqrt()
            * velocity_scale
            * (1.0 + element.prime_stability_frequency() * 0.06)
            * (1.0 + element.prime_dimension_scale() * 0.015);
        let velocity = tangent * orbital_speed;
        let base_mass = (disk_mass / disk_count as f32).max(0.0005);
        let mass = (base_mass + (fastrand::f32() - 0.5) * disk_scatter * 0.8).max(mass_min * 0.25);
        let radius = (mass.cbrt() * 0.8 + 0.05) * element_radius_scale * 0.6;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            orbital_speed * 0.15 * morphology.disk_spin_scale() * velocity_scale,
            rotation_axis,
            element.atomic_number,
            crate::body::ParticleSegmentType::Orbital,
            prime_dimension as f32,
            reactivity,
            magnetism,
            gravitation,
            light_interaction,
            mass_dimension,
        ));
        accumulated_mass += mass;
    }

    let halo_radius = disk_radius * 2.1;
    for _i in 0..halo_count {
        let radius_factor = 0.95 + fastrand::f32() * 0.35;
        let angle = fastrand::f32() * std::f32::consts::TAU;
        let inclination = (fastrand::f32() * 0.45).acos();
        let sin_i = inclination.sin();
        let cos_i = inclination.cos();
        let offset = u * (angle.cos() * sin_i) + v * (angle.sin() * sin_i) + rotation_axis * cos_i;
        let position = center + offset.normalized() * halo_radius * radius_factor;
        let tangent = if offset.cross(rotation_axis).mag_sq() > 1e-8 {
            rotation_axis.cross(offset).normalized()
        } else {
            u
        };
        let orbital_speed =
            ((accumulated_mass / (halo_radius * radius_factor).max(1.0)).max(0.003)).sqrt()
                * velocity_scale
                * element.halo_binding_factor()
                * (1.0 + element.prime_dimension_scale() * 0.01);
        let velocity =
            tangent * orbital_speed * 0.45 + rotation_axis * ((fastrand::f32() - 0.5) * 0.03);
        let base_mass = (halo_mass / halo_count as f32).max(0.0002);
        let mass = (base_mass + (fastrand::f32() - 0.5) * halo_scatter * 0.9).max(mass_min * 0.15);
        let radius = (mass.cbrt() * 0.7 + 0.04) * element_radius_scale * 0.55;

        bodies.push(Body::new_element(
            position,
            velocity,
            mass,
            radius,
            orbital_speed * 0.08 * velocity_scale,
            rotation_axis,
            element.atomic_number,
            crate::body::ParticleSegmentType::Satellite,
            prime_dimension as f32,
            reactivity,
            magnetism * 0.9,
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
    particle_mass_range: (f32, f32),
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
        particle_mass_range,
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
