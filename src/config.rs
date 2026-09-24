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
    pub performance_dynamic_interval_scaling: bool,
    pub performance_body_count_threshold: usize,
    pub performance_max_interval_scale: usize,
    pub performance_render_body_limit: usize,
    pub performance_render_sample_ratio: f32,
    pub performance_high_count_mode: bool,
    pub performance_disable_adaptive_updates_in_high_count: bool,
    pub performance_octree_reserve_factor: f32,
    pub particle_budget: usize,
    pub max_gas_particle_ratio: f32,
    pub render_lod_distance_factor: f32,
    pub gas_enabled: bool,
    pub gas_meta_particle_count: usize,
    pub gas_target_neighbors: usize,
    pub gas_volume_enabled: bool,
    pub gas_volume_count: usize,
    pub gas_volume_radius_factor: f32,
    pub gas_update_interval: usize,
    pub gas_h_min: f32,
    pub gas_h_max: f32,
    pub gas_pressure_coefficient: f32,
    pub gas_condensation_strength: f32,
    pub gas_merge_distance_factor: f32,
    pub gas_merge_velocity_max: f32,
    pub gas_ignition_density: f32,
    pub gas_ignition_temperature: f32,
    pub gas_ignition_mass: f32,
    pub gas_rest_density: f32,
    pub gas_cooling_rate: f32,
    pub gas_relaxation_strength: f32,
    pub gas_render_opacity: f32,
    pub gas_motion_blur_strength: f32,
    pub gas_density_color_scale: f32,
    pub gas_temperature_color_scale: f32,
    pub orbital_speed_multiplier: f32,
    pub spawn_angular_speed_base: f32,
    pub spawn_angular_speed_range: f32,
    pub spin_speed_multiplier: f32,
    pub equatorial_growth_factor: f32,
    pub polar_flattening_factor: f32,
    pub radius_scale: f32,
    pub depth_scale_factor: f32,
    pub spacetime_dilation_factor: f32,
    pub enable_disk_equilibrium_mode: bool,
    pub disk_equilibrium_toomre_q: f32,
    pub disk_equilibrium_scale_height_factor: f32,
    pub disk_equilibrium_asymmetric_drift_strength: f32,
    pub disk_equilibrium_softening_ratio: f32,
    pub disk_equilibrium_theta: f32,
    /* GPDM configuration (see GPDM_IMPLEMENTATION_HANDOFF.md) */
    pub enable_gpdm: bool,
    pub gpdm_tick_interval: usize,
    // Phase-0 behavior: whether to bootstrap account balances from host body.mass
    // when the runtime requests transfers. Default=true to preserve existing tests.
    pub gpdm_bootstrap_accounts_from_body_mass: bool,
    pub gpdm_local_cell_size_factor: f32,
    pub gpdm_region_min_tables: usize,
    pub gpdm_max_flow_fraction: f32,
    pub gpdm_mass_tolerance: f64,
    pub gpdm_energy_tolerance: f64,
    pub gpdm_momentum_tolerance: f64,
    pub gpdm_role_sum_tolerance: f32,
    pub gpdm_enable_institution_detection: bool,
    pub gpdm_institution_window_frames: usize,
    pub gpdm_institution_create_threshold: f32,
    pub gpdm_institution_destroy_threshold: f32,
    pub gpdm_contact_radius_factor: f32,
    pub gpdm_contact_min_frames: usize,
    pub gpdm_history_retention_events: usize,
    pub gpdm_participant_tombstone_grace_frames: usize,
    pub gpdm_smbh_accretion_radius_factor: f32,
    pub gpdm_smbh_max_accretion_fraction: f32,
    pub gpdm_feedback_efficiency: f32,
    pub gpdm_feedback_energy_scale: f32,
    pub gpdm_wind_fraction: f32,
    pub gpdm_jet_fraction: f32,
    pub gpdm_enable_backreaction: bool,
    pub gpdm_allow_equilibrium_backreaction: bool,
    pub enable_galactic_atom_simulation: bool,
    pub enable_elemental_galaxy_pair_mode: bool,
    pub element_atomic_numbers: String,
    pub element_atomic_count: usize,
    pub element_pair_atomic_numbers: String,
    pub element_pair_core_atomic_numbers: String,
    pub enable_chemistry_compass_coloring: bool,
    pub enable_elemental_binding_response: bool,
    pub elemental_binding_response_strength: f32,
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
    pub enable_scientific_element_gravity: bool,
    pub element_gravity_mass_weight: f32,
    pub element_gravity_electronegativity_weight: f32,
    pub element_gravity_radius_weight: f32,
    pub element_gravity_reactivity_weight: f32,
    pub enable_molecular_bonding: bool,
    pub molecule_bond_strength_factor: f32,
    pub molecule_break_energy_threshold: f32,
    pub molecule_max_bodies: usize,
    pub molecule_identify_compounds: bool,
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
            performance_dynamic_interval_scaling: true,
            performance_body_count_threshold: 5000,
            performance_max_interval_scale: 4,
            performance_render_body_limit: 5000,
            performance_render_sample_ratio: 1.0,
            performance_high_count_mode: false,
            performance_disable_adaptive_updates_in_high_count: false,
            performance_octree_reserve_factor: 6.0,
            particle_budget: 8000,
            max_gas_particle_ratio: 0.12,
            render_lod_distance_factor: 1.2,
            gas_enabled: true,
            gas_meta_particle_count: 250,
            gas_target_neighbors: 12,
            gas_volume_enabled: true,
            gas_volume_count: 180,
            gas_volume_radius_factor: 0.4,
            gas_update_interval: 2,
            gas_h_min: 0.3,
            gas_h_max: 7.0,
            gas_pressure_coefficient: 0.045,
            gas_condensation_strength: 0.12,
            gas_merge_distance_factor: 1.25,
            gas_merge_velocity_max: 0.14,
            gas_ignition_density: 0.0006,
            gas_ignition_temperature: 0.68,
            gas_ignition_mass: 0.00001,
            gas_rest_density: 0.3,
            gas_cooling_rate: 0.08,
            gas_relaxation_strength: 0.26,
            gas_render_opacity: 0.72,
            gas_motion_blur_strength: 0.38,
            gas_density_color_scale: 1.4,
            gas_temperature_color_scale: 0.9,
            orbital_speed_multiplier: 0.025,
            spawn_angular_speed_base: 0.3,
            spawn_angular_speed_range: 0.5,
            spin_speed_multiplier: 24.0,
            equatorial_growth_factor: 0.18,
            polar_flattening_factor: 0.14,
            radius_scale: 0.5,
            depth_scale_factor: 0.002,
            spacetime_dilation_factor: 2.0,
            enable_disk_equilibrium_mode: false,
            disk_equilibrium_toomre_q: 1.3,
            disk_equilibrium_scale_height_factor: 0.025,
            disk_equilibrium_asymmetric_drift_strength: 0.12,
            disk_equilibrium_softening_ratio: 0.025,
            disk_equilibrium_theta: 0.60,
            /* GPDM defaults */
            enable_gpdm: false,
            gpdm_tick_interval: 1,
            // Keep Phase-0 behavior (bootstrap account balances from body.mass)
            // on by default to preserve existing deterministic tests.
            gpdm_bootstrap_accounts_from_body_mass: true,
            // Phase-1: grace period (frames) before a removed participant ID can be considered reusable.
            // This protects against immediate id-reuse races and gives observers time to converge.
            // Default: 300 frames (~5s at dt=0.016). See GPDM_IMPLEMENTATION_HANDOFF.md for details.
            gpdm_participant_tombstone_grace_frames: 300,
            gpdm_local_cell_size_factor: 1.0,
            gpdm_region_min_tables: 4,
            gpdm_max_flow_fraction: 0.02,
            gpdm_mass_tolerance: 1e-8,
            gpdm_energy_tolerance: 1e-6,
            gpdm_momentum_tolerance: 1e-6,
            gpdm_role_sum_tolerance: 1e-5,
            gpdm_enable_institution_detection: true,
            gpdm_institution_window_frames: 32,
            gpdm_institution_create_threshold: 0.80,
            gpdm_institution_destroy_threshold: 0.55,
            gpdm_contact_radius_factor: 1.0,
            gpdm_contact_min_frames: 8,
            gpdm_history_retention_events: 512,
            gpdm_smbh_accretion_radius_factor: 2.0,
            gpdm_smbh_max_accretion_fraction: 0.01,
            gpdm_feedback_efficiency: 0.05,
            gpdm_feedback_energy_scale: 1.0,
            gpdm_wind_fraction: 0.70,
            gpdm_jet_fraction: 0.20,
            gpdm_enable_backreaction: false,
            gpdm_allow_equilibrium_backreaction: false,
            enable_galactic_atom_simulation: false,
            enable_elemental_galaxy_pair_mode: false,
            element_atomic_numbers: String::from("1"),
            element_atomic_count: 1,
            element_pair_atomic_numbers: String::new(),
            element_pair_core_atomic_numbers: String::new(),
            enable_chemistry_compass_coloring: true,
            enable_elemental_binding_response: true,
            elemental_binding_response_strength: 0.35,
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
            enable_scientific_element_gravity: false,
            element_gravity_mass_weight: 1.0,
            element_gravity_electronegativity_weight: 0.4,
            element_gravity_radius_weight: 0.3,
            element_gravity_reactivity_weight: 0.2,
            enable_molecular_bonding: false,
            molecule_bond_strength_factor: 1.0,
            molecule_break_energy_threshold: 0.8,
            molecule_max_bodies: 12,
            molecule_identify_compounds: true,
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
                        "performance_dynamic_interval_scaling" => parse_bool(value).map(|v| {
                            config.performance_dynamic_interval_scaling = v;
                            true
                        }),
                        "performance_body_count_threshold" => {
                            value.parse::<usize>().ok().map(|v| {
                                config.performance_body_count_threshold = v;
                                true
                            })
                        }
                        "performance_max_interval_scale" => value.parse::<usize>().ok().map(|v| {
                            config.performance_max_interval_scale = v;
                            true
                        }),
                        "performance_render_body_limit" => value.parse::<usize>().ok().map(|v| {
                            config.performance_render_body_limit = v;
                            true
                        }),
                        "performance_render_sample_ratio" => parse_f32(value).map(|v| {
                            config.performance_render_sample_ratio = v;
                            true
                        }),
                        "performance_high_count_mode" => parse_bool(value).map(|v| {
                            config.performance_high_count_mode = v;
                            true
                        }),
                        "performance_disable_adaptive_updates_in_high_count" => parse_bool(value)
                            .map(|v| {
                                config.performance_disable_adaptive_updates_in_high_count = v;
                                true
                            }),
                        "performance_octree_reserve_factor" => parse_f32(value).map(|v| {
                            config.performance_octree_reserve_factor = v;
                            true
                        }),
                        "particle_budget" => value.parse::<usize>().ok().map(|v| {
                            config.particle_budget = v;
                            true
                        }),
                        "max_gas_particle_ratio" => parse_f32(value).map(|v| {
                            config.max_gas_particle_ratio = v;
                            true
                        }),
                        "render_lod_distance_factor" => parse_f32(value).map(|v| {
                            config.render_lod_distance_factor = v;
                            true
                        }),
                        "gas_enabled" => parse_bool(value).map(|v| {
                            config.gas_enabled = v;
                            true
                        }),
                        "gas_meta_particle_count" => value.parse::<usize>().ok().map(|v| {
                            config.gas_meta_particle_count = v;
                            true
                        }),
                        "gas_target_neighbors" => value.parse::<usize>().ok().map(|v| {
                            config.gas_target_neighbors = v;
                            true
                        }),
                        "gas_volume_enabled" => parse_bool(value).map(|v| {
                            config.gas_volume_enabled = v;
                            true
                        }),
                        "gas_volume_count" => value.parse::<usize>().ok().map(|v| {
                            config.gas_volume_count = v;
                            true
                        }),
                        "gas_volume_radius_factor" => parse_f32(value).map(|v| {
                            config.gas_volume_radius_factor = v;
                            true
                        }),
                        "gas_update_interval" => value.parse::<usize>().ok().map(|v| {
                            config.gas_update_interval = v;
                            true
                        }),
                        "gas_h_min" => parse_f32(value).map(|v| {
                            config.gas_h_min = v;
                            true
                        }),
                        "gas_h_max" => parse_f32(value).map(|v| {
                            config.gas_h_max = v;
                            true
                        }),
                        "gas_pressure_coefficient" => parse_f32(value).map(|v| {
                            config.gas_pressure_coefficient = v;
                            true
                        }),
                        "gas_condensation_strength" => parse_f32(value).map(|v| {
                            config.gas_condensation_strength = v;
                            true
                        }),
                        "gas_merge_distance_factor" => parse_f32(value).map(|v| {
                            config.gas_merge_distance_factor = v;
                            true
                        }),
                        "gas_merge_velocity_max" => parse_f32(value).map(|v| {
                            config.gas_merge_velocity_max = v;
                            true
                        }),
                        "gas_ignition_density" => parse_f32(value).map(|v| {
                            config.gas_ignition_density = v;
                            true
                        }),
                        "gas_ignition_temperature" => parse_f32(value).map(|v| {
                            config.gas_ignition_temperature = v;
                            true
                        }),
                        "gas_ignition_mass" => parse_f32(value).map(|v| {
                            config.gas_ignition_mass = v;
                            true
                        }),
                        "gas_rest_density" => parse_f32(value).map(|v| {
                            config.gas_rest_density = v;
                            true
                        }),
                        "gas_cooling_rate" => parse_f32(value).map(|v| {
                            config.gas_cooling_rate = v;
                            true
                        }),
                        "gas_relaxation_strength" => parse_f32(value).map(|v| {
                            config.gas_relaxation_strength = v;
                            true
                        }),
                        "gas_render_opacity" => parse_f32(value).map(|v| {
                            config.gas_render_opacity = v;
                            true
                        }),
                        "gas_motion_blur_strength" => parse_f32(value).map(|v| {
                            config.gas_motion_blur_strength = v;
                            true
                        }),
                        "gas_density_color_scale" => parse_f32(value).map(|v| {
                            config.gas_density_color_scale = v;
                            true
                        }),
                        "gas_temperature_color_scale" => parse_f32(value).map(|v| {
                            config.gas_temperature_color_scale = v;
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
                        "enable_disk_equilibrium_mode" => parse_bool(value).map(|v| {
                            config.enable_disk_equilibrium_mode = v;
                            true
                        }),
                        "disk_equilibrium_toomre_q" => parse_f32(value).map(|v| {
                            config.disk_equilibrium_toomre_q = v;
                            true
                        }),
                        "disk_equilibrium_scale_height_factor" => parse_f32(value).map(|v| {
                            config.disk_equilibrium_scale_height_factor = v;
                            true
                        }),
                        "disk_equilibrium_asymmetric_drift_strength" => parse_f32(value).map(|v| {
                            config.disk_equilibrium_asymmetric_drift_strength = v;
                            true
                        }),
                        "disk_equilibrium_softening_ratio" => parse_f32(value).map(|v| {
                            config.disk_equilibrium_softening_ratio = v;
                            true
                        }),
                        "disk_equilibrium_theta" => parse_f32(value).map(|v| {
                            config.disk_equilibrium_theta = v;
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
                        "enable_elemental_galaxy_pair_mode" => parse_bool(value).map(|v| {
                            config.enable_elemental_galaxy_pair_mode = v;
                            true
                        }),
                        "element_atomic_numbers" => {
                            let normalized = value.trim().replace(' ', "");
                            config.element_atomic_numbers = normalized;
                            Some(true)
                        }
                        "element_pair_atomic_numbers" => {
                            let normalized = value.trim().replace(' ', "");
                            config.element_pair_atomic_numbers = normalized;
                            Some(true)
                        }
                        "element_pair_core_atomic_numbers" => {
                            let normalized = value.trim().replace(' ', "");
                            config.element_pair_core_atomic_numbers = normalized;
                            Some(true)
                        }
                        "enable_chemistry_compass_coloring" => parse_bool(value).map(|v| {
                            config.enable_chemistry_compass_coloring = v;
                            true
                        }),
                        "enable_elemental_binding_response" => parse_bool(value).map(|v| {
                            config.enable_elemental_binding_response = v;
                            true
                        }),
                        "elemental_binding_response_strength" => parse_f32(value).map(|v| {
                            config.elemental_binding_response_strength = v;
                            true
                        }),
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
                        "enable_elemental_mass_dimension_visualization" => {
                            parse_bool(value).map(|v| {
                                config.enable_elemental_mass_dimension_visualization = v;
                                true
                            })
                        }
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
                        "enable_scientific_element_gravity" => parse_bool(value).map(|v| {
                            config.enable_scientific_element_gravity = v;
                            true
                        }),
                        "element_gravity_mass_weight" => parse_f32(value).map(|v| {
                            config.element_gravity_mass_weight = v;
                            true
                        }),
                        "element_gravity_electronegativity_weight" => parse_f32(value).map(|v| {
                            config.element_gravity_electronegativity_weight = v;
                            true
                        }),
                        "element_gravity_radius_weight" => parse_f32(value).map(|v| {
                            config.element_gravity_radius_weight = v;
                            true
                        }),
                        "element_gravity_reactivity_weight" => parse_f32(value).map(|v| {
                            config.element_gravity_reactivity_weight = v;
                            true
                        }),
                        "enable_molecular_bonding" => parse_bool(value).map(|v| {
                            config.enable_molecular_bonding = v;
                            true
                        }),
                        "molecule_bond_strength_factor" => parse_f32(value).map(|v| {
                            config.molecule_bond_strength_factor = v;
                            true
                        }),
                        "molecule_break_energy_threshold" => parse_f32(value).map(|v| {
                            config.molecule_break_energy_threshold = v;
                            true
                        }),
                        "molecule_max_bodies" => value.parse::<usize>().ok().map(|v| {
                            config.molecule_max_bodies = v;
                            true
                        }),
                        "molecule_identify_compounds" => parse_bool(value).map(|v| {
                            config.molecule_identify_compounds = v;
                            true
                        }),
                        "enable_gpdm" => parse_bool(value).map(|v| {
                            config.enable_gpdm = v;
                            true
                        }),
                        "gpdm_tick_interval" => value.parse::<usize>().ok().map(|v| {
                            config.gpdm_tick_interval = v;
                            true
                        }),
                        "gpdm_local_cell_size_factor" => parse_f32(value).map(|v| {
                            config.gpdm_local_cell_size_factor = v;
                            true
                        }),
                        "gpdm_region_min_tables" => value.parse::<usize>().ok().map(|v| {
                            config.gpdm_region_min_tables = v;
                            true
                        }),
                        "gpdm_max_flow_fraction" => parse_f32(value).map(|v| {
                            config.gpdm_max_flow_fraction = v;
                            true
                        }),
                        "gpdm_mass_tolerance" => value.parse::<f64>().ok().map(|v| {
                            config.gpdm_mass_tolerance = v;
                            true
                        }),
                        "gpdm_energy_tolerance" => value.parse::<f64>().ok().map(|v| {
                            config.gpdm_energy_tolerance = v;
                            true
                        }),
                        "gpdm_momentum_tolerance" => value.parse::<f64>().ok().map(|v| {
                            config.gpdm_momentum_tolerance = v;
                            true
                        }),
                        "gpdm_role_sum_tolerance" => parse_f32(value).map(|v| {
                            config.gpdm_role_sum_tolerance = v;
                            true
                        }),
                        "gpdm_enable_institution_detection" => parse_bool(value).map(|v| {
                            config.gpdm_enable_institution_detection = v;
                            true
                        }),
                        "gpdm_institution_window_frames" => value.parse::<usize>().ok().map(|v| {
                            config.gpdm_institution_window_frames = v;
                            true
                        }),
                        "gpdm_institution_create_threshold" => parse_f32(value).map(|v| {
                            config.gpdm_institution_create_threshold = v;
                            true
                        }),
                        "gpdm_institution_destroy_threshold" => parse_f32(value).map(|v| {
                            config.gpdm_institution_destroy_threshold = v;
                            true
                        }),
                        "gpdm_contact_radius_factor" => parse_f32(value).map(|v| {
                            config.gpdm_contact_radius_factor = v;
                            true
                        }),
                        "gpdm_contact_min_frames" => value.parse::<usize>().ok().map(|v| {
                            config.gpdm_contact_min_frames = v;
                            true
                        }),
                        "gpdm_history_retention_events" => value.parse::<usize>().ok().map(|v| {
                            config.gpdm_history_retention_events = v;
                            true
                        }),
                        "gpdm_smbh_accretion_radius_factor" => parse_f32(value).map(|v| {
                            config.gpdm_smbh_accretion_radius_factor = v;
                            true
                        }),
                        "gpdm_smbh_max_accretion_fraction" => parse_f32(value).map(|v| {
                            config.gpdm_smbh_max_accretion_fraction = v;
                            true
                        }),
                        "gpdm_feedback_efficiency" => parse_f32(value).map(|v| {
                            config.gpdm_feedback_efficiency = v;
                            true
                        }),
                        "gpdm_feedback_energy_scale" => parse_f32(value).map(|v| {
                            config.gpdm_feedback_energy_scale = v;
                            true
                        }),
                        "gpdm_wind_fraction" => parse_f32(value).map(|v| {
                            config.gpdm_wind_fraction = v;
                            true
                        }),
                        "gpdm_jet_fraction" => parse_f32(value).map(|v| {
                            config.gpdm_jet_fraction = v;
                            true
                        }),
                        "gpdm_enable_backreaction" => parse_bool(value).map(|v| {
                            config.gpdm_enable_backreaction = v;
                            true
                        }),
                        "gpdm_allow_equilibrium_backreaction" => parse_bool(value).map(|v| {
                            config.gpdm_allow_equilibrium_backreaction = v;
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
        if !config.elemental_binding_response_strength.is_finite() {
            config.elemental_binding_response_strength = 0.35;
        }
        config.elemental_binding_response_strength =
            config.elemental_binding_response_strength.clamp(0.0, 1.0);
        if !config.element_universe_scatter_factor.is_finite()
            || config.element_universe_scatter_factor <= 0.0
        {
            config.element_universe_scatter_factor = 1.0;
        }
        config.gas_meta_particle_count = config.gas_meta_particle_count.max(1);
        config.gas_target_neighbors = config.gas_target_neighbors.max(1);
        if config.gas_volume_count == 0 {
            config.gas_volume_count = 1;
        }
        config.gas_update_interval = config.gas_update_interval.max(1);
        if !config.gas_volume_radius_factor.is_finite() || config.gas_volume_radius_factor <= 0.0 {
            config.gas_volume_radius_factor = 0.4;
        }
        if !config.gas_h_min.is_finite() || config.gas_h_min <= 0.0 {
            config.gas_h_min = 0.1;
        }
        if !config.gas_h_max.is_finite() || config.gas_h_max <= config.gas_h_min {
            config.gas_h_max = (config.gas_h_min * 2.0).max(config.gas_h_min + 1.0);
        }
        config.gas_pressure_coefficient = config.gas_pressure_coefficient.max(0.0);
        config.gas_condensation_strength = config.gas_condensation_strength.max(0.0);
        config.gas_merge_distance_factor = config.gas_merge_distance_factor.max(0.1);
        if !config.gas_merge_velocity_max.is_finite() || config.gas_merge_velocity_max < 0.0 {
            config.gas_merge_velocity_max = 0.0;
        }
        config.gas_ignition_density = config.gas_ignition_density.max(0.0);
        config.gas_ignition_temperature = config.gas_ignition_temperature.clamp(0.0, 10.0);
        config.gas_ignition_mass = config.gas_ignition_mass.max(0.0);
        config.gas_rest_density = config.gas_rest_density.max(0.01);
        config.gas_cooling_rate = config.gas_cooling_rate.clamp(0.0, 1.0);
        config.gas_relaxation_strength = config.gas_relaxation_strength.clamp(0.0, 1.0);
        config.gas_render_opacity = config.gas_render_opacity.clamp(0.0, 1.0);
        config.gas_motion_blur_strength = config.gas_motion_blur_strength.clamp(0.0, 1.0);
        config.gas_density_color_scale = config.gas_density_color_scale.max(0.1);
        config.gas_temperature_color_scale = config.gas_temperature_color_scale.max(0.1);
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
        if !config.performance_render_sample_ratio.is_finite()
            || config.performance_render_sample_ratio <= 0.0
        {
            config.performance_render_sample_ratio = 1.0;
        }
        if !config.performance_octree_reserve_factor.is_finite()
            || config.performance_octree_reserve_factor <= 0.0
        {
            config.performance_octree_reserve_factor = 6.0;
        }
        if config.particle_budget == 0 {
            config.particle_budget = 8000;
        }
        if !config.max_gas_particle_ratio.is_finite() || config.max_gas_particle_ratio < 0.0 {
            config.max_gas_particle_ratio = 0.12;
        }
        config.max_gas_particle_ratio = config.max_gas_particle_ratio.clamp(0.0, 1.0);
        if !config.render_lod_distance_factor.is_finite()
            || config.render_lod_distance_factor <= 0.0
        {
            config.render_lod_distance_factor = 1.2;
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
        if config.performance_body_count_threshold == 0 {
            config.performance_body_count_threshold = 5000;
        }
        if config.performance_max_interval_scale == 0 {
            config.performance_max_interval_scale = 4;
        }
        if config.performance_render_body_limit == 0 {
            config.performance_render_body_limit = 5000;
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
        config.element_gravity_mass_weight = config.element_gravity_mass_weight.clamp(0.0, 5.0);
        config.element_gravity_electronegativity_weight =
            config.element_gravity_electronegativity_weight.clamp(0.0, 5.0);
        config.element_gravity_radius_weight = config.element_gravity_radius_weight.clamp(0.0, 5.0);
        config.element_gravity_reactivity_weight =
            config.element_gravity_reactivity_weight.clamp(0.0, 5.0);
        config.molecule_bond_strength_factor = config.molecule_bond_strength_factor.clamp(0.0, 5.0);
        config.molecule_break_energy_threshold =
            config.molecule_break_energy_threshold.clamp(0.0, 10.0);
        if config.molecule_max_bodies == 0 {
            config.molecule_max_bodies = 12;
        }
        if !config.disk_equilibrium_toomre_q.is_finite() || config.disk_equilibrium_toomre_q <= 0.0
        {
            config.disk_equilibrium_toomre_q = 1.3;
        }
        if !config.disk_equilibrium_scale_height_factor.is_finite()
            || config.disk_equilibrium_scale_height_factor <= 0.0
        {
            config.disk_equilibrium_scale_height_factor = 0.025;
        }
        config.disk_equilibrium_asymmetric_drift_strength = config
            .disk_equilibrium_asymmetric_drift_strength
            .clamp(0.0, 1.0);
        config.disk_equilibrium_softening_ratio =
            config.disk_equilibrium_softening_ratio.clamp(0.001, 0.15);
        config.disk_equilibrium_theta = config.disk_equilibrium_theta.clamp(0.05, 1.0);

        /* GPDM sanitization */
        if config.gpdm_tick_interval == 0 {
            config.gpdm_tick_interval = 1;
        }
        if !config.gpdm_local_cell_size_factor.is_finite() || config.gpdm_local_cell_size_factor <= 0.0 {
            config.gpdm_local_cell_size_factor = 1.0;
        }
        if config.gpdm_region_min_tables == 0 {
            config.gpdm_region_min_tables = 4;
        }
        if !config.gpdm_max_flow_fraction.is_finite() {
            config.gpdm_max_flow_fraction = 0.02;
        }
        config.gpdm_max_flow_fraction = config.gpdm_max_flow_fraction.clamp(0.0, 1.0);

        if !config.gpdm_mass_tolerance.is_finite() || config.gpdm_mass_tolerance <= 0.0 {
            config.gpdm_mass_tolerance = 1e-8;
        }
        if !config.gpdm_energy_tolerance.is_finite() || config.gpdm_energy_tolerance <= 0.0 {
            config.gpdm_energy_tolerance = 1e-6;
        }
        if !config.gpdm_momentum_tolerance.is_finite() || config.gpdm_momentum_tolerance <= 0.0 {
            config.gpdm_momentum_tolerance = 1e-6;
        }
        if !config.gpdm_role_sum_tolerance.is_finite() || config.gpdm_role_sum_tolerance <= 0.0 {
            config.gpdm_role_sum_tolerance = 1e-5;
        }
        config.gpdm_institution_create_threshold = config.gpdm_institution_create_threshold.clamp(0.0, 1.0);
        config.gpdm_institution_destroy_threshold = config.gpdm_institution_destroy_threshold.clamp(0.0, 1.0);
        if !config.gpdm_contact_radius_factor.is_finite() || config.gpdm_contact_radius_factor <= 0.0 {
            config.gpdm_contact_radius_factor = 1.0;
        }
        if config.gpdm_contact_min_frames == 0 {
            config.gpdm_contact_min_frames = 8;
        }
        if config.gpdm_history_retention_events == 0 {
            config.gpdm_history_retention_events = 512;
        }
        config.gpdm_wind_fraction = config.gpdm_wind_fraction.clamp(0.0, 1.0);
        config.gpdm_jet_fraction = config.gpdm_jet_fraction.clamp(0.0, 1.0);
        let gpdm_sum = config.gpdm_wind_fraction + config.gpdm_jet_fraction;
        if gpdm_sum.is_finite() && gpdm_sum > 1.0 && gpdm_sum > 0.0 {
            let scale = 1.0 / gpdm_sum;
            config.gpdm_wind_fraction *= scale;
            config.gpdm_jet_fraction *= scale;
        }
        config.gpdm_smbh_max_accretion_fraction = config.gpdm_smbh_max_accretion_fraction.clamp(0.0, 1.0);
        config.gpdm_feedback_efficiency = config.gpdm_feedback_efficiency.clamp(0.0, 1.0);

        config
    }

    pub fn effective_theta(&self) -> f32 {
        if self.enable_disk_equilibrium_mode {
            self.disk_equilibrium_theta.max(0.05)
        } else {
            self.theta.max(0.05)
        }
    }

    pub fn effective_epsilon(&self) -> f32 {
        if self.enable_disk_equilibrium_mode {
            (self.outer_radius * self.disk_equilibrium_softening_ratio).max(1e-6)
        } else {
            self.epsilon
        }
    }

    pub fn effective_dt(&self) -> f32 {
        self.dt
    }

    pub fn effective_dt_min(&self) -> f32 {
        if self.enable_disk_equilibrium_mode {
            (self.dt.abs() * 0.05).max(1e-6)
        } else {
            self.dt_min.max(1e-6)
        }
    }

    pub fn effective_dt_max(&self) -> f32 {
        if self.enable_disk_equilibrium_mode {
            self.dt.abs().max(self.effective_dt_min())
        } else {
            self.dt_max.max(self.effective_dt_min())
        }
    }

    pub fn effective_collision_interval(&self) -> usize {
        if self.enable_disk_equilibrium_mode {
            1
        } else {
            self.collision_interval.max(1)
        }
    }

    pub fn effective_attract_interval(&self) -> usize {
        if self.enable_disk_equilibrium_mode {
            1
        } else {
            self.attract_interval.max(1)
        }
    }

    pub fn effective_high_count_mode(&self) -> bool {
        if self.enable_disk_equilibrium_mode {
            false
        } else {
            self.performance_high_count_mode
        }
    }

    pub fn effective_performance_dynamic_interval_scaling(&self) -> bool {
        if self.enable_disk_equilibrium_mode {
            false
        } else {
            self.performance_dynamic_interval_scaling
        }
    }

    pub fn effective_disable_adaptive_updates_in_high_count(&self) -> bool {
        if self.enable_disk_equilibrium_mode {
            false
        } else {
            self.performance_disable_adaptive_updates_in_high_count
        }
    }

    pub fn effective_enable_adaptive_theta(&self) -> bool {
        self.enable_adaptive_theta
    }

    pub fn effective_enable_adaptive_multipole(&self) -> bool {
        self.enable_adaptive_multipole
    }

    pub fn effective_enable_adaptive_timestep(&self) -> bool {
        self.enable_adaptive_timestep
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
                let (min_v, max_v) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
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

    pub fn selected_pair_atomic_numbers(&self) -> Vec<u8> {
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
                let (min_v, max_v) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                Some((min_v..=max_v).collect())
            } else {
                None
            }
        }

        let element_count = elements::elements().len() as u8;
        let mut numbers = Vec::new();

        if !self.element_pair_atomic_numbers.trim().is_empty() {
            for token in self.element_pair_atomic_numbers.split(',') {
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
            numbers.extend(1..=element_count);
        }

        numbers
    }

    pub fn selected_pair_core_atomic_numbers(&self) -> Vec<u8> {
        if self.element_pair_core_atomic_numbers.trim().is_empty() {
            return self.selected_pair_atomic_numbers();
        }

        let mut core_config = self.clone();
        core_config.element_pair_atomic_numbers = self.element_pair_core_atomic_numbers.clone();
        core_config.selected_pair_atomic_numbers()
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
    fn parse_text_reads_elemental_pair_atomic_numbers() {
        let config_text = r#"
```config
# explicit pair mode element selection
element_pair_atomic_numbers: 3,5,7
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.element_pair_atomic_numbers, "3,5,7");
        assert_eq!(config.selected_pair_atomic_numbers(), vec![3, 5, 7]);
    }

    #[test]
    fn selected_pair_atomic_numbers_defaults_to_all_elements() {
        let config_text = r#"
```config
enable_elemental_galaxy_pair_mode: true
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.selected_pair_atomic_numbers().len(), 118);
        assert_eq!(config.selected_pair_atomic_numbers()[0], 1);
        assert_eq!(config.selected_pair_atomic_numbers()[117], 118);
    }

    #[test]
    fn parse_text_reads_chemistry_compass_and_core_settings() {
        let config_text = r#"
```config
element_pair_atomic_numbers: 1,8,26
element_pair_core_atomic_numbers: 26,28
enable_chemistry_compass_coloring: false
enable_elemental_binding_response: true
elemental_binding_response_strength: 0.7
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.selected_pair_atomic_numbers(), vec![1, 8, 26]);
        assert_eq!(config.selected_pair_core_atomic_numbers(), vec![26, 28]);
        assert!(!config.enable_chemistry_compass_coloring);
        assert!(config.enable_elemental_binding_response);
        assert!((config.elemental_binding_response_strength - 0.7).abs() < 1e-6);
    }

    #[test]
    fn pair_core_selection_falls_back_to_pair_elements() {
        let config_text = r#"
```config
element_pair_atomic_numbers: 6,8
element_pair_core_atomic_numbers:
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert_eq!(config.selected_pair_core_atomic_numbers(), vec![6, 8]);
    }

    #[test]
    fn parse_text_reads_high_count_performance_settings() {
        let config_text = r#"
```config
performance_high_count_mode: true
performance_disable_adaptive_updates_in_high_count: true
performance_octree_reserve_factor: 4.5
performance_render_sample_ratio: 0.2
particle_budget: 9000
max_gas_particle_ratio: 0.18
render_lod_distance_factor: 1.4
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert!(config.performance_high_count_mode);
        assert!(config.performance_disable_adaptive_updates_in_high_count);
        assert_eq!(config.performance_octree_reserve_factor, 4.5);
        assert!((config.performance_render_sample_ratio - 0.2).abs() < 1e-6);
        assert_eq!(config.particle_budget, 9000);
        assert!((config.max_gas_particle_ratio - 0.18).abs() < 1e-6);
        assert!((config.render_lod_distance_factor - 1.4).abs() < 1e-6);
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

    #[test]
    fn parse_text_reads_performance_settings() {
        let config_text = r#"
```config
performance_dynamic_interval_scaling: false
performance_body_count_threshold: 3000
performance_max_interval_scale: 3
performance_render_body_limit: 2500
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert!(!config.performance_dynamic_interval_scaling);
        assert_eq!(config.performance_body_count_threshold, 3000);
        assert_eq!(config.performance_max_interval_scale, 3);
        assert_eq!(config.performance_render_body_limit, 2500);
    }

    #[test]
    fn parse_text_reads_disk_equilibrium_settings() {
        let config_text = r#"
```config
enable_disk_equilibrium_mode: true
disk_equilibrium_toomre_q: 1.4
disk_equilibrium_scale_height_factor: 0.03
disk_equilibrium_asymmetric_drift_strength: 0.18
disk_equilibrium_softening_ratio: 0.035
disk_equilibrium_theta: 0.58
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert!(config.enable_disk_equilibrium_mode);
        assert!((config.disk_equilibrium_toomre_q - 1.4).abs() < 1e-6);
        assert!((config.disk_equilibrium_scale_height_factor - 0.03).abs() < 1e-6);
        assert!((config.disk_equilibrium_asymmetric_drift_strength - 0.18).abs() < 1e-6);
        assert!((config.disk_equilibrium_softening_ratio - 0.035).abs() < 1e-6);
        assert!((config.disk_equilibrium_theta - 0.58).abs() < 1e-6);
    }

    #[test]
    fn disk_equilibrium_effective_parameters_override_defaults() {
        let config_text = r#"
```config
enable_disk_equilibrium_mode: true
disk_equilibrium_theta: 0.12
disk_equilibrium_softening_ratio: 0.02
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert!(config.enable_disk_equilibrium_mode);
        assert!((config.effective_theta() - 0.12).abs() < 1e-6);
        assert!((config.effective_epsilon() - (config.outer_radius * 0.02)).abs() < 1e-6);
    }

    #[test]
    fn effective_values_override_legacy_settings_in_equilibrium_mode() {
        let mut config = InformationsConfig::default();
        config.dt = 500.0;
        config.dt_min = 2.0;
        config.dt_max = 20.0;
        config.collision_interval = 5;
        config.attract_interval = 3;
        config.performance_high_count_mode = true;
        config.performance_disable_adaptive_updates_in_high_count = true;
        config.enable_disk_equilibrium_mode = true;
        config.disk_equilibrium_theta = 0.12;
        config.disk_equilibrium_softening_ratio = 0.01;

        assert_eq!(config.effective_theta(), 0.12);
        assert_eq!(
            config.effective_epsilon(),
            (config.outer_radius * 0.01).max(1e-6)
        );
        assert_eq!(config.effective_dt_min(), (500.0_f32 * 0.05).max(1e-6));
        assert_eq!(config.effective_dt_max(), 500.0);
        assert_eq!(config.effective_collision_interval(), 1);
        assert_eq!(config.effective_attract_interval(), 1);
        assert!(!config.effective_high_count_mode());
        assert!(!config.effective_disable_adaptive_updates_in_high_count());
    }

    #[test]
    fn parse_text_reads_gas_settings() {
        let config_text = r#"
```config
gas_enabled: false
gas_meta_particle_count: 123
gas_target_neighbors: 8
gas_volume_enabled: true
gas_volume_count: 32
gas_volume_radius_factor: 0.55
gas_update_interval: 3
gas_h_min: 0.7
gas_h_max: 5.5
gas_pressure_coefficient: 0.12
gas_condensation_strength: 0.18
gas_merge_distance_factor: 1.2
gas_merge_velocity_max: 0.22
gas_ignition_density: 1.8
gas_ignition_temperature: 0.7
gas_ignition_mass: 0.00002
gas_rest_density: 0.42
gas_cooling_rate: 0.14
gas_relaxation_strength: 0.32
gas_render_opacity: 0.84
gas_motion_blur_strength: 0.27
gas_density_color_scale: 1.8
gas_temperature_color_scale: 0.7
```
"#;
        let config = InformationsConfig::parse_text(config_text);
        assert!(!config.gas_enabled);
        assert_eq!(config.gas_meta_particle_count, 123);
        assert_eq!(config.gas_target_neighbors, 8);
        assert!(config.gas_volume_enabled);
        assert_eq!(config.gas_volume_count, 32);
        assert!((config.gas_volume_radius_factor - 0.55).abs() < 1e-6);
        assert_eq!(config.gas_update_interval, 3);
        assert!((config.gas_h_min - 0.7).abs() < 1e-6);
        assert!((config.gas_h_max - 5.5).abs() < 1e-6);
        assert!((config.gas_pressure_coefficient - 0.12).abs() < 1e-6);
        assert!((config.gas_condensation_strength - 0.18).abs() < 1e-6);
        assert!((config.gas_merge_distance_factor - 1.2).abs() < 1e-6);
        assert!((config.gas_merge_velocity_max - 0.22).abs() < 1e-6);
        assert!((config.gas_ignition_density - 1.8).abs() < 1e-6);
        assert!((config.gas_ignition_temperature - 0.7).abs() < 1e-6);
        assert!((config.gas_ignition_mass - 0.00002).abs() < 1e-9);
        assert!((config.gas_rest_density - 0.42).abs() < 1e-6);
        assert!((config.gas_cooling_rate - 0.14).abs() < 1e-6);
        assert!((config.gas_relaxation_strength - 0.32).abs() < 1e-6);
        assert!((config.gas_render_opacity - 0.84).abs() < 1e-6);
        assert!((config.gas_motion_blur_strength - 0.27).abs() < 1e-6);
        assert!((config.gas_density_color_scale - 1.8).abs() < 1e-6);
        assert!((config.gas_temperature_color_scale - 0.7).abs() < 1e-6);
    }

    #[test]
    fn parse_text_reads_molecular_and_scientific_gravity_settings() {
        let config_text = r#"
```config
enable_scientific_element_gravity: true
element_gravity_mass_weight: 1.2
element_gravity_electronegativity_weight: 0.5
element_gravity_radius_weight: 0.4
element_gravity_reactivity_weight: 0.3
enable_molecular_bonding: true
molecule_bond_strength_factor: 1.5
molecule_break_energy_threshold: 1.2
molecule_max_bodies: 8
molecule_identify_compounds: false
```
"#;

        let config = InformationsConfig::parse_text(config_text);
        assert!(config.enable_scientific_element_gravity);
        assert!((config.element_gravity_mass_weight - 1.2).abs() < 1e-6);
        assert!(
            (config.element_gravity_electronegativity_weight - 0.5).abs() < 1e-6
        );
        assert!((config.element_gravity_radius_weight - 0.4).abs() < 1e-6);
        assert!((config.element_gravity_reactivity_weight - 0.3).abs() < 1e-6);
        assert!(config.enable_molecular_bonding);
        assert!((config.molecule_bond_strength_factor - 1.5).abs() < 1e-6);
        assert!((config.molecule_break_energy_threshold - 1.2).abs() < 1e-6);
        assert_eq!(config.molecule_max_bodies, 8);
        assert!(!config.molecule_identify_compounds);
    }
}
