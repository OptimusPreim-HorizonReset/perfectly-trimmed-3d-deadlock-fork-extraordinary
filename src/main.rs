use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

mod body;
mod config;
mod galaxy_templates;
mod quadtree;
mod renderer;
mod simulation;
mod utils;

use renderer::Renderer;
use simulation::Simulation;

fn main() {
    let app_config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(900, 900),
    };

    let sim_config = config::InformationsConfig::load();

    if std::env::args().any(|arg| arg == "--headless-test") || std::env::var("HEADLESS_SIM").is_ok()
    {
        run_headless_test(&sim_config);
        return;
    }

    let simulation = Simulation::new(&sim_config);
    let last_config_load = Instant::now();
    let config_reload_interval = Duration::from_secs(1); // Alle 1 Sekunde versuchen neu zu laden

    std::thread::spawn(move || {
        let mut simulation = simulation;
        let mut last_config_load = last_config_load;
        let mut last_config_hash = config_hash(&sim_config);

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
                        simulation = Simulation::new(&new_config);
                        last_config_hash = new_hash;
                    }
                    last_config_load = Instant::now();
                }

                // Alt: RESET_REQUESTED Flag (für Benutzer-Aktion)
                if renderer::RESET_REQUESTED.swap(false, Ordering::Relaxed) {
                    let sim_config = config::InformationsConfig::load();
                    simulation = Simulation::new(&sim_config);
                    eprintln!("🔄 Reset via UI angefordert!");
                }

                simulation.step();
                let frame = simulation.frame;
                render(&mut simulation, frame);
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
    hash ^= config.outer_radius.to_bits() as u64;
    hash ^= config.inner_radius.to_bits() as u64;
    hash ^= config.galaxy_separation_factor.to_bits() as u64;
    hash ^= config.accretion_spawn_rate.to_bits() as u64;
    hash ^= config.central_mass.to_bits() as u64;
    hash ^= config.collision_interval as u64;
    hash ^= config.attract_interval as u64;
    hash ^= config.render_update_interval as u64;
    hash ^= config.spacetime_dilation_factor.to_bits() as u64;
    hash ^= config.performance_mode as u64;
    hash ^= config.ultra_performance_mode as u64;
    hash ^= config.adaptive_softening_enabled as u64;
    hash ^= config.sph_enabled as u64;
    hash ^= config.softening_scale_factor.to_bits() as u64;
    hash ^= config.softening_min_factor.to_bits() as u64;
    hash ^= config.softening_max_factor.to_bits() as u64;
    hash ^= config.hydro_pressure_strength.to_bits() as u64;
    hash ^= config.hydro_kernel_factor.to_bits() as u64;
    hash
}

fn render(simulation: &mut Simulation, frame: usize) {
    let interval = simulation.render_update_interval();
    let mut lock = renderer::UPDATE_LOCK.lock();
    if frame % interval == 0 {
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
        *lock |= true;
    }
}

fn run_headless_test(config: &config::InformationsConfig) {
    let mut test_config = config.clone();
    test_config.n = 20;
    test_config.galaxy_count = 2;
    let mut simulation = Simulation::new(&test_config);
    let body_count = simulation.bodies.len();
    println!(
        "Headless particle movement test ({} galaxies, {} particles each): {} bodies",
        test_config.galaxy_count, test_config.n, body_count
    );

    let before_positions: Vec<_> = simulation.bodies.iter().map(|body| body.pos).collect();
    let steps = 20;
    for step in 0..steps {
        simulation.step();
        if step == 0 || step + 1 == steps {
            let displacement = simulation
                .bodies
                .iter()
                .zip(before_positions.iter())
                .map(|(body, before_pos)| (body.pos - *before_pos).mag())
                .fold(0.0_f32, |sum, d| sum + d);
            println!(
                " step {:>2}: total displacement = {:.6}",
                step + 1,
                displacement
            );
        }
    }

    let moved_count = simulation
        .bodies
        .iter()
        .zip(before_positions.iter())
        .filter(|(body, before_pos)| body.pos != **before_pos)
        .count();

    println!(
        "Particle movement verification: {} / {} moved",
        moved_count, body_count
    );
    if moved_count == 0 {
        eprintln!("ERROR: no particles moved during simulation steps.");
        std::process::exit(1);
    }
}
