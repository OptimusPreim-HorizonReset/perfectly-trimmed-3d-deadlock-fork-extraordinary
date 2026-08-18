use std::env;
use std::fs;
use std::path::PathBuf;

use crate::elements;

#[derive(Debug, Clone)]
pub struct InformationsConfig {
    pub dt: f32,
    pub n: usize,
    pub theta: f32,
    pub epsilon: f32,
    pub enable_adaptive_theta: bool,
    pub enable_adaptive_multipole: bool,
    pub enable_mixed_precision: bool,
    pub enable_adaptive_timestep: bool,
    pub adaptive_theta_quiet: f32,
    pub adaptive_theta_medium: f32,
    pub adaptive_theta_critical: f32,
    pub adaptive_multipole_quiet: u8,
    pub adaptive_multipole_medium: u8,
    pub adaptive_multipole_critical: u8,
    pub adaptive_dynamics_quiet_threshold: f32,
    pub adaptive_dynamics_critical_threshold: f32,
    pub eta_timestep: f32,
    pub dt_min: f32,
    pub dt_max: f32,
    pub max_energy_error: f32,
    pub adaptive_feedback_theta_scale: f32,
    pub accretion_spawn_rate: f32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub outer_ring_spawn_zone_inner_ratio: f32,
    pub outer_ring_spawn_zone_outer_ratio: f32,
    pub central_mass: f32,
    pub particle_mass_range: (f32, f32),
    pub inflow_strength: f32,
    pub restore_strength: f32,
    pub galaxy_separation_factor: f32,
    /// Total number of galaxies to initialize (arranged as pairs; rounded down to an even number).
    pub galaxy_count: usize,
    /// Scales the overall scatter volume in which galaxy pairs are chaotically distributed
    /// (multiplied by pair separation and the cube root of pair count).
    pub galaxy_volume_scatter_factor: f32,
    pub prob_merge: f32,
    pub prob_repeated: f32,
    pub prob_flyby: f32,
    pub merge_speed_factor: f32,
    pub repeated_speed_factor: f32,
    pub flyby_speed_factor: f32,
    pub merge_angle: f32,
    pub repeated_angle: f32,
    pub flyby_angle: f32,
    pub collision_interval: usize,
    pub attract_interval: usize,
    pub orbital_speed_multiplier: f32,
    pub spawn_angular_speed_base: f32,
    pub spawn_angular_speed_range: f32,
    pub spin_speed_multiplier: f32,
    pub equatorial_growth_factor: f32,
    pub polar_flattening_factor: f32,
    pub radius_scale: f32,
    pub depth_scale_factor: f32,
    pub spacetime_dilation_factor: f32,
    pub enable_galactic_atom_simulation: bool,
    pub element_atomic_numbers: String,
    pub element_atomic_count: usize,
    pub element_max_atomic_number: u8,
    pub element_universe_scatter_factor: f32,
    pub element_center_mass_unit: f32,
    pub element_orbital_mass_unit: f32,
    pub element_radius_scale: f32,
    pub element_prime_alpha: f32,
    pub element_prime_beta: f32,
    pub element_light_energy: f32,
    pub enable_elemental_gravitation_tuning: bool,
    pub enable_elemental_mass_dimension_visualization: bool,
    pub element_initial_sorting_mode: u8,
    pub element_initial_grouping_mode: u8,
    pub element_group_spacing: f32,
    pub element_group_internal_velocity_scale: f32,
    pub element_sample_orbit_radius_scale: f32,
    pub element_sample_orbit_speed_factor: f32,
}

impl Default for InformationsConfig {
    fn default() -> Self {
        Self {
            dt: 100.0,
            n: 10000,
            theta: 0.50,
            epsilon: 1.7,
            enable_adaptive_theta: true,
            enable_adaptive_multipole: true,
            enable_mixed_precision: true,
            enable_adaptive_timestep: true,
            adaptive_theta_quiet: 1.0,
            adaptive_theta_medium: 0.5,
            adaptive_theta_critical: 0.25,
            adaptive_multipole_quiet: 1,
            adaptive_multipole_medium: 2,
            adaptive_multipole_critical: 3,
            adaptive_dynamics_quiet_threshold: 0.10,
            adaptive_dynamics_critical_threshold: 0.35,
            eta_timestep: 100.0,
            dt_min: 0.0,
            dt_max: 0.0,
            max_energy_error: 0.15,
            adaptive_feedback_theta_scale: 0.85,
            accretion_spawn_rate: 0.001,
            inner_radius: 1.0,
            outer_radius: 7.03,
            outer_ring_spawn_zone_inner_ratio: 0.32,
            outer_ring_spawn_zone_outer_ratio: 1.0,
            central_mass: 0.000002,
            particle_mass_range: (0.000000000000005, 0.0000000005),
            inflow_strength: 0.000000025,
            restore_strength: 0.00000000008,
            galaxy_separation_factor: 25.0,
            galaxy_count: 2,
            galaxy_volume_scatter_factor: 3.0,
            prob_merge: 0.35,
            prob_repeated: 0.35,
            prob_flyby: 0.30,
            merge_speed_factor: 0.10,
            repeated_speed_factor: 0.70,
            flyby_speed_factor: 0.90,
            merge_angle: 0.05,
            repeated_angle: 0.25,
            flyby_angle: 0.35,
            collision_interval: 1,
            attract_interval: 1,
            orbital_speed_multiplier: 0.025,
            spawn_angular_speed_base: 0.3,
            spawn_angular_speed_range: 0.5,
            spin_speed_multiplier: 24.0,
            equatorial_growth_factor: 0.18,
            polar_flattening_factor: 0.14,
            radius_scale: 0.5,
            depth_scale_factor: 0.002,
            spacetime_dilation_factor: 2.0,
            enable_galactic_atom_simulation: false,
            element_atomic_numbers: String::from("1"),
            element_atomic_count: 1,
            element_max_atomic_number: 118,
            element_universe_scatter_factor: 1.8,
            element_center_mass_unit: 1.0,
            element_orbital_mass_unit: 1.0,
            element_radius_scale: 5.0,
            element_prime_alpha: 0.09,
            element_prime_beta: 0.06,
            element_light_energy: 2.0,
            enable_elemental_gravitation_tuning: true,
            enable_elemental_mass_dimension_visualization: false,
            element_initial_sorting_mode: 0,
            element_initial_grouping_mode: 0,
            element_group_spacing: 0.65,
            element_group_internal_velocity_scale: 1.0,
            element_sample_orbit_radius_scale: 1.0,
            element_sample_orbit_speed_factor: 1.0,
        }
    }
}

impl InformationsConfig {
    pub fn load() -> Self {
        // Versuche, den absoluten Pfad zu bestimmen (zuerst im Arbeitsverzeichnis, dann relativ zur Binary)
        let mut possible_paths = vec![PathBuf::from("informations.md")];
        if let Ok(exe) = env::current_exe() {
            if let Some(parent) = exe.parent() {
                possible_paths.push(parent.join("informations.md"));
            }
        }

        let mut file_text = String::new();
        let mut file_found = false;

        for path in &possible_paths {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        file_text = content;
                        file_found = true;
                        eprintln!("✓ Config geladen: {}", path.display());
                        break;
                    }
                    Err(e) => {
                        eprintln!("✗ Fehler beim Lesen von {}: {}", path.display(), e);
                    }
                }
            }
        }

        if !file_found {
            eprintln!("⚠ informations.md nicht gefunden. Verwende Standardwerte.");
        }

        InformationsConfig::parse_text(&file_text)
    }
 
    fn parse_text(file_text: &str) -> Self {
        let mut config = InformationsConfig::default();
        let mut in_block = false;
        let mut parse_errors = 0;
 
        let parse_f32 = |value: &str| {
            value
                .trim()
                .replace(':', ".")
                .replace(';', ".")
                .parse::<f32>()
                .ok()
        };
        let parse_bool = |value: &str| match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        };
 
        for (line_num, line) in file_text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed == "```config" || trimmed.starts_with("```config ") {
                in_block = true;
                continue;
            }
            if in_block {
                if trimmed == "```" || trimmed.starts_with("``` ") {
                    break;
                }
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let pair = if trimmed.contains('=') {
                    trimmed.split_once('=')
                } else {
                    trimmed.split_once(':')
                };
                if let Some((key, value)) = pair {
                    let key = key.trim();
                    let value = value.trim();
                    let parse_result = match key {
                        "dt" => parse_f32(value).map(|v| {
                            config.dt = v;
                            true
                        }),
                        "n" => value.parse::<usize>().ok().map(|v| {
                            config.n = v;
                            true
                        }),
                        "theta" => parse_f32(value).map(|v| {
                            config.theta = v;
                            true
                        }),
                        "epsilon" => parse_f32(value).map(|v| {
                            config.epsilon = v;
                            true
                        }),
                        "enable_adaptive_theta" => parse_bool(value).map(|v| {
                            config.enable_adaptive_theta = v;
                            true
                        }),
                        "enable_adaptive_multipole" => parse_bool(value).map(|v| {
                            config.enable_adaptive_multipole = v;
                            true
                        }),
                        "enable_mixed_precision" => parse_bool(value).map(|v| {
                            config.enable_mixed_precision = v;
                            true
                        }),
                        "enable_adaptive_timestep" => parse_bool(value).map(|v| {
                            config.enable_adaptive_timestep = v;
                            true
                        }),
                        "adaptive_theta_quiet" => parse_f32(value).map(|v| {
                            config.adaptive_theta_quiet = v;
                            true
                        }),
                        "adaptive_theta_medium" => parse_f32(value).map(|v| {
                            config.adaptive_theta_medium = v;
                            true
                        }),
                        "adaptive_theta_critical" => parse_f32(value).map(|v| {
                            config.adaptive_theta_critical = v;
                            true
                        }),
                        "adaptive_multipole_quiet" => value.parse::<u8>().ok().map(|v| {
                            config.adaptive_multipole_quiet = v;
                            true
                        }),
                        "adaptive_multipole_medium" => value.parse::<u8>().ok().map(|v| {
                            config.adaptive_multipole_medium = v;
                            true
                        }),
                        "adaptive_multipole_critical" => value.parse::<u8>().ok().map(|v| {
                            config.adaptive_multipole_critical = v;
                            true
                        }),
                        "adaptive_dynamics_quiet_threshold" => parse_f32(value).map(|v| {
                            config.adaptive_dynamics_quiet_threshold = v;
                            true
                        }),
                        "adaptive_dynamics_critical_threshold" => parse_f32(value).map(|v| {
                            config.adaptive_dynamics_critical_threshold = v;
                            true
                        }),
                        "eta_timestep" => parse_f32(value).map(|v| {
                            config.eta_timestep = v;
                            true
                        }),
                        "dt_min" => parse_f32(value).map(|v| {
                            config.dt_min = v;
                            true
                        }),
                        "dt_max" => parse_f32(value).map(|v| {
                            config.dt_max = v;
                            true
                        }),
                        "max_energy_error" => parse_f32(value).map(|v| {
                            config.max_energy_error = v;
                            true
                        }),
                        "adaptive_feedback_theta_scale" => parse_f32(value).map(|v| {
                            config.adaptive_feedback_theta_scale = v;
                            true
                        }),
                        "accretion_spawn_rate" => parse_f32(value).map(|v| {
                            config.accretion_spawn_rate = v;
                            true
                        }),
                        "inner_radius" => parse_f32(value).map(|v| {
                            config.inner_radius = v;
                            true
                        }),
                        "outer_radius" => parse_f32(value).map(|v| {
                            config.outer_radius = v;
                            true
                        }),
                        "outer_ring_spawn_zone_inner_ratio" => parse_f32(value).map(|v| {
                            config.outer_ring_spawn_zone_inner_ratio = v;
                            true
                        }),
                        "outer_ring_spawn_zone_outer_ratio" => parse_f32(value).map(|v| {
                            config.outer_ring_spawn_zone_outer_ratio = v;
                            true
                        }),
                        "central_mass" => parse_f32(value).map(|v| {
                            config.central_mass = v;
                            true
                        }),
                        "particle_mass_range" => {
                            let parts: Vec<&str> = value.split(',').map(str::trim).collect();
                            if parts.len() == 2 {
                                if let (Some(min), Some(max)) =
                                    (parse_f32(parts[0]), parse_f32(parts[1]))
                                {
                                    config.particle_mass_range = (min, max);
                                    Some(true)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        "inflow_strength" => parse_f32(value).map(|v| {
                            config.inflow_strength = v;
                            true
                        }),
                        "restore_strength" => parse_f32(value).map(|v| {
                            config.restore_strength = v;
                            true
                        }),
                        "galaxy_separation_factor" => parse_f32(value).map(|v| {
                            config.galaxy_separation_factor = v;
                            true
                        }),
                        "galaxy_count" => value.parse::<usize>().ok().map(|v| {
                            config.galaxy_count = v;
                            true
                        }),
                        "galaxy_volume_scatter_factor" => parse_f32(value).map(|v| {
                            config.galaxy_volume_scatter_factor = v;
                            true
                        }),
                        "prob_merge" => parse_f32(value).map(|v| {
                            config.prob_merge = v;
                            true
                        }),
                        "prob_repeated" => parse_f32(value).map(|v| {
                            config.prob_repeated = v;
                            true
                        }),
                        "prob_flyby" => parse_f32(value).map(|v| {
                            config.prob_flyby = v;
                            true
                        }),
                        "merge_speed_factor" => parse_f32(value).map(|v| {
                            config.merge_speed_factor = v;
                            true
                        }),
                        "repeated_speed_factor" => parse_f32(value).map(|v| {
                            config.repeated_speed_factor = v;
                            true
                        }),
                        "flyby_speed_factor" => parse_f32(value).map(|v| {
                            config.flyby_speed_factor = v;
                            true
                        }),
                        "merge_angle" => parse_f32(value).map(|v| {
                            config.merge_angle = v;
                            true
                        }),
                        "repeated_angle" => parse_f32(value).map(|v| {
                            config.repeated_angle = v;
                            true
                        }),
                        "flyby_angle" => parse_f32(value).map(|v| {
                            config.flyby_angle = v;
                            true
                        }),
                        "collision_interval" => value.parse::<usize>().ok().map(|v| {
                            config.collision_interval = v;
                            true
                        }),
                        "attract_interval" => value.parse::<usize>().ok().map(|v| {
                            config.attract_interval = v;
                            true
                        }),
                        "orbital_speed_multiplier" => parse_f32(value).map(|v| {
                            config.orbital_speed_multiplier = v;
                            true
                        }),
                        "spawn_angular_speed_base" => parse_f32(value).map(|v| {
                            config.spawn_angular_speed_base = v;
                            true
                        }),
                        "spawn_angular_speed_range" => parse_f32(value).map(|v| {
                            config.spawn_angular_speed_range = v;
                            true
                        }),
                        "spin_speed_multiplier" => parse_f32(value).map(|v| {
                            config.spin_speed_multiplier = v;
                            true
                        }),
                        "equatorial_growth_factor" => parse_f32(value).map(|v| {
                            config.equatorial_growth_factor = v;
                            true
                        }),
                        "polar_flattening_factor" => parse_f32(value).map(|v| {
                            config.polar_flattening_factor = v;
                            true
                        }),
                        "radius_scale" => parse_f32(value).map(|v| {
                            config.radius_scale = v;
                            true
                        }),
                        "depth_scale_factor" => parse_f32(value).map(|v| {
                            config.depth_scale_factor = v;
                            true
                        }),
                        "spacetime_dilation_factor" => parse_f32(value).map(|v| {
                            config.spacetime_dilation_factor = v;
                            true
                        }),
                        "enable_elemental_simulation" => parse_bool(value).map(|v| {
                            config.enable_galactic_atom_simulation = v;
                            true
                        }),
                        "enable_galactic_atom_simulation" => parse_bool(value).map(|v| {
                            config.enable_galactic_atom_simulation = v;
                            true
                        }),
                        "element_atomic_numbers" => {
                            let normalized = value.trim().replace(' ', "");
                            config.element_atomic_numbers = normalized;
                            Some(true)
                        },
                        "element_atomic_count" => value.parse::<usize>().ok().map(|v| {
                            config.element_atomic_count = v;
                            true
                        }),
                        "element_max_atomic_number" => value.parse::<u8>().ok().map(|v| {
                            config.element_max_atomic_number = v;
                            true
                        }),
                        "element_universe_scatter_factor" => parse_f32(value).map(|v| {
                            config.element_universe_scatter_factor = v;
                            true
                        }),
                        "element_center_mass_unit" => parse_f32(value).map(|v| {
                            config.element_center_mass_unit = v;
                            true
                        }),
                        "element_orbital_mass_unit" => parse_f32(value).map(|v| {
                            config.element_orbital_mass_unit = v;
                            true
                        }),
                        "element_radius_scale" => parse_f32(value).map(|v| {
                            config.element_radius_scale = v;
                            true
                        }),
                        "element_prime_alpha" => parse_f32(value).map(|v| {
                            config.element_prime_alpha = v;
                            true
                        }),
                        "element_prime_beta" => parse_f32(value).map(|v| {
                            config.element_prime_beta = v;
                            true
                        }),
                        "element_light_energy" => parse_f32(value).map(|v| {
                            config.element_light_energy = v;
                            true
                        }),
                        "enable_elemental_gravitation_tuning" => parse_bool(value).map(|v| {
                            config.enable_elemental_gravitation_tuning = v;
                            true
                        }),
                        "enable_elemental_mass_dimension_visualization" => parse_bool(value).map(|v| {
                            config.enable_elemental_mass_dimension_visualization = v;
                            true
                        }),
                        "element_initial_sorting_mode" => value.parse::<u8>().ok().map(|v| {
                            config.element_initial_sorting_mode = v;
                            true
                        }),
                        "element_initial_grouping_mode" => value.parse::<u8>().ok().map(|v| {
                            config.element_initial_grouping_mode = v;
                            true
                        }),
                        "element_group_spacing" => parse_f32(value).map(|v| {
                            config.element_group_spacing = v;
                            true
                        }),
                        "element_group_internal_velocity_scale" => parse_f32(value).map(|v| {
                            config.element_group_internal_velocity_scale = v;
                            true
                        }),
                        "element_sample_orbit_radius_scale" => parse_f32(value).map(|v| {
                            config.element_sample_orbit_radius_scale = v;
                            true
                        }),
                        "element_sample_orbit_speed_factor" => parse_f32(value).map(|v| {
                            config.element_sample_orbit_speed_factor = v;
                            true
                        }),
                        _ => None,
                    };
 
                    if parse_result.is_none() {
                        eprintln!(
                            "⚠ Config Parse-Fehler Zeile {}: '{}' = '{}' (ungültiger Typ)",
                            line_num + 1,
                            key,
                            value
                        );
                        parse_errors += 1;
                    }
                }
            }
        }
 
        if parse_errors > 0 {
            eprintln!(
                "⚠ {} Parse-Fehler gefunden. Verwende Fallback-Werte für diese.",
                parse_errors
            );
        }
 
        config.adaptive_theta_quiet = config.adaptive_theta_quiet.max(0.05);
        config.adaptive_theta_medium = config.adaptive_theta_medium.max(0.05);
        config.adaptive_theta_critical = config.adaptive_theta_critical.max(0.05);
        config.adaptive_multipole_quiet = config.adaptive_multipole_quiet.clamp(1, 4);
        config.adaptive_multipole_medium = config.adaptive_multipole_medium.clamp(1, 4);
        config.adaptive_multipole_critical = config.adaptive_multipole_critical.clamp(1, 4);
        config.adaptive_dynamics_quiet_threshold =
            config.adaptive_dynamics_quiet_threshold.clamp(0.0, 1.0);
        config.adaptive_dynamics_critical_threshold = config
            .adaptive_dynamics_critical_threshold
            .max(config.adaptive_dynamics_quiet_threshold)
            .clamp(0.0, 1.0);
        config.eta_timestep = config.eta_timestep.max(1e-4);
        if !config.dt_min.is_finite() || config.dt_min <= 0.0 {
            config.dt_min = (config.dt.abs() * 0.1).max(1e-4);
        }
        if !config.dt_max.is_finite() || config.dt_max <= 0.0 {
            config.dt_max = config.dt.abs().max(config.dt_min);
        }
        if config.dt_max < config.dt_min {
            config.dt_max = config.dt_min;
        }
        config.max_energy_error = config.max_energy_error.max(0.0);
        config.adaptive_feedback_theta_scale = config.adaptive_feedback_theta_scale.clamp(0.1, 1.0);
        config.element_max_atomic_number = config.element_max_atomic_number.clamp(1, 118);
        if !config.element_universe_scatter_factor.is_finite()
            || config.element_universe_scatter_factor <= 0.0
        {
            config.element_universe_scatter_factor = 1.0;
        }
        if !config.element_center_mass_unit.is_finite() || config.element_center_mass_unit <= 0.0 {
            config.element_center_mass_unit = 1.0;
        }
        if !config.element_orbital_mass_unit.is_finite() || config.element_orbital_mass_unit <= 0.0
        {
            config.element_orbital_mass_unit = 1.0;
        }
        if !config.element_radius_scale.is_finite() || config.element_radius_scale <= 0.0 {
            config.element_radius_scale = 1.0;
        }
        if !config.element_prime_alpha.is_finite() {
            config.element_prime_alpha = 0.0;
        }
        if !config.element_prime_beta.is_finite() {
            config.element_prime_beta = 0.0;
        }
        if !config.element_light_energy.is_finite() || config.element_light_energy < 0.0 {
            config.element_light_energy = 2.0;
        }
        config.element_initial_sorting_mode = config.element_initial_sorting_mode.clamp(0, 2);
        config.element_initial_grouping_mode = config.element_initial_grouping_mode.clamp(0, 2);
        if !config.element_group_spacing.is_finite() || config.element_group_spacing <= 0.0 {
            config.element_group_spacing = 1.0;
        }
        if !config.element_group_internal_velocity_scale.is_finite()
            || config.element_group_internal_velocity_scale <= 0.0
        {
            config.element_group_internal_velocity_scale = 1.0;
        }
        if !config.element_sample_orbit_radius_scale.is_finite()
            || config.element_sample_orbit_radius_scale <= 0.0
        {
            config.element_sample_orbit_radius_scale = 1.0;
        }
        if !config.element_sample_orbit_speed_factor.is_finite()
            || config.element_sample_orbit_speed_factor <= 0.0
        {
            config.element_sample_orbit_speed_factor = 1.0;
        }
 
        config
    }

    pub fn selected_atomic_numbers(&self) -> Vec<u8> {
        fn parse_range(range: &str) -> Option<Vec<u8>> {
            let pair = if range.contains("..") {
                range.split_once("..")
            } else if range.contains('-') {
                range.split_once('-')
            } else {
                None
            };
            if let Some((start, end)) = pair {
                let start = start.trim().parse::<u8>().ok()?;
                let end = end.trim().parse::<u8>().ok()?;
                if start == 0 || end == 0 {
                    return None;
                }
                let (min_v, max_v) = if start <= end { (start, end) } else { (end, start) };
                Some((min_v..=max_v).collect())
            } else {
                None
            }
        }

        let element_count = elements::elements().len() as u8;
        let mut numbers = Vec::new();

        if !self.element_atomic_numbers.trim().is_empty() {
            for token in self.element_atomic_numbers.split(',') {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }
                if let Some(range) = parse_range(token) {
                    for z in range {
                        if z >= 1 && z <= element_count {
                            if !numbers.contains(&z) {
                                numbers.push(z);
                            }
                        }
                    }
                } else if let Ok(z) = token.parse::<u8>() {
                    if z >= 1 && z <= element_count && !numbers.contains(&z) {
                        numbers.push(z);
                    }
                }
            }
        }

        if numbers.is_empty() {
            let max_atomic = self.element_max_atomic_number.min(element_count);
            numbers.extend(1..=max_atomic);
        }

        if self.element_atomic_count == 0 {
            return numbers;
        }

        let mut selected = Vec::new();
        let mut index = 0;
        while selected.len() < self.element_atomic_count {
            if numbers.is_empty() {
                break;
            }
            selected.push(numbers[index % numbers.len()]);
            index += 1;
        }

        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_reads_atomic_list_and_count() {
        let config_text = r#"
Some introduction text
```config
# explicit element selection
element_atomic_numbers: 1,2,3
element_atomic_count: 40
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.element_atomic_numbers, "1,2,3");
        assert_eq!(config.element_atomic_count, 40);
        let selected = config.selected_atomic_numbers();
        assert_eq!(selected.len(), 40);
        assert_eq!(&selected[0..3], &[1, 2, 3]);
        assert_eq!(&selected[3..6], &[1, 2, 3]);
    }

    #[test]
    fn parse_text_ignores_inline_config_fence_without_whitespace() {
        let config_text = r#"
This line contains an inline ```config```-Block marker.
```config
# actual config block
element_atomic_numbers: 6,7
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.element_atomic_numbers, "6,7");
        assert_eq!(config.selected_atomic_numbers(), vec![6]);
    }
}
