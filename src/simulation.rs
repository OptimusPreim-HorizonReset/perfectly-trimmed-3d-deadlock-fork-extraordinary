use crate::{
    body::Body,
    config::InformationsConfig,
    galaxy_templates::GalaxyTemplate,
    quadtree::{Oct, Octree},
    renderer,
};

use rayon::prelude::*;
use ultraviolet::{Vec2, Vec3};

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub octree: Octree,
    /// Template used to derive accretion spawn parameters.
    accretion_template: GalaxyTemplate,
    config: InformationsConfig,
    /// Index of first galaxy center
    center1_idx: usize,
    /// Index of second galaxy center
    center2_idx: usize,
    /// Resulting equatorial plane normal for hydrodynamic inflow alignment.
    equatorial_plane_normal: Vec3,
    /// Position through which the equatorial plane passes.
    equatorial_plane_center: Vec3,
}

impl Simulation {
    pub fn new(config: &InformationsConfig) -> Self {
        let dt = config.dt;
        let theta = config.theta;
        let epsilon = config.epsilon;

        let accretion_template = GalaxyTemplate::from_config(config);
        let config = config.clone();

        let axis1 = Self::random_inclination_axis();
        let axis2 = Self::random_inclination_axis();
        let clockwise1 = fastrand::bool();
        let clockwise2 = fastrand::bool();

        let mut bodies1 = accretion_template.generate_inclined(Vec2::zero(), Vec2::zero(), axis1, clockwise1);
        let separation = accretion_template.outer_radius * config.galaxy_separation_factor;
        let offset = Vec2::new(separation, 0.0);
        let mut bodies2 = accretion_template.generate_inclined(offset, Vec2::zero(), axis2, clockwise2);

        let m1: f32 = bodies1.iter().map(|b| b.mass).sum();
        let m2: f32 = bodies2.iter().map(|b| b.mass).sum();
        let center1 = bodies1[0].pos;
        let center2 = bodies2[0].pos;
        let (v1, v2) = Self::galaxy_bulk_velocities(&config, m1, m2, center1, center2);
        for body in &mut bodies1 {
            body.vel += v1;
        }
        for body in &mut bodies2 {
            body.vel += v2;
        }

        let mut bodies = Vec::with_capacity(bodies1.len() + bodies2.len());
        bodies.extend(bodies1);
        let center2_idx = bodies.len();
        bodies.extend(bodies2);

        let octree = Octree::new(theta, epsilon);

        Self {
            dt,
            frame: 0,
            bodies,
            octree,
            accretion_template,
            config,
            center1_idx: 0,
            center2_idx,
            equatorial_plane_normal: Vec3::new(0.0, 0.0, 1.0),
            equatorial_plane_center: Vec3::zero(),
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

    pub(crate) fn record_body_state_change<I>(&mut self, _operation: &'static str, _ids: I)
    where I: IntoIterator<Item = u64> {
        // No-op shim for instrumentation audit. Real GPDM integration will
        // replace this with event buffering and HostEvent construction.
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
        }
        if renderer::SPAWN_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            self.spawn_accretion();
        }
        self.frame += 1;
    }

    pub fn render_update_interval(&self) -> usize {
        self.config.render_update_interval.max(1)
    }

    pub fn attract(&mut self) {
        let oct = Oct::new_containing(&self.bodies);
        let reserve_nodes = self.bodies.len() * 6;
        let reserve_parents = self.bodies.len();
        self.octree.reserve(reserve_nodes, reserve_parents);
        self.octree.clear(oct);

        for body in &self.bodies {
            self.octree.insert(body.pos, body.mass);
        }

        self.octree.propagate();

        let softening = self.config.epsilon * self.config.softening_scale_factor;
        let softening_sq = softening * softening;
        self.bodies.par_iter_mut().for_each(|body| {
            body.acc = self.octree.acc(body.pos, softening_sq);
        });

        self.apply_hydrodynamic_inflow();
    }

    fn apply_hydrodynamic_inflow(&mut self) {
        let outer_r = self.accretion_template.outer_radius.max(1.0);
        let inflow_strength = self.config.inflow_strength;
        let restore_strength = self.config.restore_strength;
        let plane_center = self.equatorial_plane_center;
        let plane_normal = self.equatorial_plane_normal;

        self.bodies.par_iter_mut().for_each(|body| {
            let offset = body.pos - plane_center;
            let dist_to_plane = offset.dot(plane_normal);
            let in_plane_pos = offset - plane_normal * dist_to_plane;
            let in_plane_dist = in_plane_pos.mag();

            let plane_attraction = -plane_normal * dist_to_plane * restore_strength / outer_r;
            let radial_inflow = if in_plane_dist > 0.0 {
                -in_plane_pos / in_plane_dist * inflow_strength * (1.0 - (in_plane_dist / outer_r).clamp(0.0, 1.0))
            } else {
                Vec3::zero()
            };

            body.acc += plane_attraction + radial_inflow;
        });
    }

    /// Raumzeitkrümmungs-Dilatation für Center-zu-Center Anziehung mit Spiralisierungs-Effekt
    /// Die Zentren nähern sich während ihrer Umkreisung an (orbitale Energie-Dissipation)
    fn apply_center_spacetime_dilation(&mut self) {
        if self.center1_idx >= self.bodies.len() || self.center2_idx >= self.bodies.len() {
            return;
        }

        let center1_pos = self.bodies[self.center1_idx].pos;
        let center1_vel = self.bodies[self.center1_idx].vel;
        let center1_mass = self.bodies[self.center1_idx].mass;
        let center2_pos = self.bodies[self.center2_idx].pos;
        let center2_vel = self.bodies[self.center2_idx].vel;
        let center2_mass = self.bodies[self.center2_idx].mass;

        let delta = center2_pos - center1_pos;
        let distance = delta.mag();

        if distance < 1e-6 {
            return; // Zu nah, ignoriere
        }

        let direction = delta.normalized();
        
        // 1. GRAVITATIONS-KRAFT (anziehendes Potential)
        // Stärke erhöht um Annäherung zu forcieren
        let force_magnitude = (center1_mass * center2_mass) / (distance * distance);
        let grav_force = direction * force_magnitude;

        // 2. DISSIPATIONS-KRAFT (Gravitationswellenstrahlung simulieren)
        // Entzieht dem System Energie basierend auf Relativgeschwindigkeit und Abstand
        let rel_vel = center2_vel - center1_vel;
        let radial_vel = rel_vel.dot(direction); // Nur die radiale Komponente
        
        // Dissipation ist stark, wenn Relative-Geschwindigkeit hoch ist und Abstand klein ist
        // Dies führt zu einer Spirale ins Zentrum
        let dissipation_factor = self.config.spacetime_dilation_factor * 0.001; // Bremsfaktor
        let dissipation_force = -direction * (radial_vel.abs() * force_magnitude * dissipation_factor);

        // 3. GESAMTKRAFT = Gravitation + Dissipation
        let total_force = grav_force + dissipation_force;

        // Wende Kraft auf beide Center an (in entgegengesetzter Richtung)
        if self.center1_idx < self.bodies.len() {
            self.bodies[self.center1_idx].acc += total_force / center1_mass.max(1.0);
        }
        if self.center2_idx < self.bodies.len() {
            self.bodies[self.center2_idx].acc -= total_force / center2_mass.max(1.0);
        }

        eprintln!("🌌 Spiral: dist={:.2}, grav={:.2}, dissipation={:.2}", 
                 distance, force_magnitude, dissipation_force.mag());
    }

    pub fn iterate(&mut self) {
        self.bodies.par_iter_mut().for_each(|body| {
            body.update(self.dt);
        });
    }

    pub fn collide(&mut self) {
        if self.bodies.len() < 2 {
            return;
        }

        let mut center_merge_pair: Option<(usize, usize)> = None;

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

        let mut size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
        let mut size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
        let mut size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;
        let max_grid_dim = 128_usize;
        let max_total_cells = max_grid_dim.saturating_mul(max_grid_dim).saturating_mul(max_grid_dim);

        if size_x > max_grid_dim || size_y > max_grid_dim || size_z > max_grid_dim {
            let scale = ((size_x.max(size_y).max(size_z)) as f32 / max_grid_dim as f32).ceil();
            cell_size *= scale;
            size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
            size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
            size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;
        }

        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells == 0 || total_cells > max_total_cells {
            return;
        }

        let mut cells = Vec::with_capacity(total_cells);
        cells.resize_with(total_cells, Vec::new);

        let cell_index = |pos: Vec3| {
            let ix = ((pos.x - min_x) / cell_size).floor() as isize;
            let iy = ((pos.y - min_y) / cell_size).floor() as isize;
            let iz = ((pos.z - min_z) / cell_size).floor() as isize;
            ((ix as usize) * size_y + iy as usize) * size_z + iz as usize
        };

        for (index, body) in self.bodies.iter().enumerate() {
            let idx = cell_index(body.pos);
            if idx < cells.len() {
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
                                        self.resolve(i, j, &mut center_merge_pair);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((i, j)) = center_merge_pair {
            self.merge_center_particles(i, j);
        }
    }

    /// Spawns accretion particles in the outer ring spawn zone.
    ///
    /// On each step rolls against `accretion_spawn_rate`. A successful roll places
    /// one new particle at a random position within the outer ring spawn zone
    /// (outer third of the galactic disk) and assigns it a circular orbital velocity
    /// derived from the local gravitational acceleration reported by the octree.
    /// Spawns are placed in absolute disk coordinates, avoiding the central bulge.
    fn spawn_accretion(&mut self) {
        let spawn_start_r = self
            .accretion_template
            .outer_ring_spawn_zone_inner_radius;
        let outer_r = self
            .accretion_template
            .outer_ring_spawn_zone_outer_radius;
        if spawn_start_r >= outer_r {
            return;
        }

        if fastrand::f32() > self.accretion_template.accretion_spawn_rate {
            return;
        }

        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;

        // Spawn in absolute disk coordinates, not relative to detected centers.
        // This ensures spawns occur in the outer third of the galactic disk.
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let r = spawn_start_r + fastrand::f32() * (outer_r - spawn_start_r);
        let spawn_pos = Vec3::new(
            cos * r,
            sin * r,
            (fastrand::f32() - 0.5) * outer_r * 0.03,
        );
        // Tangent direction for a clockwise orbit in the disk plane.
        let tangent = Vec3::new(sin, -cos, 0.0);

        let mass = mass_min + fastrand::f32() * (mass_max - mass_min);
        let radius = mass.cbrt();

        let softening = self.config.epsilon * self.config.softening_scale_factor;
        let softening_sq = softening * softening;
        let acc = self.octree.acc(spawn_pos, softening_sq);
        let orbital_speed = (acc.mag() * r).sqrt();
        let vel = tangent * orbital_speed;

        let angular_speed = (self.config.spawn_angular_speed_base 
            + fastrand::f32() * self.config.spawn_angular_speed_range 
            + orbital_speed * 0.02) * self.config.spin_speed_multiplier;
        let new_id = self.bodies.len() as u64;
        let new_body = Body::new(
            spawn_pos,
            vel,
            mass,
            radius,
            angular_speed,
            Vec3::new(0.0, 0.0, 1.0),
        );
        self.record_body_state_change("spawn_accretion", [new_id]);
        self.bodies.push(new_body);
    }

    fn resolve(&mut self, i: usize, j: usize, center_merge_pair: &mut Option<(usize, usize)>) {
        if (i == self.center1_idx && j == self.center2_idx) || (i == self.center2_idx && j == self.center1_idx) {
            *center_merge_pair = Some((i, j));
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

        let t = (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        self.bodies[i].pos -= v1 * t;
        self.bodies[j].pos -= v2 * t;

        let p1 = self.bodies[i].pos;
        let p2 = self.bodies[j].pos;
        let d = p2 - p1;
        let d_dot_v = d.dot(v);
        let d_sq = d.mag_sq();

        let tmp = d * (1.5 * d_dot_v / d_sq);
        let v1 = v1 + tmp * weight1;
        let v2 = v2 - tmp * weight2;

        self.bodies[i].vel = v1;
        self.bodies[j].vel = v2;
        self.bodies[i].pos += v1 * t;
        self.bodies[j].pos += v2 * t;
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
        let merged_angular_speed = (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        let replace_id = first as u64;
        let remove_id = second as u64;
        self.record_body_state_change("merge_center_replace", [replace_id]);
        self.bodies[first] = Body::new(merged_pos, merged_vel, total_mass, merged_radius, merged_angular_speed, axis);
        self.record_body_state_change("merge_center_remove", [remove_id]);
        self.bodies.remove(second);

        self.center1_idx = first;
        self.center2_idx = usize::MAX;
        self.equatorial_plane_normal = axis;
        self.equatorial_plane_center = merged_pos;

        eprintln!(
            "🔗 Center merge: mass={} pos={:?} axis={:?} inflow-plane-normal={:?}",
            total_mass,
            merged_pos,
            axis,
            self.equatorial_plane_normal
        );
    }
}

