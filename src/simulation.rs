use crate::{
    body::Body,
    config::InformationsConfig,
    elements,
    galaxy_templates::GalaxyTemplate,
    quadtree::{Oct, Octree},
    renderer,
};

use rayon::prelude::*;
use std::collections::HashMap;
use ultraviolet::{Vec2, Vec3};

/// Tracks the per-pair state needed to continue running the merge/repeat/flyby
/// center-to-center dynamics and hydrodynamic inflow plane independently for
/// each of the (potentially many) galaxy pairs in the simulation.
#[derive(Clone, Copy)]
struct GalaxyPairState {
    /// Index of first galaxy center of this pair.
    center1_idx: usize,
    /// Index of second galaxy center of this pair, or `usize::MAX` once merged.
    center2_idx: usize,
    /// Start index (inclusive) of the contiguous body range belonging to this pair.
    range_start: usize,
    /// End index (exclusive) of the contiguous body range belonging to this pair.
    range_end: usize,
    /// Frames that the pair's center particles have remained in overlap contact.
    center_contact_frames: usize,
}

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub octree: Octree,
    /// Template used to derive accretion spawn parameters.
    accretion_template: GalaxyTemplate,
    config: InformationsConfig,
    /// Per-pair state for every galaxy pair in the simulation.
    pairs: Vec<GalaxyPairState>,
    /// Sustained contact counts for non-pair mergeable bodies across all segment types.
    merge_contact_frames: HashMap<(usize, usize), usize>,
}

impl Simulation {
    fn sanitize_vec(v: Vec3) -> Vec3 {
        Vec3::new(
            if v.x.is_finite() { v.x } else { 0.0 },
            if v.y.is_finite() { v.y } else { 0.0 },
            if v.z.is_finite() { v.z } else { 0.0 },
        )
    }

    fn initialize_adaptive_state(body: &mut Body, theta: f32, dt: f32) {
        body.theta_i = theta.max(0.05);
        body.multipole_order = 1;
        body.density = 0.0;
        body.jerk = Vec3::zero();
        body.prev_acc = Vec3::zero();
        body.energy_error = 0.0;
        body.local_dt = dt;
    }

    fn adaptive_force_enabled(&self) -> bool {
        self.config.enable_adaptive_theta
            || self.config.enable_adaptive_multipole
            || self.config.enable_mixed_precision
    }
 
    fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
        if a <= b { (a, b) } else { (b, a) }
    }
 
    fn plane_basis(axis: Vec3) -> (Vec3, Vec3) {
        let axis = axis.normalized();
        let arbitrary = if axis.x.abs() > axis.y.abs() {
            Vec3::new(axis.z, 0.0, -axis.x)
        } else {
            Vec3::new(0.0, -axis.z, axis.y)
        };
        let u = arbitrary.normalized();
        let v = axis.cross(u).normalized();
        (u, v)
    }
 
    fn is_center_pair(&self, i: usize, j: usize) -> bool {
        self.pairs.iter().any(|pair| {
            pair.center2_idx != usize::MAX
                && ((i == pair.center1_idx && j == pair.center2_idx)
                    || (i == pair.center2_idx && j == pair.center1_idx))
        })
    }

    fn merge_contact_threshold(segment_type: crate::body::ParticleSegmentType) -> usize {
        match segment_type {
            crate::body::ParticleSegmentType::Core => 6,
            crate::body::ParticleSegmentType::Bulge => 3,
            crate::body::ParticleSegmentType::Orbital => 2,
            crate::body::ParticleSegmentType::Gas => 3,
            crate::body::ParticleSegmentType::Satellite => 2,
            _ => 1,
        }
    }
  
    fn shift_contact_frames_after_removal(&mut self, removed_idx: usize) {
        let mut shifted = HashMap::with_capacity(self.merge_contact_frames.len());
        for ((a, b), count) in self.merge_contact_frames.drain() {
            if a == removed_idx || b == removed_idx {
                continue;
            }
            let a = if a > removed_idx { a - 1 } else { a };
            let b = if b > removed_idx { b - 1 } else { b };
            shifted.insert(Self::ordered_pair(a, b), count);
        }
        self.merge_contact_frames = shifted;
    }

    fn shift_contact_frames_after_insertion(&mut self, insert_idx: usize) {
        let mut shifted = HashMap::with_capacity(self.merge_contact_frames.len());
        for ((a, b), count) in self.merge_contact_frames.drain() {
            let a = if a >= insert_idx { a + 1 } else { a };
            let b = if b >= insert_idx { b + 1 } else { b };
            shifted.insert(Self::ordered_pair(a, b), count);
        }
        self.merge_contact_frames = shifted;
    }

    fn galaxy_body_membership(&self, body_idx: usize) -> Option<(usize, usize, usize, usize)> {
        for (pair_idx, pair) in self.pairs.iter().enumerate() {
            if body_idx < pair.range_start || body_idx >= pair.range_end {
                continue;
            }

            if pair.center2_idx == usize::MAX {
                return Some((pair_idx, pair.center1_idx, pair.range_start, pair.range_end));
            }

            let center_idx = if body_idx < pair.center2_idx {
                pair.center1_idx
            } else {
                pair.center2_idx
            };
            let range_start = if body_idx < pair.center2_idx {
                pair.range_start
            } else {
                pair.center2_idx
            };
            let range_end = if body_idx < pair.center2_idx {
                pair.center2_idx
            } else {
                pair.range_end
            };
            return Some((pair_idx, center_idx, range_start, range_end));
        }
        None
    }

    fn insert_body_at(&mut self, insert_idx: usize, body: Body) {
        self.bodies.insert(insert_idx, body);
        for pair in self.pairs.iter_mut() {
            if pair.center1_idx != usize::MAX && pair.center1_idx >= insert_idx {
                pair.center1_idx += 1;
            }
            if pair.center2_idx != usize::MAX && pair.center2_idx >= insert_idx {
                pair.center2_idx += 1;
            }
            if pair.range_start >= insert_idx {
                pair.range_start += 1;
            }
            if pair.range_end >= insert_idx {
                pair.range_end += 1;
            }
        }
        self.shift_contact_frames_after_insertion(insert_idx);
    }

    fn spawn_accretion_for_galaxy(
        &mut self,
        galaxy_center_idx: usize,
        galaxy_start: usize,
        galaxy_end: usize,
    ) {
        if self.accretion_template.accretion_spawn_rate <= 0.0 {
            return;
        }

        if galaxy_center_idx >= self.bodies.len() || galaxy_start >= galaxy_end {
            return;
        }

        let center = self.bodies[galaxy_center_idx];
        let plane_normal = self.galaxy_plane_normal(
            galaxy_start,
            galaxy_end,
            center.pos,
            center.rotation_axis,
            galaxy_center_idx,
        );
        let (u, v) = Self::plane_basis(plane_normal);

        let spawn_start_r = self.accretion_template.outer_ring_spawn_zone_inner_radius;
        let outer_r = self.accretion_template.outer_ring_spawn_zone_outer_radius;
        if spawn_start_r >= outer_r {
            return;
        }

        let angle = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        let r = spawn_start_r + fastrand::f32() * (outer_r - spawn_start_r);
        let thickness = outer_r * 0.03;
        let offset = (u * cos + v * sin) * r + plane_normal * ((fastrand::f32() - 0.5) * thickness);
        let spawn_pos = center.pos + offset;

        let tangent = if center.angular_speed >= 0.0 {
            -(plane_normal.cross(offset)).normalized()
        } else {
            (plane_normal.cross(offset)).normalized()
        };

        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;
        let mass = (mass_min + fastrand::f32() * (mass_max - mass_min)).clamp(mass_min, mass_max);
        let radius = mass.cbrt();
        let acc = self.octree.acc(spawn_pos);
        let orbital_speed = (acc.mag() * r).sqrt().max(0.0);
        let velocity = center.vel + tangent * orbital_speed;
        let angular_speed = (self.config.spawn_angular_speed_base
            + fastrand::f32() * self.config.spawn_angular_speed_range
            + orbital_speed * 0.02)
            * self.config.spin_speed_multiplier;

        let mut body = Body::new(
            spawn_pos,
            velocity,
            mass,
            radius,
            angular_speed,
            plane_normal,
            crate::body::ParticleSegmentType::Orbital,
        );
        Self::initialize_adaptive_state(&mut body, self.config.theta, self.dt);

        let insertion_idx = self.pairs.iter().find_map(|pair| {
            if galaxy_center_idx == pair.center1_idx || galaxy_center_idx == pair.center2_idx {
                Some(pair.range_end)
            } else {
                None
            }
        });

        if let Some(index) = insertion_idx {
            self.insert_body_at(index, body);
        } else {
            self.bodies.push(body);
        }
    }

    pub fn new(config: &InformationsConfig) -> Self {
        let dt = config.dt;
        let theta = config.theta;
        let epsilon = config.epsilon;

        let accretion_template = GalaxyTemplate::from_config(config);
        let config = config.clone();

        // Number of galaxies is rounded down to an even number (pairs), with a
        // hard minimum of one pair, so the existing pairwise merge/repeat/flyby
        // logic can be reused unchanged for every pair.
        let galaxy_count = config.galaxy_count.max(2) & !1;
        let num_pairs = galaxy_count / 2;

        let base_separation = accretion_template.outer_radius * config.galaxy_separation_factor;
        // Scatter radius for the chaotic distribution of pair midpoints through
        // the volume: grows with the number of pairs (cube-root scaling keeps
        // the average density roughly constant) and is tunable via
        // `galaxy_volume_scatter_factor`.
        let scatter_radius = base_separation
            * config.galaxy_volume_scatter_factor
            * (num_pairs as f32).cbrt().max(1.0);

        let mut bodies: Vec<Body> = Vec::new();
        let mut pairs: Vec<GalaxyPairState> = Vec::with_capacity(num_pairs);

        if config.enable_galactic_atom_simulation {
            let universe_radius = config.outer_radius * config.element_universe_scatter_factor;
            let center_mass_unit = config.element_center_mass_unit;
            let orbital_mass_unit = config.element_orbital_mass_unit;

            let mut atomic_numbers = config.selected_atomic_numbers();
            match config.element_initial_sorting_mode {
                1 => {
                    // sort by atomic number: default order
                }
                2 => {
                    atomic_numbers.sort_by(|&a, &b| {
                        let wa = elements::element_by_atomic_number(a)
                            .map(|e| e.atomic_weight)
                            .unwrap_or(0.0);
                        let wb = elements::element_by_atomic_number(b)
                            .map(|e| e.atomic_weight)
                            .unwrap_or(0.0);
                        wa.partial_cmp(&wb).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                _ => {}
            }

            let atomic_count = atomic_numbers.len();
            for atomic_number in atomic_numbers {
                let element = elements::element_by_atomic_number(atomic_number)
                    .expect("Element definitions must cover the enabled atomic range");
                let center = if atomic_count == 1 {
                    Vec3::zero()
                } else {
                    match config.element_initial_grouping_mode {
                        1 => {
                            let cluster_radius = universe_radius * config.element_group_spacing;
                            let cluster_center = match element.galaxy_morphology() {
                                elements::GalaxyMorphology::Dwarf => {
                                    Vec3::new(cluster_radius, 0.0, 0.0)
                                }
                                elements::GalaxyMorphology::Spiral => {
                                    Vec3::new(-cluster_radius, 0.0, 0.0)
                                }
                                elements::GalaxyMorphology::Elliptical => {
                                    Vec3::new(0.0, cluster_radius, 0.0)
                                }
                                elements::GalaxyMorphology::AGN => {
                                    Vec3::new(0.0, -cluster_radius, 0.0)
                                }
                            };
                            cluster_center + Self::random_point_in_ball(cluster_radius * 0.35)
                        }
                        2 => {
                            let fraction = atomic_number as f32 / atomic_count as f32;
                            let ring_radius = universe_radius * (0.15 + 0.65 * fraction);
                            Self::random_direction() * ring_radius
                        }
                        _ => Self::random_point_in_ball(universe_radius),
                    }
                };
                let rotation_axis = Self::random_inclination_axis();
                let mut atom_bodies = elements::generate_atomic_system(
                    element,
                    center,
                    rotation_axis,
                    center_mass_unit,
                    orbital_mass_unit,
                    config.element_radius_scale,
                    config.element_prime_alpha,
                    config.element_prime_beta,
                    config.element_light_energy,
                    config.particle_mass_range,
                );
                if config.element_group_internal_velocity_scale != 1.0 {
                    let scale = config.element_group_internal_velocity_scale;
                    for body in &mut atom_bodies {
                        if (body.pos - center).mag() > 0.1 {
                            body.vel *= scale;
                        }
                    }
                }
                bodies.extend(atom_bodies);
            }
        } else {
            for _ in 0..num_pairs {
                // Chaotic random midpoint for this pair, uniformly distributed
                // within a sphere of `scatter_radius` (rejection sampling).
                let midpoint = if num_pairs == 1 {
                    Vec3::zero()
                } else {
                    Self::random_point_in_ball(scatter_radius)
                };

                // Realistic 1x-2x separation distance between the paired galaxies,
                // vectorized along a random 3D direction instead of a fixed axis.
                let separation_dist = base_separation * (1.0 + fastrand::f32());
                let separation_dir = Self::random_direction();
                let half = separation_dir * (separation_dist * 0.5);
                let center_a3 = midpoint - half;
                let center_b3 = midpoint + half;

                let axis1 = Self::random_inclination_axis();
                let axis2 = Self::random_inclination_axis();
                let clockwise1 = fastrand::bool();
                let clockwise2 = fastrand::bool();

                let mut bodies_a = accretion_template.generate_inclined(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis1,
                    clockwise1,
                );
                let mut bodies_b = accretion_template.generate_inclined(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis2,
                    clockwise2,
                );

                // Translate each freshly generated disc from the origin to its
                // chaotic 3D placement within the volume.
                for body in &mut bodies_a {
                    body.pos += center_a3;
                }
                for body in &mut bodies_b {
                    body.pos += center_b3;
                }

                let m1: f32 = bodies_a.iter().map(|b| b.mass).sum();
                let m2: f32 = bodies_b.iter().map(|b| b.mass).sum();
                let center1 = bodies_a[0].pos;
                let center2 = bodies_b[0].pos;
                let (v1, v2) = Self::galaxy_bulk_velocities(&config, m1, m2, center1, center2);
                for body in &mut bodies_a {
                    body.vel += v1;
                }
                for body in &mut bodies_b {
                    body.vel += v2;
                }

                let range_start = bodies.len();
                let center1_idx = bodies.len();
                bodies.extend(bodies_a);
                let center2_idx = bodies.len();
                bodies.extend(bodies_b);
                let range_end = bodies.len();

                pairs.push(GalaxyPairState {
                    center1_idx,
                    center2_idx,
                    range_start,
                    range_end,
                    center_contact_frames: 0,
                });
            }
        }

        for body in &mut bodies {
            Self::initialize_adaptive_state(body, theta, dt);
        }

        let octree = Octree::new(theta, epsilon);

        Self {
            dt,
            frame: 0,
            bodies,
            octree,
            accretion_template,
            config,
            pairs,
            merge_contact_frames: HashMap::new(),
        }
    }

    fn random_inclination_axis() -> Vec3 {
        let theta = fastrand::f32() * std::f32::consts::PI * 0.5;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let x = theta.sin() * phi.cos();
        let y = theta.sin() * phi.sin();
        let z = theta.cos();
        Vec3::new(x, y, z).normalized()
    }

    /// Uniformly random unit direction over the full sphere.
    fn random_direction() -> Vec3 {
        let theta = fastrand::f32() * std::f32::consts::PI;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let x = theta.sin() * phi.cos();
        let y = theta.sin() * phi.sin();
        let z = theta.cos();
        Vec3::new(x, y, z).normalized()
    }

    /// Uniformly random point within a sphere of the given radius (rejection sampling).
    fn random_point_in_ball(radius: f32) -> Vec3 {
        loop {
            let p = Vec3::new(
                (fastrand::f32() * 2.0 - 1.0) * radius,
                (fastrand::f32() * 2.0 - 1.0) * radius,
                (fastrand::f32() * 2.0 - 1.0) * radius,
            );
            if p.mag_sq() <= radius * radius {
                return p;
            }
        }
    }

    fn ingest_spawn_queue(&mut self) {
        let mut queue = renderer::SPAWN_QUEUE.lock();
        if !queue.is_empty() {
            self.bodies.extend(queue.drain(..));
        }
    }

    fn galaxy_bulk_velocities(
        config: &InformationsConfig,
        m1: f32,
        m2: f32,
        center1: Vec3,
        center2: Vec3,
    ) -> (Vec3, Vec3) {
        let distance = (center2 - center1).mag().max(config.outer_radius * 6.0);
        let mu = 1.0;
        let escape_speed = ((2.0 * mu * (m1 + m2)) / distance).sqrt();

        let roll = fastrand::f32();
        let interaction_type = if roll < config.prob_merge {
            0
        } else if roll < config.prob_merge + config.prob_repeated {
            1
        } else {
            2
        };
        let (speed_factor, lateral_factor, angle) = match interaction_type {
            0 => (
                config.merge_speed_factor,
                config.merge_speed_factor * 0.1,
                config.merge_angle,
            ),
            1 => (
                config.repeated_speed_factor,
                config.repeated_speed_factor * 0.2,
                config.repeated_angle,
            ),
            _ => (
                config.flyby_speed_factor,
                config.flyby_speed_factor * 0.25,
                config.flyby_angle,
            ),
        };

        let direction = (center2 - center1).normalized();
        let perp = Vec3::new(-direction.y, direction.x, 0.0).normalized();
        let sign = if fastrand::bool() { 1.0 } else { -1.0 };

        let v_radial = direction * escape_speed * speed_factor;
        let v_tangent = perp * escape_speed * lateral_factor * sign;
        let v_rel = v_radial * angle.cos() + v_tangent * angle.sin();

        let v1 = v_rel * (m2 / (m1 + m2));
        let v2 = -v_rel * (m1 / (m1 + m2));

        (v1, v2)
    }

    pub fn step(&mut self) {
        self.ingest_spawn_queue();
        self.iterate();

        if self.frame % self.config.collision_interval == 0 {
            if renderer::COLLISIONS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                self.collide();
            }
        }

        if self.frame % self.config.attract_interval == 0 {
            self.attract();
            // Raumzeitkrümmungs-Dilatation für Center-Partikel überschreibt Teil der Gravitation
            self.apply_center_spacetime_dilation();

            if self.config.enable_adaptive_theta || self.config.enable_adaptive_multipole {
                self.classify_and_adapt();
            }
            if self.config.enable_adaptive_timestep {
                self.compute_adaptive_timesteps();
            }
        }

        if renderer::SPAWN_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            // Legacy paired galaxy disk mode now spawns accretion particles
            // automatically per merge within the same galaxy.
        }
        self.frame += 1;
    }

    pub fn attract(&mut self) {
        let oct = Oct::new_containing(&self.bodies);
        let reserve_nodes = self.bodies.len() * 6;
        let reserve_parents = self.bodies.len();
        self.octree.reserve(reserve_nodes, reserve_parents);
        self.octree.clear(oct);
        self.octree.adaptive_mixed_precision = self.config.enable_mixed_precision;

        for body in &self.bodies {
            let mass_value = if self.config.enable_elemental_gravitation_tuning {
                body.effective_gravitational_mass()
            } else {
                body.mass
            };
            self.octree.insert(body.pos, mass_value);
        }

        self.octree.propagate();

        let use_adaptive_force = self.adaptive_force_enabled();
        self.bodies.par_iter_mut().for_each(|body| {
            body.prev_acc = body.acc;
            body.acc = if use_adaptive_force {
                self.octree
                    .acc_adaptive(body.pos, body.theta_i, body.multipole_order)
            } else {
                self.octree.acc(body.pos)
            };
            body.acc = Simulation::sanitize_vec(body.acc);
        });

        self.apply_hydrodynamic_inflow();
    }

    fn classify_and_adapt(&mut self) {
        let base_theta = self.config.theta.max(0.05);
        let enable_adaptive_theta = self.config.enable_adaptive_theta;
        let enable_adaptive_multipole = self.config.enable_adaptive_multipole;
        let theta_quiet = self.config.adaptive_theta_quiet.max(0.05);
        let theta_medium = self.config.adaptive_theta_medium.max(0.05);
        let theta_critical = self.config.adaptive_theta_critical.max(0.05);
        let order_quiet = self.config.adaptive_multipole_quiet.clamp(1, 4);
        let order_medium = self.config.adaptive_multipole_medium.clamp(1, 4);
        let order_critical = self.config.adaptive_multipole_critical.clamp(1, 4);
        let quiet_threshold = self
            .config
            .adaptive_dynamics_quiet_threshold
            .clamp(0.0, 1.0);
        let critical_threshold = self
            .config
            .adaptive_dynamics_critical_threshold
            .max(quiet_threshold)
            .clamp(0.0, 1.0);
        let max_energy_error = self.config.max_energy_error.max(0.0);
        let feedback_theta_scale = self.config.adaptive_feedback_theta_scale.clamp(0.1, 1.0);
        let fallback_dt = self.dt;
        let density_radius_min = self.config.outer_radius * 0.01;
        let density_radius_max = self.config.outer_radius * 0.08;

        self.bodies.par_iter_mut().for_each(|body| {
            let acc_mag = body.acc.mag();
            let dynamics_score = acc_mag / (1.0 + acc_mag);
            let base_density_radius =
                self.config.outer_radius * (0.01 + 0.06 * (1.0 - dynamics_score));
            let density_radius = base_density_radius.clamp(density_radius_min, density_radius_max);
            let local_volume = (4.0 / 3.0) * std::f32::consts::PI * density_radius.powi(3);
            let (local_mass, local_quad) = self
                .octree
                .local_mass_and_quadrupole_within(body.pos, density_radius);

            let density_raw = local_mass / (local_volume + 1e-6);
            let density_score = (density_raw / (1.0 + density_raw)).clamp(0.0, 0.999);

            let quad_norm = local_quad.iter().map(|&q| q * q).sum::<f32>().sqrt();
            let anisotropy_raw = if local_mass > 1e-6 {
                quad_norm / (local_mass * density_radius * density_radius + 1e-6)
            } else {
                0.0
            };
            let anisotropy_score = (anisotropy_raw * 0.65).clamp(0.0, 0.999);
            let region_score = dynamics_score.max(density_score).max(anisotropy_score);

            body.density = density_score;
            let high_anisotropy = anisotropy_score >= critical_threshold;
            let medium_anisotropy = anisotropy_score >= quiet_threshold;
            let (mut theta_i, order_state) =
                if region_score >= critical_threshold || high_anisotropy {
                    (theta_critical, order_critical)
                } else if region_score >= quiet_threshold || medium_anisotropy {
                    (theta_medium, order_medium)
                } else {
                    (theta_quiet, order_quiet)
                };

            let mut multipole_order = if high_anisotropy {
                order_critical
            } else if medium_anisotropy {
                order_state.max(order_medium)
            } else {
                order_state
            };

            if !enable_adaptive_theta {
                theta_i = base_theta;
            }
            if !enable_adaptive_multipole {
                multipole_order = 1;
            }

            let prev_acc_mag = body.prev_acc.mag();
            body.energy_error = if prev_acc_mag > 1e-6 {
                ((acc_mag - prev_acc_mag).abs() / prev_acc_mag.max(1e-6)).min(f32::MAX)
            } else {
                0.0
            };

            if body.energy_error > max_energy_error {
                if enable_adaptive_theta {
                    theta_i = (theta_i * feedback_theta_scale).max(theta_critical);
                }
                if enable_adaptive_multipole {
                    multipole_order = multipole_order.saturating_add(1).clamp(1, 4);
                }
            }

            body.theta_i = theta_i.max(0.05);
            body.multipole_order = multipole_order.clamp(1, 4);
            if !body.local_dt.is_finite() || body.local_dt <= 0.0 {
                body.local_dt = fallback_dt;
            }
        });
    }

    fn compute_adaptive_timesteps(&mut self) {
        let estimate_dt = self.dt.abs().max(1e-6);
        let dt_min = self.config.dt_min.max(1e-6);
        let dt_max = self.config.dt_max.max(dt_min);
        let eta_timestep = self.config.eta_timestep.max(1e-4);
        let adaptive_enabled = self.config.enable_adaptive_timestep;
        let fallback_dt = self.dt;

        self.bodies.par_iter_mut().for_each(|body| {
            body.jerk = Simulation::sanitize_vec((body.acc - body.prev_acc) / estimate_dt);

            if adaptive_enabled {
                let acc_mag = body.acc.mag();
                let jerk_mag = body.jerk.mag();
                let candidate_dt = if acc_mag > 0.0 && jerk_mag > 1e-8 {
                    eta_timestep * (acc_mag / jerk_mag).sqrt()
                } else {
                    dt_max
                };
                let candidate_dt = if candidate_dt.is_finite() {
                    candidate_dt.clamp(dt_min, dt_max)
                } else {
                    dt_max
                };

                let old_dt = if body.local_dt.is_finite() && body.local_dt > 0.0 {
                    body.local_dt
                } else {
                    fallback_dt
                };
                let smoothing = 0.2_f32;
                body.local_dt = old_dt * (1.0 - smoothing) + candidate_dt * smoothing;
            } else {
                body.local_dt = fallback_dt;
            }
        });
    }

    fn apply_hydrodynamic_inflow(&mut self) {
        let outer_r = self.accretion_template.outer_radius.max(1.0);
        let inflow_strength = self.config.inflow_strength;
        let restore_strength = self.config.restore_strength;

        for pair in &self.pairs {
            let start = pair.range_start.min(self.bodies.len());
            let end = pair.range_end.min(self.bodies.len());
            if start >= end {
                continue;
            }

            if pair.center1_idx >= self.bodies.len() {
                continue;
            }
            let center1 = self.bodies[pair.center1_idx];
            // Der stabile Scheibenebenen-Normalenvektor wird nicht allein vom
            // zentralen Schwarzloch-Partikel abgeleitet, sondern aus der
            // tatsächlichen Bewegung der Scheibenpartikel in der Galaxie.
            let galaxy1_normal = self.galaxy_plane_normal(
                pair.center1_idx,
                if pair.center2_idx == usize::MAX {
                    end
                } else {
                    pair.center2_idx
                },
                center1.pos,
                center1.rotation_axis,
                pair.center1_idx,
            );

            let (center2, galaxy2_normal) =
                if pair.center2_idx != usize::MAX && pair.center2_idx < self.bodies.len() {
                    let center2 = self.bodies[pair.center2_idx];
                    let normal = self.galaxy_plane_normal(
                        pair.center2_idx,
                        end,
                        center2.pos,
                        center2.rotation_axis,
                        pair.center2_idx,
                    );
                    (Some(center2), normal)
                } else {
                    (None, galaxy1_normal)
                };

            self.bodies[start..end].par_iter_mut().for_each(|body| {
                let (plane_center, plane_normal) = if let Some(center2) = center2 {
                    let d1 = (body.pos - center1.pos).mag_sq();
                    let d2 = (body.pos - center2.pos).mag_sq();
                    if d1 <= d2 {
                        (center1.pos, galaxy1_normal)
                    } else {
                        (center2.pos, galaxy2_normal)
                    }
                } else {
                    (center1.pos, galaxy1_normal)
                };

                let normal = if plane_normal.mag_sq() > 1e-8 {
                    plane_normal.normalized()
                } else {
                    Vec3::new(0.0, 0.0, 1.0)
                };

                let offset = body.pos - plane_center;
                let dist_to_plane = offset.dot(normal);
                let in_plane_pos = offset - normal * dist_to_plane;
                let in_plane_dist = in_plane_pos.mag();

                let plane_attraction = -normal * dist_to_plane * restore_strength / outer_r;
                let radial_inflow = if in_plane_dist > 1e-3 {
                    -in_plane_pos / in_plane_dist
                        * inflow_strength
                        * (1.0 - (in_plane_dist / outer_r).clamp(0.0, 1.0))
                } else {
                    Vec3::zero()
                };

                body.acc += plane_attraction + radial_inflow;
            });
        }
    }

    fn galaxy_plane_normal(
        &self,
        range_start: usize,
        range_end: usize,
        center_pos: Vec3,
        fallback_axis: Vec3,
        center_idx: usize,
    ) -> Vec3 {
        // Berechne die Scheibenebene aus der Summe der lokalen Drehimpulse
        // der Scheibenpartikel. Damit wird das stabile Gleichgewicht nicht
        // ausschließlich vom Zentralobjekt abgeleitet, sondern von der
        // tatsächlichen Orientierung der Partikel im galaktischen Diskussystem.
        let mut normal = Vec3::zero();
        for idx in range_start..range_end.min(self.bodies.len()) {
            if idx == center_idx {
                continue;
            }
            let body = self.bodies[idx];
            let offset = body.pos - center_pos;
            if offset.mag_sq() <= 1e-8 {
                continue;
            }
            let contribution = offset.cross(body.vel) * body.mass;
            if contribution.mag_sq().is_finite() {
                normal += contribution;
            }
        }

        if normal.mag_sq() > 1e-8 {
            normal.normalized()
        } else if fallback_axis.mag_sq() > 1e-8 {
            fallback_axis.normalized()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        }
    }

    /// Raumzeitkrümmungs-Dilatation für Center-zu-Center Anziehung mit Spiralisierungs-Effekt
    /// Die Zentren nähern sich während ihrer Umkreisung an (orbitale Energie-Dissipation)
    /// Wird für jedes noch nicht verschmolzene Galaxienpaar unabhängig angewendet.
    fn apply_center_spacetime_dilation(&mut self) {
        let dissipation_factor = self.config.spacetime_dilation_factor * 0.001;

        for pair_idx in 0..self.pairs.len() {
            let (center1_idx, center2_idx) = {
                let pair = &self.pairs[pair_idx];
                (pair.center1_idx, pair.center2_idx)
            };

            if center2_idx == usize::MAX
                || center1_idx >= self.bodies.len()
                || center2_idx >= self.bodies.len()
            {
                continue;
            }

            let center1_pos = self.bodies[center1_idx].pos;
            let center1_vel = self.bodies[center1_idx].vel;
            let center1_mass = self.bodies[center1_idx].mass;
            let center2_pos = self.bodies[center2_idx].pos;
            let center2_vel = self.bodies[center2_idx].vel;
            let center2_mass = self.bodies[center2_idx].mass;

            let delta = center2_pos - center1_pos;
            let distance = delta.mag();
            let safe_distance = distance.max(0.3);

            if distance < 1e-3 {
                continue; // Zu nah, ignoriere
            }

            let direction = delta.normalized();

            // 1. GRAVITATIONS-KRAFT (anziehendes Potential)
            // Stärke wird begrenzt, um numerische Explosionen zu vermeiden
            let force_magnitude =
                ((center1_mass * center2_mass) / (safe_distance * safe_distance)).min(1e8);
            if !force_magnitude.is_finite() {
                continue;
            }
            let grav_force = direction * force_magnitude;

            // 2. DISSIPATIONS-KRAFT (Gravitationswellenstrahlung simulieren)
            let rel_vel = center2_vel - center1_vel;
            let radial_vel = rel_vel.dot(direction);
            let dissipation_force =
                -direction * (radial_vel.abs() * force_magnitude * dissipation_factor);

            let total_force = grav_force + dissipation_force;
            let total_force = Simulation::sanitize_vec(total_force);

            self.bodies[center1_idx].acc += total_force / center1_mass.max(1.0);
            self.bodies[center2_idx].acc -= total_force / center2_mass.max(1.0);

            eprintln!(
                "🌌 Spiral (Paar {}): dist={:.3}, grav={:.3}, dissipation={:.3}",
                pair_idx,
                distance,
                force_magnitude,
                dissipation_force.mag()
            );
        }
    }

    pub fn iterate(&mut self) {
        self.bodies.par_iter_mut().for_each(|body| {
            let dt = if self.config.enable_adaptive_timestep
                && body.local_dt.is_finite()
                && body.local_dt > 0.0
            {
                body.local_dt
            } else {
                self.dt
            };
            body.update(dt);
        });
    }

    pub fn collide(&mut self) {
        if self.bodies.len() < 2 {
            return;
        }
 
        let mut center_merge_candidates: Vec<(usize, usize)> = Vec::new();
        for pair in self.pairs.iter_mut() {
            if pair.center2_idx != usize::MAX
                && pair.center1_idx < self.bodies.len()
                && pair.center2_idx < self.bodies.len()
            {
                let b1 = self.bodies[pair.center1_idx];
                let b2 = self.bodies[pair.center2_idx];
                let sum_radius = b1.effective_radius() + b2.effective_radius();
                if (b2.pos - b1.pos).mag_sq() <= sum_radius * sum_radius {
                    pair.center_contact_frames += 1;
                } else {
                    pair.center_contact_frames = 0;
                }
                if pair.center_contact_frames > 5 {
                    center_merge_candidates.push((pair.center1_idx, pair.center2_idx));
                }
            } else {
                pair.center_contact_frames = 0;
            }
        }
 
        center_merge_candidates.sort_unstable_by(|&(a1, b1), &(a2, b2)| {
            let max1 = a1.max(b1);
            let max2 = a2.max(b2);
            max2.cmp(&max1)
        });
        center_merge_candidates.dedup();
        for &(i, j) in &center_merge_candidates {
            if i < self.bodies.len() && j < self.bodies.len() {
                self.merge_center_particles(i, j);
            }
        }
 
        if self.bodies.len() < 2 {
            return;
        }
 
        let mut next_merge_contact_frames: HashMap<(usize, usize), usize> = HashMap::new();
        let mut core_merge_candidates: Vec<(usize, usize)> = Vec::new();
        let mut particle_merge_pairs: Vec<(usize, usize)> = Vec::new();
 
        let max_radius = self
            .bodies
            .iter()
            .map(|body| body.effective_radius())
            .fold(0.0_f32, f32::max)
            .max(1.0);
        let mut cell_size = max_radius * 3.0;

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;
        for body in &self.bodies {
            min_x = min_x.min(body.pos.x);
            min_y = min_y.min(body.pos.y);
            min_z = min_z.min(body.pos.z);
            max_x = max_x.max(body.pos.x);
            max_y = max_y.max(body.pos.y);
            max_z = max_z.max(body.pos.z);
        }

        min_x -= cell_size;
        min_y -= cell_size;
        min_z -= cell_size;
        max_x += cell_size;
        max_y += cell_size;
        max_z += cell_size;

        if !min_x.is_finite()
            || !min_y.is_finite()
            || !min_z.is_finite()
            || !max_x.is_finite()
            || !max_y.is_finite()
            || !max_z.is_finite()
        {
            eprintln!("⚠ Collision skipped: invalid body bounds");
            return;
        }

        let size_x = ((max_x - min_x) / cell_size).ceil();
        let size_y = ((max_y - min_y) / cell_size).ceil();
        let size_z = ((max_z - min_z) / cell_size).ceil();
        if !size_x.is_finite() || !size_y.is_finite() || !size_z.is_finite() {
            eprintln!("⚠ Collision skipped: invalid grid dimensions");
            return;
        }

        let mut size_x = size_x as usize + 1;
        let mut size_y = size_y as usize + 1;
        let mut size_z = size_z as usize + 1;
        let max_grid_dim = 128_usize;
        let max_total_cells = max_grid_dim
            .saturating_mul(max_grid_dim)
            .saturating_mul(max_grid_dim);

        if size_x > max_grid_dim || size_y > max_grid_dim || size_z > max_grid_dim {
            let scale = ((size_x.max(size_y).max(size_z)) as f32 / max_grid_dim as f32).ceil();
            cell_size *= scale;
            let size_xf = ((max_x - min_x) / cell_size).ceil();
            let size_yf = ((max_y - min_y) / cell_size).ceil();
            let size_zf = ((max_z - min_z) / cell_size).ceil();
            if !size_xf.is_finite() || !size_yf.is_finite() || !size_zf.is_finite() {
                eprintln!("⚠ Collision skipped: invalid scaled grid dimensions");
                return;
            }
            size_x = size_xf as usize + 1;
            size_y = size_yf as usize + 1;
            size_z = size_zf as usize + 1;
        }

        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells == 0 || total_cells > max_total_cells {
            eprintln!(
                "⚠ Collision skipped: grid cell count out of bounds ({}, {}, {})",
                size_x, size_y, size_z
            );
            return;
        }

        let mut cells = Vec::with_capacity(total_cells);
        cells.resize_with(total_cells, Vec::new);

        let cell_index = |pos: Vec3| -> Option<usize> {
            if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
                return None;
            }
            let ix = ((pos.x - min_x) / cell_size).floor();
            let iy = ((pos.y - min_y) / cell_size).floor();
            let iz = ((pos.z - min_z) / cell_size).floor();
            if ix.is_sign_negative() || iy.is_sign_negative() || iz.is_sign_negative() {
                return None;
            }
            let ix = ix as usize;
            let iy = iy as usize;
            let iz = iz as usize;
            if ix >= size_x || iy >= size_y || iz >= size_z {
                return None;
            }
            Some((ix * size_y + iy) * size_z + iz)
        };

        for (index, body) in self.bodies.iter().enumerate() {
            if let Some(idx) = cell_index(body.pos) {
                cells[idx].push(index);
            }
        }

        for x in 0..size_x {
            for y in 0..size_y {
                for z in 0..size_z {
                    let cell_idx = (x * size_y + y) * size_z + z;
                    let indices = &cells[cell_idx];
                    if indices.is_empty() {
                        continue;
                    }

                    for dx in 0..=1 {
                        for dy in 0..=1 {
                            for dz in 0..=1 {
                                let nx = x + dx;
                                let ny = y + dy;
                                let nz = z + dz;
                                if nx >= size_x || ny >= size_y || nz >= size_z {
                                    continue;
                                }
                                let neighbor_idx = (nx * size_y + ny) * size_z + nz;
                                if neighbor_idx >= cells.len() {
                                    continue;
                                }
                                let neighbor_indices = &cells[neighbor_idx];
                                if neighbor_indices.is_empty() {
                                    continue;
                                }

                                for &i in indices {
                                    for &j in neighbor_indices {
                                        if neighbor_idx == cell_idx && i >= j {
                                            continue;
                                        }
                                        self.resolve(
                                            i,
                                            j,
                                            &mut next_merge_contact_frames,
                                            &mut core_merge_candidates,
                                            &mut particle_merge_pairs,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
 
        self.merge_contact_frames = next_merge_contact_frames;
 
        core_merge_candidates.sort_unstable();
        core_merge_candidates.dedup();
        particle_merge_pairs.sort_unstable();
        particle_merge_pairs.dedup();
 
        enum MergeAction {
            Center(usize, usize),
            Particle(usize, usize),
        }
 
        let mut merge_actions: Vec<MergeAction> = Vec::new();
        merge_actions.extend(center_merge_candidates.into_iter().map(|(a, b)| MergeAction::Center(a, b)));
        merge_actions.extend(core_merge_candidates.into_iter().map(|(a, b)| MergeAction::Particle(a, b)));
        merge_actions.extend(particle_merge_pairs.into_iter().map(|(a, b)| MergeAction::Particle(a, b)));
        merge_actions.sort_unstable_by(|a, b| {
            let (a1, a2) = match a {
                MergeAction::Center(x, y) | MergeAction::Particle(x, y) => (*x, *y),
            };
            let (b1, b2) = match b {
                MergeAction::Center(x, y) | MergeAction::Particle(x, y) => (*x, *y),
            };
            let max_a = a1.max(a2);
            let max_b = b1.max(b2);
            max_b.cmp(&max_a).then_with(|| a2.cmp(&b2)).then_with(|| a1.cmp(&b1))
        });
 
        let mut merged = vec![false; self.bodies.len()];
        for action in merge_actions {
            let (first, second, is_center) = match action {
                MergeAction::Center(a, b) => (a, b, true),
                MergeAction::Particle(a, b) => (a, b, false),
            };
            if first >= self.bodies.len() || second >= self.bodies.len() {
                continue;
            }
            if merged[first] || merged[second] {
                continue;
            }
            if is_center {
                self.merge_center_particles(first, second);
            } else {
                self.merge_particle_bodies(first, second);
            }
            merged[first] = true;
            merged[second] = true;
        }
    }

    /// Spawns accretion particles in the outer ring spawn zone.
    ///
    /// On each step rolls against `accretion_spawn_rate`. A successful roll places
    /// one new particle at a random position within the outer ring spawn zone
    /// (outer third of the galactic disk) and assigns it a circular orbital velocity
    /// derived from the local gravitational acceleration reported by the octree.
    /// Spawns are placed in absolute disk coordinates, avoiding the central bulge.
    #[allow(dead_code)]
    fn spawn_accretion(&mut self) {
        let spawn_start_r = self.accretion_template.outer_ring_spawn_zone_inner_radius;
        let outer_r = self.accretion_template.outer_ring_spawn_zone_outer_radius;
        if spawn_start_r >= outer_r {
            eprintln!(
                "⚠ Spawn-Zone ungültig: start_r >= outer_r ({:.3} >= {:.3})",
                spawn_start_r, outer_r
            );
            return;
        }

        let spawn_rate = self.accretion_template.accretion_spawn_rate;
        if spawn_rate <= 0.0 {
            eprintln!("⚠ Spawn deaktiviert: accretion_spawn_rate = {}", spawn_rate);
            return;
        }

        if fastrand::f32() > spawn_rate {
            return;
        }

        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;

        // Spawn in absolute disk coordinates, not relative to detected centers.
        // This ensures spawns occur in the outer third of the galactic disk.
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let r = spawn_start_r + fastrand::f32() * (outer_r - spawn_start_r);
        let spawn_pos = Vec3::new(cos * r, sin * r, (fastrand::f32() - 0.5) * outer_r * 0.03);
        // Tangent direction for a clockwise orbit in the disk plane.
        let tangent = Vec3::new(sin, -cos, 0.0);

        let mass = mass_min + fastrand::f32() * (mass_max - mass_min);
        let radius = mass.cbrt();

        let acc = self.octree.acc(spawn_pos);
        let orbital_speed = (acc.mag() * r).sqrt();
        let vel = tangent * orbital_speed;

        let angular_speed = (self.config.spawn_angular_speed_base
            + fastrand::f32() * self.config.spawn_angular_speed_range
            + orbital_speed * 0.02)
            * self.config.spin_speed_multiplier;
        let mut body = Body::new(
            spawn_pos,
            vel,
            mass,
            radius,
            angular_speed,
            Vec3::new(0.0, 0.0, 1.0),
            crate::body::ParticleSegmentType::Orbital,
        );
        Self::initialize_adaptive_state(&mut body, self.config.theta, self.dt);
        self.bodies.push(body);
    }

    fn resolve(
        &mut self,
        i: usize,
        j: usize,
        next_merge_contact_frames: &mut HashMap<(usize, usize), usize>,
        core_merge_candidates: &mut Vec<(usize, usize)>,
        particle_merge_pairs: &mut Vec<(usize, usize)>,
    ) {
        if self.is_center_pair(i, j) {
            return;
        }
 
        let b1 = &self.bodies[i];
        let b2 = &self.bodies[j];
 
        let p1 = b1.pos;
        let p2 = b2.pos;
 
        let r1 = b1.effective_radius();
        let r2 = b2.effective_radius();
 
        let d = p2 - p1;
        let r = r1 + r2;
 
        if d.mag_sq() > r * r {
            return;
        }
 
        if b1.segment_type == crate::body::ParticleSegmentType::Core
            && b2.segment_type == crate::body::ParticleSegmentType::Core
            && b1.can_merge_with(b2)
        {
            let key = Self::ordered_pair(i, j);
            let current = self.merge_contact_frames.get(&key).copied().unwrap_or(0);
            let count = next_merge_contact_frames.entry(key).or_insert(current + 1);
            if *count >= Self::merge_contact_threshold(b1.segment_type) {
                core_merge_candidates.push(key);
            }
            return;
        }
  
        if b1.can_merge_with(b2) {
            let key = Self::ordered_pair(i, j);
            let current = self.merge_contact_frames.get(&key).copied().unwrap_or(0);
            let count = next_merge_contact_frames.entry(key).or_insert(current + 1);
            if *count >= Self::merge_contact_threshold(b1.segment_type) {
                let mut first = i;
                let mut second = j;
                if first > second {
                    std::mem::swap(&mut first, &mut second);
                }
                particle_merge_pairs.push((first, second));
            }
            return;
        }
 
        let v1 = b1.vel;
        let v2 = b2.vel;

        let v = v2 - v1;

        let d_dot_v = d.dot(v);

        let m1 = b1.mass;
        let m2 = b2.mass;

        let weight1 = m2 / (m1 + m2);
        let weight2 = m1 / (m1 + m2);

        if d_dot_v >= 0.0 && d != Vec3::zero() {
            let tmp = d * (r / d.mag() - 1.0);
            self.bodies[i].pos -= weight1 * tmp;
            self.bodies[j].pos += weight2 * tmp;
            return;
        }

        let v_sq = v.mag_sq();
        let d_sq = d.mag_sq();
        let r_sq = r * r;

        if !v_sq.is_finite() || v_sq <= 0.0 {
            eprintln!("⚠ Collision resolution skipped: invalid relative speed");
            return;
        }

        let discriminant = (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0);
        let t = (d_dot_v + discriminant.sqrt()) / v_sq;
        if !t.is_finite() {
            eprintln!("⚠ Collision resolution skipped: invalid time step");
            return;
        }

        self.bodies[i].pos -= v1 * t;
        self.bodies[j].pos -= v2 * t;

        let p1 = self.bodies[i].pos;
        let p2 = self.bodies[j].pos;
        let d = p2 - p1;
        let d_dot_v = d.dot(v);
        let d_sq = d.mag_sq();

        if d_sq > 1e-8 {
            let tmp = d * (1.5 * d_dot_v / d_sq);
            let v1 = v1 + tmp * weight1;
            let v2 = v2 - tmp * weight2;

            self.bodies[i].vel = v1;
            self.bodies[j].vel = v2;
            self.bodies[i].pos += v1 * t;
            self.bodies[j].pos += v2 * t;
        } else {
            eprintln!("⚠ Collision fallback applied: bodies nearly coincident");
            let average_vel = (v1 + v2) * 0.5;
            self.bodies[i].vel = average_vel;
            self.bodies[j].vel = average_vel;
            self.bodies[i].pos += v1 * t;
            self.bodies[j].pos += v2 * t;
        }
    }

    fn merge_particle_bodies(&mut self, i: usize, j: usize) {
        let mut first = i;
        let mut second = j;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
 
        let merge_membership = if let (Some((pair_idx_a, center_idx_a, start_a, end_a)), Some((pair_idx_b, center_idx_b, start_b, end_b))) = (
            self.galaxy_body_membership(first),
            self.galaxy_body_membership(second),
        ) {
            if pair_idx_a == pair_idx_b && center_idx_a == center_idx_b {
                Some((pair_idx_a, center_idx_a, start_a, end_a))
            } else {
                None
            }
        } else {
            None
        };
 
        let b1 = self.bodies[first];
        let b2 = self.bodies[second];
        let total_mass = b1.mass + b2.mass;
        let merged_pos = (b1.pos * b1.mass + b2.pos * b2.mass) / total_mass;
        let merged_vel = (b1.vel * b1.mass + b2.vel * b2.mass) / total_mass;
 
        let orbital_axis = (b2.pos - b1.pos).cross(b2.vel - b1.vel);
        let axis = if orbital_axis.mag_sq() > 1e-8 {
            orbital_axis.normalized()
        } else {
            let axis_sum = b1.rotation_axis * b1.mass + b2.rotation_axis * b2.mass;
            if axis_sum.mag_sq() > 1e-8 {
                axis_sum.normalized()
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            }
        };
 
        let merged_radius = (b1.base_radius.powi(3) + b2.base_radius.powi(3)).cbrt();
        let base_merged_angular_speed =
            (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        let merged_angular_speed = if !self.config.enable_galactic_atom_simulation {
            base_merged_angular_speed * 1.5
        } else {
            base_merged_angular_speed
        };
        let mut merged_body = Body::new_element(
            merged_pos,
            merged_vel,
            total_mass,
            merged_radius,
            merged_angular_speed,
            axis,
            b1.element_atomic_number,
            b1.segment_type,
            (b1.element_prime_energy * b1.mass + b2.element_prime_energy * b2.mass) / total_mass,
            (b1.element_reactivity * b1.mass + b2.element_reactivity * b2.mass) / total_mass,
            (b1.element_magnetism * b1.mass + b2.element_magnetism * b2.mass) / total_mass,
            (b1.element_gravitation * b1.mass + b2.element_gravitation * b2.mass) / total_mass,
            (b1.element_light * b1.mass + b2.element_light * b2.mass) / total_mass,
            (b1.element_mass_dimension * b1.mass + b2.element_mass_dimension * b2.mass) / total_mass,
        );
        Self::initialize_adaptive_state(&mut merged_body, self.config.theta, self.dt);
        self.bodies[first] = merged_body;
        self.bodies.remove(second);
        self.shift_pair_indices_after_removal(second);
        self.shift_contact_frames_after_removal(second);

        if let Some((pair_idx, center_idx, _, _)) = merge_membership {
            let adjusted_center_idx = if center_idx > second { center_idx - 1 } else { center_idx };
            if let Some(pair) = self.pairs.get(pair_idx) {
                self.spawn_accretion_for_galaxy(
                    adjusted_center_idx,
                    pair.range_start,
                    pair.range_end,
                );
            }
        }
    }
 
    fn shift_pair_indices_after_removal(&mut self, removed_idx: usize) {
        for pair in self.pairs.iter_mut() {
            if pair.center1_idx == removed_idx {
                pair.center1_idx = usize::MAX;
            } else if pair.center1_idx > removed_idx {
                pair.center1_idx -= 1;
            }
            if pair.center2_idx == removed_idx {
                pair.center2_idx = usize::MAX;
            } else if pair.center2_idx > removed_idx {
                pair.center2_idx -= 1;
            }
            if pair.range_start > removed_idx {
                pair.range_start -= 1;
            }
            if pair.range_end > removed_idx {
                pair.range_end -= 1;
            }
        }
    }
 
    fn merge_center_particles(&mut self, i: usize, j: usize) {
        let mut first = i;
        let mut second = j;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
 
        let b1 = self.bodies[first];
        let b2 = self.bodies[second];
        let total_mass = b1.mass + b2.mass;
        let merged_pos = (b1.pos * b1.mass + b2.pos * b2.mass) / total_mass;
        let merged_vel = (b1.vel * b1.mass + b2.vel * b2.mass) / total_mass;

        let orbital_axis = (b2.pos - b1.pos).cross(b2.vel - b1.vel);
        let axis = if orbital_axis.mag_sq() > 1e-8 {
            orbital_axis.normalized()
        } else {
            let axis_sum = b1.rotation_axis * b1.mass + b2.rotation_axis * b2.mass;
            if axis_sum.mag_sq() > 1e-8 {
                axis_sum.normalized()
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            }
        };

        let merged_radius = (b1.base_radius.powi(3) + b2.base_radius.powi(3)).cbrt();
        let base_merged_angular_speed =
            (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        let merged_angular_speed = if !self.config.enable_galactic_atom_simulation {
            base_merged_angular_speed * 1.5
        } else {
            base_merged_angular_speed
        };
        let mut merged_body = Body::new(
            merged_pos,
            merged_vel,
            total_mass,
            merged_radius,
            merged_angular_speed,
            axis,
            crate::body::ParticleSegmentType::Core,
        );
        Self::initialize_adaptive_state(&mut merged_body, self.config.theta, self.dt);
        self.bodies[first] = merged_body;
        self.bodies.remove(second);
        self.shift_contact_frames_after_removal(second);
 
        // Find which pair this merge belongs to and update its state; also
        // shift every other pair's stored indices/ranges to account for the
        // single body removed from `self.bodies` at index `second`.
        let mut merged_pair_idx = None;
        for (idx, pair) in self.pairs.iter().enumerate() {
            if pair.center2_idx != usize::MAX
                && ((pair.center1_idx == first && pair.center2_idx == second)
                    || (pair.center1_idx == second && pair.center2_idx == first))
            {
                merged_pair_idx = Some(idx);
                break;
            }
        }

        for (idx, pair) in self.pairs.iter_mut().enumerate() {
            if Some(idx) == merged_pair_idx {
                pair.center1_idx = first;
                pair.center2_idx = usize::MAX;
                if pair.range_end > second {
                    pair.range_end -= 1;
                }
                continue;
            }

            if pair.center1_idx > second {
                pair.center1_idx -= 1;
            }
            if pair.center2_idx != usize::MAX && pair.center2_idx > second {
                pair.center2_idx -= 1;
            }
            if pair.range_start > second {
                pair.range_start -= 1;
            }
            if pair.range_end > second {
                pair.range_end -= 1;
            }
        }

        eprintln!(
            "🔗 Center merge: mass={} pos={:?} axis={:?} inflow-plane-normal={:?}",
            total_mass, merged_pos, axis, axis
        );
    }
 }
 
 #[cfg(test)]
    mod tests {
        use super::*;
        use crate::body::ParticleSegmentType;
        use crate::config::InformationsConfig;
        use ultraviolet::Vec3;
 
        #[test]
        fn core_particles_merge_only_after_more_than_five_frames_of_contact() {
            let mut sim = Simulation::new(&InformationsConfig::default());
            sim.bodies.clear();
            sim.pairs.clear();
            sim.merge_contact_frames.clear();
 
            let body1 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                1,
                ParticleSegmentType::Core,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            let body2 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                1,
                ParticleSegmentType::Core,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            sim.bodies.push(body1);
            sim.bodies.push(body2);
  
            for _ in 0..5 {
                sim.collide();
                assert_eq!(sim.bodies.len(), 2, "core bodies should not merge before frame 6");
            }
  
            sim.collide();
            assert_eq!(sim.bodies.len(), 1, "core bodies should merge after sustained contact");
        }
 
        #[test]
        fn particle_bodies_merge_only_when_same_atomic_number_and_segment() {
            let mut sim = Simulation::new(&InformationsConfig::default());
            sim.bodies.clear();
            sim.pairs.clear();
            sim.merge_contact_frames.clear();
 
            let body1 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Orbital,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            let body2 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Orbital,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            sim.bodies.push(body1);
            sim.bodies.push(body2);
 
            sim.collide();
            assert_eq!(sim.bodies.len(), 2, "same segment and element particles should not merge on first frame due to contact threshold");
            sim.collide();
            assert_eq!(sim.bodies.len(), 1, "same segment and element particles should merge after sustained contact");
        }
 
        #[test]
        fn pgd_merge_increases_child_spin_by_factor_1_5() {
            let mut sim = Simulation::new(&InformationsConfig::default());
            sim.bodies.clear();
            sim.pairs.clear();
            sim.merge_contact_frames.clear();
 
            let body1 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                1.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Orbital,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            let body2 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                2.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Orbital,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            sim.bodies.push(body1);
            sim.bodies.push(body2);
 
            sim.collide();
            assert_eq!(sim.bodies.len(), 2, "first contact should not yet merge orbital particles");
 
            sim.collide();
            assert_eq!(sim.bodies.len(), 1, "second contact should merge orbital particles");
            assert_eq!(sim.bodies[0].angular_speed, 2.25);
        }
 
        #[test]
        fn particle_bodies_do_not_merge_across_segments() {
            let mut sim = Simulation::new(&InformationsConfig::default());
            sim.bodies.clear();
            sim.pairs.clear();
            sim.merge_contact_frames.clear();
 
            let body1 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Orbital,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            let body2 = Body::new_element(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                6,
                ParticleSegmentType::Gas,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
                1.0,
            );
            sim.bodies.push(body1);
            sim.bodies.push(body2);
 
            sim.collide();
            assert_eq!(sim.bodies.len(), 2, "different segment particles should not merge on contact");
        }
}
