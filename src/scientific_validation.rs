use std::time::Instant;

use crate::{
    body::{Body, ParticleSegmentType},
    config::InformationsConfig,
    galaxy_templates::GalaxyTemplate,
    quadtree::{Oct, Octree},
    renderer,
    simulation::Simulation,
};
use ultraviolet::Vec3;

const VECTOR_TOLERANCE: f32 = 2.0e-5;

fn test_body(pos: Vec3, vel: Vec3, mass: f32) -> Body {
    Body::new(
        pos,
        vel,
        mass,
        1.0e-3,
        0.0,
        Vec3::new(0.0, 0.0, 1.0),
        ParticleSegmentType::Orbital,
    )
}

fn test_element_body(pos: Vec3, mass: f32, elemental_gravitation: f32) -> Body {
    Body::new_element(
        pos,
        Vec3::zero(),
        mass,
        1.0e-3,
        0.0,
        Vec3::new(0.0, 0.0, 1.0),
        6,
        ParticleSegmentType::Orbital,
        13.0,
        0.5,
        0.0,
        elemental_gravitation,
        1.0,
        2.0,
    )
}

fn build_tree(bodies: &[Body], theta: f32, epsilon: f32) -> Octree {
    assert!(!bodies.is_empty());
    let mut tree = Octree::new(theta, epsilon);
    tree.clear(Oct::new_containing(bodies));
    for body in bodies {
        tree.insert(body.pos, body.mass);
    }
    tree.propagate();
    tree
}

fn plummer_acceleration(delta: Vec3, source_mass: f32, epsilon: f32) -> Vec3 {
    let softened_distance_sq = delta.mag_sq() + epsilon * epsilon;
    if softened_distance_sq <= 0.0 {
        Vec3::zero()
    } else {
        delta * (source_mass / softened_distance_sq.powf(1.5))
    }
}

fn direct_acceleration(bodies: &[Body], target_index: usize, epsilon: f32) -> Vec3 {
    let target = bodies[target_index].pos;
    bodies
        .iter()
        .enumerate()
        .filter(|(source_index, _)| *source_index != target_index)
        .fold(Vec3::zero(), |acceleration, (_, source)| {
            acceleration + plummer_acceleration(source.pos - target, source.mass, epsilon)
        })
}

fn tree_accelerations(bodies: &[Body], theta: f32, epsilon: f32) -> Vec<Vec3> {
    let tree = build_tree(bodies, theta, epsilon);
    bodies.iter().map(|body| tree.acc(body.pos)).collect()
}

fn adaptive_tree_accelerations(
    bodies: &[Body],
    theta: f32,
    epsilon: f32,
    multipole_order: u8,
    mixed_precision: bool,
) -> Vec<Vec3> {
    let mut tree = build_tree(bodies, theta, epsilon);
    tree.adaptive_mixed_precision = mixed_precision;
    bodies
        .iter()
        .map(|body| tree.acc_adaptive(body.pos, theta, multipole_order))
        .collect()
}

fn advance_exact(bodies: &mut [Body], dt: f32, epsilon: f32) {
    let accelerations = tree_accelerations(bodies, 0.0, epsilon);
    for (body, acceleration) in bodies.iter_mut().zip(accelerations) {
        body.acc = acceleration;
    }
    for body in bodies {
        body.update(dt);
    }
}

fn total_mass(bodies: &[Body]) -> f64 {
    bodies.iter().map(|body| body.mass as f64).sum()
}

fn total_momentum(bodies: &[Body]) -> Vec3 {
    bodies
        .iter()
        .fold(Vec3::zero(), |sum, body| sum + body.vel * body.mass)
}

fn center_of_mass(bodies: &[Body]) -> Vec3 {
    let mass: f32 = bodies.iter().map(|body| body.mass).sum();
    bodies
        .iter()
        .fold(Vec3::zero(), |sum, body| sum + body.pos * body.mass)
        / mass
}

fn total_angular_momentum(bodies: &[Body]) -> Vec3 {
    bodies.iter().fold(Vec3::zero(), |sum, body| {
        sum + body.pos.cross(body.vel * body.mass)
    })
}

fn total_newtonian_energy(bodies: &[Body]) -> f64 {
    let kinetic: f64 = bodies
        .iter()
        .map(|body| 0.5 * body.mass as f64 * body.vel.mag_sq() as f64)
        .sum();
    let mut potential = 0.0_f64;
    for first in 0..bodies.len() {
        for second in first + 1..bodies.len() {
            let distance = (bodies[second].pos - bodies[first].pos).mag() as f64;
            potential -= bodies[first].mass as f64 * bodies[second].mass as f64 / distance;
        }
    }
    kinetic + potential
}

fn relative_scalar_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1.0e-12)
}

fn relative_vector_error(actual: Vec3, expected: Vec3) -> f32 {
    (actual - expected).mag() / expected.mag().max(1.0e-8)
}

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as u32) as f32 / ((1_u32 << 24) - 1) as f32
    }

    fn signed(&mut self, scale: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * scale
    }
}

fn deterministic_bodies(count: usize) -> Vec<Body> {
    let mut rng = DeterministicRng::new(0x9e37_79b9_7f4a_7c15);
    (0..count)
        .map(|index| {
            let separation = index as f32 * 1.0e-3;
            test_body(
                Vec3::new(
                    rng.signed(10.0) + separation,
                    rng.signed(10.0) - separation,
                    rng.signed(10.0) + separation * 0.5,
                ),
                Vec3::new(rng.signed(0.1), rng.signed(0.1), rng.signed(0.1)),
                0.25 + rng.unit() * 2.0,
            )
        })
        .collect()
}

fn production_simulation(mut bodies: Vec<Body>, dt: f32, epsilon: f32) -> Simulation {
    let mut config = InformationsConfig::default();
    config.n = 1;
    config.particle_budget = 8;
    config.gas_enabled = false;
    config.enable_galactic_atom_simulation = true;
    config.element_atomic_numbers = "6".to_string();
    config.element_atomic_count = 1;
    config.dt = dt;
    config.dt_min = dt * 0.1;
    config.dt_max = dt;
    config.epsilon = epsilon;
    config.theta = 0.5;
    config.inflow_strength = 0.0;
    config.restore_strength = 0.0;
    for body in &mut bodies {
        body.theta_i = config.theta;
        body.multipole_order = 1;
        body.local_dt = dt;
    }

    let mut simulation = Simulation::new(&config);
    simulation.replace_bodies_with_gpdm(bodies);
    simulation.attract();
    simulation
}

fn equal_mass_circular_orbit(steps: usize, dt: f32) -> (Vec<Body>, f64, f32, Vec3, Vec3, f32) {
    let orbital_speed = 0.5_f32.sqrt();
    let mut bodies = vec![
        test_body(
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.0, -orbital_speed, 0.0),
            1.0,
        ),
        test_body(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, orbital_speed, 0.0),
            1.0,
        ),
    ];
    let initial_energy = total_newtonian_energy(&bodies);
    let initial_angular_momentum = total_angular_momentum(&bodies).mag();
    let initial_momentum = total_momentum(&bodies);
    let initial_center_of_mass = center_of_mass(&bodies);
    let mut maximum_separation_error = 0.0_f32;

    for _ in 0..steps {
        advance_exact(&mut bodies, dt, 0.0);
        let separation = (bodies[1].pos - bodies[0].pos).mag();
        maximum_separation_error = maximum_separation_error.max((separation - 1.0).abs());
    }

    (
        bodies,
        initial_energy,
        initial_angular_momentum,
        initial_momentum,
        initial_center_of_mass,
        maximum_separation_error,
    )
}

#[test]
fn unit_softened_gravity_matches_plummer_reference() {
    let source = test_body(Vec3::new(-1.0, 0.5, 0.25), Vec3::zero(), 3.5);
    let query = Vec3::new(2.0, -0.75, 1.0);
    let epsilon = 0.4;
    let tree = build_tree(&[source], 0.0, epsilon);
    let actual = tree.acc(query);
    let expected = plummer_acceleration(source.pos - query, source.mass, epsilon);

    assert!(
        relative_vector_error(actual, expected) < 1.0e-6,
        "actual={actual:?}, expected={expected:?}"
    );
}

#[test]
fn equilibrium_disk_drift_factor_remains_in_range() {
    let config = InformationsConfig::default();
    let drift_low = GalaxyTemplate::disk_drift_factor(0.0, &config);
    let drift_high = GalaxyTemplate::disk_drift_factor(1.0, &config);
    assert!((0.5..=1.0).contains(&drift_low));
    assert!((0.5..=1.0).contains(&drift_high));
}

#[test]
fn equilibrium_sech2_height_sample_is_bounded() {
    let template = GalaxyTemplate::spiral(100);
    let scale_height = template.outer_radius * 0.03;
    let z = template.sample_sech2_height(scale_height);
    assert!(z.abs() <= scale_height * 10.0);
}

#[test]
fn unit_unsoftened_gravity_follows_inverse_square_law() {
    let source = test_body(Vec3::zero(), Vec3::zero(), 2.0);
    let tree = build_tree(&[source], 0.0, 0.0);
    let near = tree.acc(Vec3::new(2.0, 0.0, 0.0)).mag();
    let far = tree.acc(Vec3::new(4.0, 0.0, 0.0)).mag();

    assert!((near / far - 4.0).abs() < 1.0e-5);
}

#[test]
fn unit_zero_distance_force_is_finite_and_zero() {
    let source = test_body(Vec3::new(1.0, 2.0, 3.0), Vec3::zero(), 2.0);
    let tree = build_tree(&[source], 0.0, 0.2);
    let acceleration = tree.acc(source.pos);

    assert_eq!(acceleration, Vec3::zero());
    assert!(acceleration.x.is_finite());
    assert!(acceleration.y.is_finite());
    assert!(acceleration.z.is_finite());
}

#[test]
fn unit_octree_preserves_mass_and_center_of_mass() {
    let bodies = vec![
        test_body(Vec3::new(-2.0, 0.0, 1.0), Vec3::zero(), 2.0),
        test_body(Vec3::new(1.0, 3.0, -1.0), Vec3::zero(), 4.0),
        test_body(Vec3::new(3.0, -2.0, 0.5), Vec3::zero(), 1.5),
    ];
    let tree = build_tree(&bodies, 0.5, 0.1);
    let root = &tree.nodes[Octree::ROOT];

    assert!((root.mass as f64 - total_mass(&bodies)).abs() < 1.0e-6);
    assert!((root.pos - center_of_mass(&bodies)).mag() < VECTOR_TOLERANCE);
}

#[test]
fn unit_body_update_is_semi_implicit_euler() {
    let mut body = test_body(Vec3::new(1.0, -2.0, 0.5), Vec3::new(0.5, 1.0, -0.25), 1.0);
    body.acc = Vec3::new(2.0, -1.0, 0.5);
    body.update(0.25);

    let expected_velocity = Vec3::new(1.0, 0.75, -0.125);
    let expected_position = Vec3::new(1.25, -1.8125, 0.46875);
    assert!((body.vel - expected_velocity).mag() < 1.0e-6);
    assert!((body.pos - expected_position).mag() < 1.0e-6);
}

#[test]
fn physics_linear_momentum_is_conserved_in_isolated_orbit() {
    let (bodies, _, _, initial_momentum, _, _) = equal_mass_circular_orbit(4_443, 0.001);
    assert!((total_momentum(&bodies) - initial_momentum).mag() < 2.0e-5);
}

#[test]
fn physics_angular_momentum_has_bounded_numerical_drift() {
    let (bodies, _, initial, _, _, _) = equal_mass_circular_orbit(4_443, 0.001);
    let final_value = total_angular_momentum(&bodies).mag();
    assert!(relative_scalar_error(final_value as f64, initial as f64) < 2.0e-4);
}

#[test]
fn physics_energy_has_bounded_symplectic_drift() {
    let (bodies, initial, _, _, _, _) = equal_mass_circular_orbit(4_443, 0.001);
    let final_value = total_newtonian_energy(&bodies);
    assert!(relative_scalar_error(final_value, initial) < 5.0e-4);
}

#[test]
fn physics_center_of_mass_remains_stationary() {
    let (bodies, _, _, _, initial, _) = equal_mass_circular_orbit(4_443, 0.001);
    assert!((center_of_mass(&bodies) - initial).mag() < 2.0e-5);
}

#[test]
fn physics_mass_is_conserved_through_production_steps() {
    let mut simulation = production_simulation(deterministic_bodies(64), 0.0002, 0.05);
    let initial_mass = total_mass(&simulation.bodies);
    for _ in 0..20 {
        simulation.step();
    }
    assert_eq!(total_mass(&simulation.bodies), initial_mass);
}

#[test]
fn physics_merge_conserves_mass_and_linear_momentum() {
    let mut config = InformationsConfig::default();
    config.n = 1;
    config.particle_budget = 8;
    config.gas_enabled = false;
    config.enable_galactic_atom_simulation = true;
    config.element_atomic_numbers = "6".to_string();
    config.element_atomic_count = 1;
    let mut simulation = Simulation::new(&config);
    simulation.clear_bodies_with_gpdm();
    let b1 = test_body(
        Vec3::new(-0.0001, 0.0, 0.0),
        Vec3::new(0.8, 0.2, 0.0),
        2.0,
    );
    simulation.record_gpdm_event(crate::gpdm::host::HostEvent::Spawned { id: b1.id });
    simulation.bodies.push(b1);
    let b2 = test_body(
        Vec3::new(0.0001, 0.0, 0.0),
        Vec3::new(-0.3, -0.1, 0.0),
        3.0,
    );
    simulation.record_gpdm_event(crate::gpdm::host::HostEvent::Spawned { id: b2.id });
    simulation.bodies.push(b2);

    let initial_mass = total_mass(&simulation.bodies);
    let initial_momentum = total_momentum(&simulation.bodies);

    simulation.collide();
    simulation.collide();

    assert_eq!(simulation.bodies.len(), 1);
    assert!((total_mass(&simulation.bodies) - initial_mass).abs() < 1.0e-6);
    assert!((total_momentum(&simulation.bodies) - initial_momentum).mag() < 1.0e-6);
}

#[test]
fn property_exact_octree_matches_direct_summation() {
    let bodies = deterministic_bodies(96);
    let actual = tree_accelerations(&bodies, 0.0, 0.03);
    for (index, acceleration) in actual.into_iter().enumerate() {
        let expected = direct_acceleration(&bodies, index, 0.03);
        assert!(
            relative_vector_error(acceleration, expected) < 2.0e-5,
            "body {index}: actual={acceleration:?}, expected={expected:?}"
        );
    }
}

#[test]
fn property_pair_force_obeys_newtons_third_law() {
    let bodies = vec![
        test_body(Vec3::new(-1.0, 0.5, 0.0), Vec3::zero(), 2.0),
        test_body(Vec3::new(2.0, -0.25, 1.0), Vec3::zero(), 5.0),
    ];
    let accelerations = tree_accelerations(&bodies, 0.0, 0.1);
    let net_internal_force = accelerations[0] * bodies[0].mass + accelerations[1] * bodies[1].mass;

    assert!(net_internal_force.mag() < 2.0e-6);
}

#[test]
fn property_barnes_hut_rms_force_error_is_bounded() {
    let bodies = deterministic_bodies(256);
    let approximate = tree_accelerations(&bodies, 0.5, 0.03);
    let mut squared_error = 0.0_f64;
    let mut squared_reference = 0.0_f64;
    for (index, acceleration) in approximate.into_iter().enumerate() {
        let reference = direct_acceleration(&bodies, index, 0.03);
        squared_error += (acceleration - reference).mag_sq() as f64;
        squared_reference += reference.mag_sq() as f64;
    }
    let relative_rms = (squared_error / squared_reference.max(1.0e-18)).sqrt();
    assert!(
        relative_rms < 0.05,
        "relative RMS force error={relative_rms}"
    );
}

#[test]
fn property_default_adaptive_force_path_has_bounded_rms_error() {
    let bodies = deterministic_bodies(256);
    for multipole_order in 1..=3 {
        let approximate = adaptive_tree_accelerations(&bodies, 0.5, 0.03, multipole_order, true);
        let mut squared_error = 0.0_f64;
        let mut squared_reference = 0.0_f64;
        for (index, acceleration) in approximate.into_iter().enumerate() {
            let reference = direct_acceleration(&bodies, index, 0.03);
            squared_error += (acceleration - reference).mag_sq() as f64;
            squared_reference += reference.mag_sq() as f64;
        }
        let relative_rms = (squared_error / squared_reference.max(1.0e-18)).sqrt();
        assert!(
            relative_rms < 0.08,
            "order {multipole_order} adaptive RMS force error={relative_rms}"
        );
    }
}

#[test]
fn property_simulation_attract_matches_direct_reference_with_default_adaptation() {
    let bodies = deterministic_bodies(128);
    let simulation = production_simulation(bodies.clone(), 0.0002, 0.03);
    let mut squared_error = 0.0_f64;
    let mut squared_reference = 0.0_f64;
    for (index, body) in simulation.bodies.iter().enumerate() {
        let reference = direct_acceleration(&bodies, index, 0.03);
        squared_error += (body.acc - reference).mag_sq() as f64;
        squared_reference += reference.mag_sq() as f64;
    }
    let relative_rms = (squared_error / squared_reference.max(1.0e-18)).sqrt();
    assert!(
        relative_rms < 0.08,
        "production attract RMS force error={relative_rms}"
    );
}

#[test]
fn property_fusion_pressure_is_local_and_nonzero_for_close_bodies() {
    let close_bodies = vec![
        test_body(Vec3::new(-0.0005, 0.0, 0.0), Vec3::zero(), 1.0),
        test_body(Vec3::new(0.0005, 0.0, 0.0), Vec3::zero(), 1.0),
    ];
    let close_reference = direct_acceleration(&close_bodies, 0, 0.0001).mag();
    let close_simulation = production_simulation(close_bodies, 0.00001, 0.0001);
    let close_ratio = close_simulation.bodies[0].acc.mag() / close_reference;
    assert!(
        (0.987..=0.989).contains(&close_ratio),
        "close-range pressure ratio={close_ratio}"
    );

    let far_bodies = vec![
        test_body(Vec3::new(-0.005, 0.0, 0.0), Vec3::zero(), 1.0),
        test_body(Vec3::new(0.005, 0.0, 0.0), Vec3::zero(), 1.0),
    ];
    let far_reference = direct_acceleration(&far_bodies, 0, 0.0001).mag();
    let far_simulation = production_simulation(far_bodies, 0.00001, 0.0001);
    let far_ratio = far_simulation.bodies[0].acc.mag() / far_reference;
    assert!(
        (far_ratio - 1.0).abs() < 1.0e-5,
        "far-range pressure must vanish, ratio={far_ratio}"
    );
}

#[test]
fn property_elemental_tuned_self_mass_does_not_create_fusion_pressure() {
    let bodies = vec![
        test_element_body(Vec3::new(-0.005, 0.0, 0.0), 1.0, 0.2),
        test_element_body(Vec3::new(0.005, 0.0, 0.0), 1.0, 0.2),
    ];
    let epsilon = 0.0001;
    let expected = plummer_acceleration(
        bodies[1].pos - bodies[0].pos,
        bodies[1].effective_gravitational_mass(),
        epsilon,
    );
    let simulation = production_simulation(bodies, 0.00001, epsilon);

    assert!(
        relative_vector_error(simulation.bodies[0].acc, expected) < 1.0e-5,
        "isolated elemental body gained a self-pressure term"
    );
}

#[test]
fn property_gravity_is_translation_invariant() {
    let bodies = deterministic_bodies(48);
    let query = Vec3::new(15.0, -11.0, 8.0);
    let shift = Vec3::new(100.0, -250.0, 75.0);
    let original = build_tree(&bodies, 0.0, 0.05).acc(query);
    let shifted_bodies: Vec<Body> = bodies
        .iter()
        .map(|body| test_body(body.pos + shift, body.vel, body.mass))
        .collect();
    let shifted = build_tree(&shifted_bodies, 0.0, 0.05).acc(query + shift);

    assert!(relative_vector_error(shifted, original) < 5.0e-5);
}

#[test]
fn property_gravity_is_rotation_covariant() {
    let bodies = deterministic_bodies(48);
    let rotate = |value: Vec3| Vec3::new(-value.y, value.x, value.z);
    let query = Vec3::new(15.0, -11.0, 8.0);
    let original = build_tree(&bodies, 0.0, 0.05).acc(query);
    let rotated_bodies: Vec<Body> = bodies
        .iter()
        .map(|body| test_body(rotate(body.pos), rotate(body.vel), body.mass))
        .collect();
    let rotated = build_tree(&rotated_bodies, 0.0, 0.05).acc(rotate(query));

    assert!(relative_vector_error(rotated, rotate(original)) < 5.0e-5);
}

#[test]
fn property_bounded_inputs_never_produce_nan_or_infinity() {
    let mut bodies = deterministic_bodies(256);
    for _ in 0..20 {
        advance_exact(&mut bodies, 0.0002, 0.05);
        for body in &bodies {
            for value in [
                body.pos.x, body.pos.y, body.pos.z, body.vel.x, body.vel.y, body.vel.z, body.acc.x,
                body.acc.y, body.acc.z,
            ] {
                assert!(value.is_finite());
            }
        }
    }
}

#[test]
fn regression_softened_force_matches_independent_reference_snapshot() {
    let bodies = vec![
        test_body(Vec3::new(-1.0, 0.0, 0.0), Vec3::zero(), 2.0),
        test_body(Vec3::new(2.0, 1.0, -0.5), Vec3::zero(), 3.0),
        test_body(Vec3::new(0.0, -2.0, 1.0), Vec3::zero(), 0.75),
    ];
    let query = Vec3::new(0.5, 0.25, -0.25);
    let actual = build_tree(&bodies, 0.0, 0.25).acc(query);
    let expected = Vec3::new(0.084_965_46, 0.223_167_79, 0.033_724_54);

    assert!((actual - expected).mag() < 2.0e-6, "actual={actual:?}");
}

#[test]
fn regression_equal_mass_binary_remains_on_bounded_circular_orbit() {
    let (bodies, _, _, _, _, maximum_separation_error) = equal_mass_circular_orbit(4_443, 0.001);
    let final_separation = (bodies[1].pos - bodies[0].pos).mag();

    assert!(maximum_separation_error < 0.002);
    assert!((final_separation - 1.0).abs() < 0.002);
}

#[test]
fn physics_production_step_keeps_equal_mass_binary_bounded() {
    let orbital_speed = 0.5_f32.sqrt();
    let bodies = vec![
        test_body(
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.0, -orbital_speed, 0.0),
            1.0,
        ),
        test_body(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, orbital_speed, 0.0),
            1.0,
        ),
    ];
    let mut simulation = production_simulation(bodies, 0.001, 0.0);
    let initial_energy = total_newtonian_energy(&simulation.bodies);
    let initial_momentum = total_momentum(&simulation.bodies);
    let mut maximum_separation_error = 0.0_f32;
    for _ in 0..4_443 {
        simulation.step();
        let separation = (simulation.bodies[1].pos - simulation.bodies[0].pos).mag();
        maximum_separation_error = maximum_separation_error.max((separation - 1.0).abs());
    }

    let final_energy = total_newtonian_energy(&simulation.bodies);
    let energy_drift = relative_scalar_error(final_energy, initial_energy);
    assert!(
        maximum_separation_error < 0.01,
        "maximum separation error={maximum_separation_error}, energy drift={energy_drift}"
    );
    assert!(energy_drift < 0.005, "energy drift={energy_drift}");
    assert!((total_momentum(&simulation.bodies) - initial_momentum).mag() < 5.0e-5);
}

#[test]
fn physics_disk_equilibrium_uses_softened_circular_speed() {
    let mut config = InformationsConfig::default();
    config.n = 256;
    config.inner_radius = 0.5;
    config.outer_radius = 10.0;
    config.central_mass = 5.0;
    config.particle_mass_range = (1.0e-5, 2.0e-5);
    config.enable_disk_equilibrium_mode = true;
    config.disk_equilibrium_softening_ratio = 0.05;
    config.disk_equilibrium_asymmetric_drift_strength = 0.0;
    let template = GalaxyTemplate::from_config(&config);
    let axis = Vec3::new(0.3, 0.7, 0.64).normalized();
    for clockwise in [true, false] {
        let bodies = template.generate_inclined_with_config(
            ultraviolet::Vec2::zero(),
            ultraviolet::Vec2::zero(),
            axis,
            clockwise,
            &config,
        );

        let mut enclosed_mass = bodies[0].mass;
        let mut previous_radius = 0.0_f32;
        for body in bodies.iter().skip(1) {
            let offset = body.pos - bodies[0].pos;
            let radius = (offset - axis * offset.dot(axis)).mag();
            assert!(
                radius + 1.0e-6 >= previous_radius,
                "inclined disk is not ordered by orbital radius: {radius} < {previous_radius}"
            );
            let expected = GalaxyTemplate::softened_circular_speed(
                enclosed_mass,
                radius,
                config.effective_epsilon(),
            );
            assert!(
                relative_scalar_error(body.vel.mag() as f64, expected as f64) < 2.0e-5,
                "radius={radius}, actual={}, expected={expected}",
                body.vel.mag()
            );
            assert!(body.vel.dot(axis).abs() < 2.0e-5);
            enclosed_mass += body.mass;
            previous_radius = radius;
        }
    }
}

#[test]
fn physics_legacy_inclined_generation_uses_projected_orbital_radius() {
    let mut config = InformationsConfig::default();
    config.n = 256;
    config.inner_radius = 0.5;
    config.outer_radius = 10.0;
    config.central_mass = 5.0;
    config.particle_mass_range = (1.0e-5, 2.0e-5);
    config.enable_disk_equilibrium_mode = false;
    let template = GalaxyTemplate::from_config(&config);
    let axis = Vec3::new(0.3, 0.7, 0.64).normalized();

    for clockwise in [true, false] {
        let bodies = template.generate_inclined(
            ultraviolet::Vec2::zero(),
            ultraviolet::Vec2::zero(),
            axis,
            clockwise,
        );
        let mut enclosed_mass = bodies[0].mass;
        let mut previous_radius = 0.0_f32;
        for body in bodies.iter().skip(1) {
            let offset = body.pos - bodies[0].pos;
            let radius = (offset - axis * offset.dot(axis)).mag();
            assert!(radius + 1.0e-6 >= previous_radius);
            let expected = GalaxyTemplate::softened_circular_speed(enclosed_mass, radius, 0.0);
            assert!(relative_scalar_error(body.vel.mag() as f64, expected as f64) < 2.0e-5);
            assert!(body.vel.dot(axis).abs() < 2.0e-5);
            enclosed_mass += body.mass;
            previous_radius = radius;
        }
    }
}

#[test]
fn physics_elemental_mass_remap_rebalances_orbital_velocities() {
    let mut config = InformationsConfig::default();
    config.n = 256;
    config.inner_radius = 0.5;
    config.outer_radius = 10.0;
    config.central_mass = 5.0;
    config.particle_mass_range = (1.0e-5, 2.0e-5);
    config.enable_disk_equilibrium_mode = true;
    config.disk_equilibrium_softening_ratio = 0.05;
    config.disk_equilibrium_asymmetric_drift_strength = 0.0;
    let template = GalaxyTemplate::from_config(&config);
    let axis = Vec3::new(0.3, 0.7, 0.64).normalized();
    let mut bodies = template.generate_inclined_with_config(
        ultraviolet::Vec2::zero(),
        ultraviolet::Vec2::zero(),
        axis,
        true,
        &config,
    );
    for (index, body) in bodies.iter_mut().enumerate().skip(1) {
        body.mass = 1.0e-5 * (1 + index % 7) as f32;
    }

    Simulation::rebalance_elemental_galaxy_orbits(&mut bodies, &config, true);

    let mut enclosed_mass = bodies[0].mass;
    for body in bodies.iter().skip(1) {
        let offset = body.pos - bodies[0].pos;
        let radius = (offset - axis * offset.dot(axis)).mag();
        let expected = GalaxyTemplate::softened_circular_speed(
            enclosed_mass,
            radius,
            config.effective_epsilon(),
        );
        assert!(relative_scalar_error(body.vel.mag() as f64, expected as f64) < 2.0e-5);
        enclosed_mass += body.mass;
    }
}

#[test]
fn shader_optional_gas_billboard_artifact_compiles_and_validates() {
    let source = include_str!("../shaders/gas_billboard.glsl");
    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options {
        stage: naga::ShaderStage::Fragment,
        defines: Default::default(),
    };
    let module = frontend
        .parse(&options, source)
        .unwrap_or_else(|errors| panic!("GLSL parse failed: {errors:#?}"));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("GLSL validation failed");
}

#[test]
fn shader_production_cpu_billboard_alpha_layers_are_bounded() {
    let style = renderer::gas_billboard_style([90, 160, 220, 204], Vec3::new(4.0, -2.0, 1.0), 0.5);
    for color in [
        style.halo_color,
        style.tail_color,
        style.core_color,
        style.center_color,
    ] {
        assert_eq!(&color[..3], &[90, 160, 220]);
    }
    assert!(style.tail_color[3] <= style.halo_color[3]);
    assert!(style.halo_color[3] <= style.core_color[3]);
    assert!(style.core_color[3] <= style.center_color[3]);
    assert!(style.blur_offset.x.is_finite() && style.blur_offset.y.is_finite());
    assert!(style.blur_offset.mag() > 0.0);
}

#[test]
fn performance_body_memory_footprint_stays_bounded() {
    let bytes = std::mem::size_of::<Body>();
    assert!(bytes <= 192, "Body uses {bytes} bytes; budget is 192 bytes");
}

#[test]
fn performance_octree_storage_growth_is_linear() {
    let bodies = deterministic_bodies(4_096);
    let tree = build_tree(&bodies, 0.5, 0.03);
    assert!(
        tree.nodes.len() <= bodies.len() * 16,
        "node count {} exceeds linear storage budget for {} bodies",
        tree.nodes.len(),
        bodies.len()
    );
}

#[test]
#[ignore = "hardware-calibrated timing gate; run through ki_autonomie_testfälle.py --performance"]
fn performance_octree_force_pass_meets_calibrated_budget() {
    let bodies = deterministic_bodies(4_096);
    let budget_ms = std::env::var("SCIENTIFIC_PERFORMANCE_BUDGET_MS")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or(250);
    let start = Instant::now();
    let tree = build_tree(&bodies, 0.7, 0.03);
    let checksum = bodies
        .iter()
        .fold(Vec3::zero(), |sum, body| sum + tree.acc(body.pos));
    std::hint::black_box(checksum);
    let elapsed_ms = start.elapsed().as_millis();

    assert!(
        elapsed_ms <= budget_ms,
        "4,096-body Barnes-Hut pass took {elapsed_ms} ms; calibrated budget is {budget_ms} ms"
    );
}

#[test]
#[ignore = "hardware-calibrated scaling gate; run through ki_autonomie_testfälle.py --performance"]
fn performance_octree_force_scaling_remains_subquadratic() {
    fn elapsed_nanos(count: usize) -> u128 {
        let bodies = deterministic_bodies(count);
        let start = Instant::now();
        let tree = build_tree(&bodies, 0.7, 0.03);
        let checksum = bodies
            .iter()
            .fold(Vec3::zero(), |sum, body| sum + tree.acc(body.pos));
        std::hint::black_box(checksum);
        start.elapsed().as_nanos().max(1)
    }

    let small = elapsed_nanos(1_024);
    let large = elapsed_nanos(4_096);
    let growth = large as f64 / small as f64;
    assert!(
        growth < 12.0,
        "4x particle count caused {growth:.2}x runtime growth; subquadratic budget is <12x"
    );
}


