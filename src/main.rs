use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

mod body;
mod chemistry;
mod config;
mod elements;
mod execution;
mod galaxy_templates;
mod gas;
mod quadtree;
mod renderer;
#[cfg(test)]
mod scientific_validation;
mod simulation;
mod gpdm;
mod utils;

use execution::ExecutionFramework;
use renderer::Renderer;
use simulation::Simulation;

fn main() {
    let app_config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(900, 900),
    };

    let sim_config = config::InformationsConfig::load();
    ExecutionFramework::maybe_run(&sim_config);
    renderer::CURRENT_CONFIG.lock().clone_from(&sim_config);
    let simulation = Simulation::new(&sim_config);
    let last_config_load = Instant::now();
    let config_reload_interval = Duration::from_secs(1); // Alle 1 Sekunde versuchen neu zu laden
    let profile_runtime = std::env::var("PROFILE_RUNTIME").is_ok();
    let profile_frames = std::env::var("PROFILE_FRAMES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(100);
    let profile_max_frames = std::env::var("PROFILE_MAX_FRAMES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());

    std::thread::spawn(move || {
        let mut simulation = simulation;
        let mut last_config_load = last_config_load;
        let mut last_config_hash = config_hash(&sim_config);
        let mut step_accumulator = 0u128;
        let mut render_accumulator = 0u128;
        let mut profile_count = 0usize;

        loop {
            if renderer::PAUSED.load(Ordering::Relaxed) {
                std::thread::yield_now();
            } else {
                // Kontinuierliches Reload: Versuche Config zu laden wenn genug Zeit vergangen ist
                if last_config_load.elapsed() > config_reload_interval {
                    let new_config = config::InformationsConfig::load();
                    let new_hash = config_hash(&new_config);
                    if new_hash != last_config_hash {
                        eprintln!("🔄 Config-Änderung erkannt! Starte Simulation neu...");
                        renderer::CURRENT_CONFIG.lock().clone_from(&new_config);
                        simulation = Simulation::new(&new_config);
                        last_config_hash = new_hash;
                    }
                    last_config_load = Instant::now();
                }

                // Alt: RESET_REQUESTED Flag (für Benutzer-Aktion)
                if renderer::RESET_REQUESTED.swap(false, Ordering::Relaxed) {
                    let sim_config = config::InformationsConfig::load();
                    renderer::CURRENT_CONFIG.lock().clone_from(&sim_config);
                    simulation = Simulation::new(&sim_config);
                    eprintln!("🔄 Reset via UI angefordert!");
                }

                let step_start = Instant::now();
                simulation.step();
                let step_duration = step_start.elapsed();
                let render_start = Instant::now();
                let frame = simulation.frame;
                render(&mut simulation, frame);
                let render_duration = render_start.elapsed();

                if profile_runtime {
                    step_accumulator += step_duration.as_nanos();
                    render_accumulator += render_duration.as_nanos();
                    profile_count += 1;
                    if profile_count >= profile_frames {
                        let step_avg = step_accumulator as f64 / profile_count as f64 / 1_000_000.0;
                        let render_avg =
                            render_accumulator as f64 / profile_count as f64 / 1_000_000.0;
                        eprintln!(
                            "[PROFILE] frames={} avg_step_ms={:.3} avg_render_ms={:.3} total_ms={:.3}",
                            frame,
                            step_avg,
                            render_avg,
                            step_avg + render_avg
                        );
                        step_accumulator = 0;
                        render_accumulator = 0;
                        profile_count = 0;
                    }

                    if let Some(max_frames) = profile_max_frames {
                        if frame >= max_frames {
                            eprintln!("[PROFILE] reached max frames {} - exiting", max_frames);
                            std::process::exit(0);
                        }
                    }
                }
            }
        }
    });

    quarkstrom::run::<Renderer>(app_config);
}

/// Berechnet einen einfachen Hash der Config für Änderungserkennung
fn config_hash(config: &config::InformationsConfig) -> u64 {
    let mut hash: u64 = 0;
    hash ^= config.dt.to_bits() as u64;
    hash ^= config.n as u64;
    hash ^= config.theta.to_bits() as u64;
    hash ^= config.epsilon.to_bits() as u64;
    hash ^= config.enable_disk_equilibrium_mode as u64;
    hash ^= config.disk_equilibrium_toomre_q.to_bits() as u64;
    hash ^= config.disk_equilibrium_scale_height_factor.to_bits() as u64;
    hash ^= config.disk_equilibrium_asymmetric_drift_strength.to_bits() as u64;
    hash ^= config.disk_equilibrium_softening_ratio.to_bits() as u64;
    hash ^= config.disk_equilibrium_theta.to_bits() as u64;
    hash ^= config.enable_adaptive_theta as u64;
    hash ^= (config.enable_adaptive_multipole as u64) << 1;
    hash ^= (config.enable_mixed_precision as u64) << 2;
    hash ^= (config.enable_adaptive_timestep as u64) << 3;
    hash ^= config.adaptive_theta_quiet.to_bits() as u64;
    hash ^= config.adaptive_theta_medium.to_bits() as u64;
    hash ^= config.adaptive_theta_critical.to_bits() as u64;
    hash ^= (config.adaptive_multipole_quiet as u64) << 8;
    hash ^= (config.adaptive_multipole_medium as u64) << 16;
    hash ^= (config.adaptive_multipole_critical as u64) << 24;
    hash ^= config.adaptive_dynamics_quiet_threshold.to_bits() as u64;
    hash ^= config.adaptive_dynamics_critical_threshold.to_bits() as u64;
    hash ^= config.eta_timestep.to_bits() as u64;
    hash ^= config.dt_min.to_bits() as u64;
    hash ^= config.dt_max.to_bits() as u64;
    hash ^= config.max_energy_error.to_bits() as u64;
    hash ^= config.adaptive_feedback_theta_scale.to_bits() as u64;
    hash ^= config.outer_radius.to_bits() as u64;
    hash ^= config.inner_radius.to_bits() as u64;
    hash ^= config.galaxy_separation_factor.to_bits() as u64;
    hash ^= config.accretion_spawn_rate.to_bits() as u64;
    hash ^= config.central_mass.to_bits() as u64;
    hash ^= config.collision_interval as u64;
    hash ^= config.attract_interval as u64;
    hash ^= config.performance_dynamic_interval_scaling as u64;
    hash ^= config.performance_body_count_threshold as u64;
    hash ^= config.performance_max_interval_scale as u64;
    hash ^= config.performance_render_body_limit as u64;
    hash ^= config.performance_render_sample_ratio.to_bits() as u64;
    hash ^= config.performance_high_count_mode as u64;
    hash ^= config.performance_disable_adaptive_updates_in_high_count as u64;
    hash ^= config.performance_octree_reserve_factor.to_bits() as u64;
    hash ^= config.gas_enabled as u64;
    hash ^= config.gas_meta_particle_count as u64;
    hash ^= config.gas_target_neighbors as u64;
    hash ^= config.gas_h_min.to_bits() as u64;
    hash ^= config.gas_h_max.to_bits() as u64;
    hash ^= config.gas_pressure_coefficient.to_bits() as u64;
    hash ^= config.gas_rest_density.to_bits() as u64;
    hash ^= config.gas_cooling_rate.to_bits() as u64;
    hash ^= config.gas_relaxation_strength.to_bits() as u64;
    hash ^= config.gas_render_opacity.to_bits() as u64;
    hash ^= config.gas_motion_blur_strength.to_bits() as u64;
    hash ^= config.gas_density_color_scale.to_bits() as u64;
    hash ^= config.gas_temperature_color_scale.to_bits() as u64;
    hash ^= config.spacetime_dilation_factor.to_bits() as u64;
    hash ^= config.enable_galactic_atom_simulation as u64;
    hash ^= config.enable_elemental_galaxy_pair_mode as u64;
    hash ^= config
        .element_atomic_numbers
        .as_bytes()
        .iter()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64));
    hash ^= config
        .element_pair_atomic_numbers
        .as_bytes()
        .iter()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64));
    hash ^= config
        .element_pair_core_atomic_numbers
        .as_bytes()
        .iter()
        .fold(0u64, |acc, b| acc.wrapping_mul(37).wrapping_add(*b as u64));
    hash ^= (config.enable_chemistry_compass_coloring as u64) << 4;
    hash ^= (config.enable_elemental_binding_response as u64) << 5;
    hash ^= config.elemental_binding_response_strength.to_bits() as u64;
    hash ^= config.element_atomic_count as u64;
    hash ^= config.element_max_atomic_number as u64;
    hash ^= config.element_universe_scatter_factor.to_bits() as u64;
    hash ^= config.element_center_mass_unit.to_bits() as u64;
    hash ^= config.element_orbital_mass_unit.to_bits() as u64;
    hash ^= config.element_radius_scale.to_bits() as u64;
    hash ^= config.element_prime_alpha.to_bits() as u64;
    hash ^= config.element_prime_beta.to_bits() as u64;
    hash ^= config.element_light_energy.to_bits() as u64;
    hash ^= config.enable_elemental_gravitation_tuning as u64;
    hash ^= config.enable_elemental_mass_dimension_visualization as u64;
    hash ^= config.element_initial_sorting_mode as u64;
    hash ^= config.element_initial_grouping_mode as u64;
    hash ^= config.element_group_spacing.to_bits() as u64;
    hash ^= config.element_group_internal_velocity_scale.to_bits() as u64;
    hash ^= config.element_sample_orbit_radius_scale.to_bits() as u64;
    hash ^= config.element_sample_orbit_speed_factor.to_bits() as u64;
    hash
}

fn render(simulation: &mut Simulation, frame: usize) {
    let mut lock = renderer::UPDATE_LOCK.lock();
    if frame % 2 == 0 {
        {
            let mut lock = renderer::BODIES.lock();
            lock.clear();
            lock.extend_from_slice(&simulation.bodies);
        }
        {
            let mut lock = renderer::QUADTREE.lock();
            lock.clear();
            lock.extend_from_slice(&simulation.octree.nodes);
        }
        {
            let mut lock = renderer::MOLECULES.lock();
            lock.clear();
            lock.extend_from_slice(&simulation.molecules);
        }
        *lock |= true;
    }
}
