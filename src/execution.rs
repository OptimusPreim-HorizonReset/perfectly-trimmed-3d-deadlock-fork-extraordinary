use crate::{config::InformationsConfig, galaxy_templates::GalaxyTemplate, simulation::Simulation};
use ultraviolet::Vec2;

fn total_kinetic_energy(bodies: &[crate::body::Body]) -> f64 {
    bodies
        .iter()
        .map(|body| 0.5 * body.mass as f64 * body.vel.mag_sq() as f64)
        .sum()
}

fn total_potential_energy(bodies: &[crate::body::Body]) -> f64 {
    let mut potential = 0.0_f64;
    for i in 0..bodies.len() {
        for j in i + 1..bodies.len() {
            let distance = (bodies[j].pos - bodies[i].pos).mag() as f64;
            if distance > 0.0 {
                potential -= bodies[i].mass as f64 * bodies[j].mass as f64 / distance;
            }
        }
    }
    potential
}

/// Execution helper for config-driven validation and automated iteration.
pub struct ExecutionFramework;

pub struct EquilibriumEvaluation {
    pub config_name: String,
    pub steps: usize,
    pub center_of_mass_drift: f32,
    pub mass_conservation_error: f32,
    pub energy_drift: f32,
}

impl ExecutionFramework {
    /// Runs an optional config-driven execution mode and exits if configured.
    pub fn maybe_run(config: &InformationsConfig) {
        if std::env::var("RUN_CONFIG_EVALUATION").is_ok() {
            let evaluation = Self::evaluate_disk_equilibrium(config, 16);
            println!("=== CONFIG EVALUATION ===");
            println!("config summary: {}", evaluation.config_name);
            println!("steps: {}", evaluation.steps);
            println!(
                "center of mass drift: {:.6}",
                evaluation.center_of_mass_drift
            );
            println!(
                "mass conservation error: {:.6}",
                evaluation.mass_conservation_error
            );
            println!("energy drift: {:.6}", evaluation.energy_drift);
            std::process::exit(0);
        }

        if std::env::var("RUN_CONFIG_SWEEP").is_ok() {
            println!("=== CONFIG SWEEP MODE ===");
            let q_value = config.disk_equilibrium_toomre_q;
            let upper = (q_value + 0.4).min(2.0);
            let steps = 12;
            let delta = (upper - q_value) / steps as f32;
            for i in 0..=steps {
                let mut sweep_config = config.clone();
                sweep_config.disk_equilibrium_toomre_q = q_value + delta * i as f32;
                let evaluation = Self::evaluate_disk_equilibrium(&sweep_config, 10);
                println!(
                    "Q={:.3}: drift={:.6}, mass_err={:.6}, energy_err={:.6}",
                    sweep_config.disk_equilibrium_toomre_q,
                    evaluation.center_of_mass_drift,
                    evaluation.mass_conservation_error,
                    evaluation.energy_drift,
                );
            }
            std::process::exit(0);
        }
    }

    pub fn evaluate_disk_equilibrium(
        config: &InformationsConfig,
        steps: usize,
    ) -> EquilibriumEvaluation {
        let mut config = config.clone();
        let evaluation_n = config.n.min(512);
        config.n = evaluation_n;
        config.particle_budget = config.particle_budget.max(evaluation_n + 8);
        config.gas_enabled = false;
        config.enable_galactic_atom_simulation = false;
        config.element_atomic_numbers = String::new();
        config.element_atomic_count = 0;

        let template = GalaxyTemplate::from_config(&config);
        let bodies =
            template.generate_with_config(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), &config);

        let initial_mass: f64 = bodies.iter().map(|body| body.mass as f64).sum();
        let initial_energy = total_kinetic_energy(&bodies) + total_potential_energy(&bodies);
        let initial_center = bodies.iter().fold(ultraviolet::Vec3::zero(), |sum, body| {
            sum + body.pos * body.mass
        }) / initial_mass as f32;

        let mut simulation = Simulation::new(&config);
        simulation.replace_bodies_with_gpdm(bodies);

        for _ in 0..steps {
            simulation.step();
        }

        let final_mass: f64 = simulation.bodies.iter().map(|body| body.mass as f64).sum();
        let final_energy =
            total_kinetic_energy(&simulation.bodies) + total_potential_energy(&simulation.bodies);
        let final_center = simulation
            .bodies
            .iter()
            .fold(ultraviolet::Vec3::zero(), |sum, body| {
                sum + body.pos * body.mass
            })
            / final_mass as f32;

        EquilibriumEvaluation {
            config_name: format!(
                "dt={} n={} Q={:.3} drift={:.3} height={:.4}",
                config.dt,
                config.n,
                config.disk_equilibrium_toomre_q,
                config.disk_equilibrium_asymmetric_drift_strength,
                config.disk_equilibrium_scale_height_factor,
            ),
            steps,
            center_of_mass_drift: (final_center - initial_center).mag(),
            mass_conservation_error: ((final_mass - initial_mass) / initial_mass).abs() as f32,
            energy_drift: ((final_energy - initial_energy) / initial_energy).abs() as f32,
        }
    }
}


