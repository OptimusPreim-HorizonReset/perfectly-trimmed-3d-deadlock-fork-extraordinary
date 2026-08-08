use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InformationsConfig {
    pub dt: f32,
    pub n: usize,
    pub theta: f32,
    pub epsilon: f32,
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
    pub adaptive_softening_enabled: bool,
    pub sph_enabled: bool,
    pub softening_scale_factor: f32,
    pub softening_min_factor: f32,
    pub softening_max_factor: f32,
    pub hydro_pressure_strength: f32,
    pub hydro_kernel_factor: f32,
    pub render_update_interval: usize,
    pub performance_mode: bool,
    pub ultra_performance_mode: bool,
}

impl Default for InformationsConfig {
    fn default() -> Self {
        Self {
            dt: 0.02,
            n: 20000,
            theta: 1.0,
            epsilon: 1.5,
            accretion_spawn_rate: 0.001,
            inner_radius: 25.0,
            outer_radius: 818.03,
            outer_ring_spawn_zone_inner_ratio: 0.82,
            outer_ring_spawn_zone_outer_ratio: 1.0,
            central_mass: 1e6,
            particle_mass_range: (0.5, 2.0),
            inflow_strength: 0.025,
            restore_strength: 0.08,
            galaxy_separation_factor: 1.0,
            galaxy_count: 40,
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
            collision_interval: 4,
            attract_interval: 2,
            orbital_speed_multiplier: 0.025,
            spawn_angular_speed_base: 0.3,
            spawn_angular_speed_range: 0.5,
            spin_speed_multiplier: 1.0,
            equatorial_growth_factor: 0.18,
            polar_flattening_factor: 0.14,
            radius_scale: 0.25,
            depth_scale_factor: 0.002,
            spacetime_dilation_factor: 2.0,
            adaptive_softening_enabled: true,
            sph_enabled: true,
            softening_scale_factor: 4.0,
            softening_min_factor: 0.5,
            softening_max_factor: 4.0,
            hydro_pressure_strength: 0.02,
            hydro_kernel_factor: 2.0,
            render_update_interval: 2,
            performance_mode: false,
            ultra_performance_mode: false,
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

        for (line_num, line) in file_text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("```config") {
                in_block = true;
                continue;
            }
            if in_block {
                if trimmed.starts_with("```") {
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
                        "adaptive_softening_enabled" => value.parse::<bool>().ok().map(|v| {
                            config.adaptive_softening_enabled = v;
                            true
                        }),
                        "sph_enabled" => value.parse::<bool>().ok().map(|v| {
                            config.sph_enabled = v;
                            true
                        }),
                        "softening_scale_factor" => parse_f32(value).map(|v| {
                            config.softening_scale_factor = v;
                            true
                        }),
                        "softening_min_factor" => parse_f32(value).map(|v| {
                            config.softening_min_factor = v;
                            true
                        }),
                        "softening_max_factor" => parse_f32(value).map(|v| {
                            config.softening_max_factor = v;
                            true
                        }),
                        "hydro_pressure_strength" => parse_f32(value).map(|v| {
                            config.hydro_pressure_strength = v;
                            true
                        }),
                        "hydro_kernel_factor" => parse_f32(value).map(|v| {
                            config.hydro_kernel_factor = v;
                            true
                        }),
                        "render_update_interval" => value.parse::<usize>().ok().map(|v| {
                            config.render_update_interval = v.max(1);
                            true
                        }),
                        "performance_mode" => value.parse::<bool>().ok().map(|v| {
                            config.performance_mode = v;
                            true
                        }),
                        "ultra_performance_mode" => value.parse::<bool>().ok().map(|v| {
                            config.ultra_performance_mode = v;
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

        config
    }
}
