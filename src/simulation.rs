use crate::{
    body::Body,
    config::InformationsConfig,
    galaxy_templates::GalaxyTemplate,
    quadtree::{Oct, Octree},
    renderer,
};

use rayon::prelude::*;
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
    /// Equatorial plane normal for the first galaxy.
    equatorial_plane_normal_a: Vec3,
    /// Center of the first galaxy's equatorial plane.
    equatorial_plane_center_a: Vec3,
    /// Equatorial plane normal for the second galaxy.
    equatorial_plane_normal_b: Vec3,
    /// Center of the second galaxy's equatorial plane.
    equatorial_plane_center_b: Vec3,
}

struct NeighborGrid {
    cells: Vec<usize>,
    cell_offsets: Vec<usize>,
    origin: Vec3,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    cell_size: f32,
}

impl NeighborGrid {
    fn with_bodies(bodies: &[Body], cell_size: f32) -> Option<Self> {
        if !(cell_size > 0.0) || bodies.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;
        for body in bodies {
            min_x = min_x.min(body.pos.x);
            min_y = min_y.min(body.pos.y);
            min_z = min_z.min(body.pos.z);
            max_x = max_x.max(body.pos.x);
            max_y = max_y.max(body.pos.y);
            max_z = max_z.max(body.pos.z);
        }

        if !min_x.is_finite() || !min_y.is_finite() || !min_z.is_finite() {
            return None;
        }

        let origin = Vec3::new(min_x, min_y, min_z);
        let size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
        let size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
        let size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;
        if size_x == 0 || size_y == 0 || size_z == 0 {
            return None;
        }

        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells == 0 {
            return None;
        }

        let mut counts = vec![0_usize; total_cells];
        for body in bodies {
            let ix = ((body.pos.x - origin.x) / cell_size).floor();
            let iy = ((body.pos.y - origin.y) / cell_size).floor();
            let iz = ((body.pos.z - origin.z) / cell_size).floor();
            if ix.is_sign_negative() || iy.is_sign_negative() || iz.is_sign_negative() {
                continue;
            }
            let ix = ix as usize;
            let iy = iy as usize;
            let iz = iz as usize;
            if ix >= size_x || iy >= size_y || iz >= size_z {
                continue;
            }
            counts[(ix * size_y + iy) * size_z + iz] += 1;
        }

        let mut cell_offsets = Vec::with_capacity(total_cells + 1);
        cell_offsets.push(0);
        for count in &counts {
            cell_offsets.push(cell_offsets.last().copied().unwrap_or(0) + *count);
        }

        let mut cells = Vec::with_capacity(cell_offsets.last().copied().unwrap_or(0));
        cells.resize(cell_offsets.last().copied().unwrap_or(0), 0);

        let mut cursor = cell_offsets[..total_cells].to_vec();
        for (index, body) in bodies.iter().enumerate() {
            let ix = ((body.pos.x - origin.x) / cell_size).floor();
            let iy = ((body.pos.y - origin.y) / cell_size).floor();
            let iz = ((body.pos.z - origin.z) / cell_size).floor();
            if ix.is_sign_negative() || iy.is_sign_negative() || iz.is_sign_negative() {
                continue;
            }
            let ix = ix as usize;
            let iy = iy as usize;
            let iz = iz as usize;
            if ix >= size_x || iy >= size_y || iz >= size_z {
                continue;
            }
            let cell_idx = (ix * size_y + iy) * size_z + iz;
            let write_pos = cursor[cell_idx];
            cells[write_pos] = index;
            cursor[cell_idx] += 1;
        }

        Some(Self {
            cells,
            cell_offsets,
            origin,
            size_x,
            size_y,
            size_z,
            cell_size,
        })
    }

    #[inline]
    fn cell_index(&self, pos: Vec3) -> Option<(usize, usize, usize)> {
        let ix = ((pos.x - self.origin.x) / self.cell_size).floor();
        let iy = ((pos.y - self.origin.y) / self.cell_size).floor();
        let iz = ((pos.z - self.origin.z) / self.cell_size).floor();
        if ix.is_sign_negative() || iy.is_sign_negative() || iz.is_sign_negative() {
            return None;
        }
        let ix = ix as usize;
        let iy = iy as usize;
        let iz = iz as usize;
        if ix >= self.size_x || iy >= self.size_y || iz >= self.size_z {
            return None;
        }
        Some((ix, iy, iz))
    }

    #[inline]
    fn cell_offset(&self, ix: usize, iy: usize, iz: usize) -> usize {
        (ix * self.size_y + iy) * self.size_z + iz
    }

    #[inline]
    fn cell_range(&self, ix: usize, iy: usize, iz: usize) -> &[usize] {
        let idx = self.cell_offset(ix, iy, iz);
        let start = self.cell_offsets[idx];
        let end = self.cell_offsets[idx + 1];
        &self.cells[start..end]
    }
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
    /// Temporary buffers reused between simulation steps to avoid allocations.
    density_buffer: Vec<f32>,
    smoothing_lengths: Vec<f32>,
    sph_acc_buffer: Vec<Vec3>,
}

impl Simulation {
    fn sanitize_vec(v: Vec3) -> Vec3 {
        Vec3::new(
            if v.x.is_finite() { v.x } else { 0.0 },
            if v.y.is_finite() { v.y } else { 0.0 },
            if v.z.is_finite() { v.z } else { 0.0 },
        )
    }

    pub fn render_update_interval(&self) -> usize {
        self.config.render_update_interval.max(1)
    }

    pub fn effective_attract_interval(&self) -> usize {
        if self.config.performance_mode {
            self.config.attract_interval.max(4)
        } else {
            self.config.attract_interval
        }
    }

    pub fn effective_collision_interval(&self) -> usize {
        if self.config.performance_mode {
            self.config.collision_interval.max(8)
        } else {
            self.config.collision_interval
        }
    }

    fn ensure_scratch_buffers(&mut self) {
        let n = self.bodies.len();
        self.density_buffer.resize(n, 0.0);
        self.smoothing_lengths.resize(n, self.config.epsilon);
        self.sph_acc_buffer.resize_with(n, Vec3::zero);
    }

    pub fn new(config: &InformationsConfig) -> Self {
        let dt = config.dt;
        let theta = config.theta;
        let epsilon = config.epsilon;

        let accretion_template = GalaxyTemplate::from_config(config);
        let mut config = config.clone();
        if config.performance_mode {
            config.sph_enabled = false;
            config.adaptive_softening_enabled = false;
            config.attract_interval = config.attract_interval.max(4);
            config.collision_interval = config.collision_interval.max(8);
            config.render_update_interval = config.render_update_interval.max(4);
        }

        if config.ultra_performance_mode {
            config.performance_mode = true;
            config.sph_enabled = false;
            config.adaptive_softening_enabled = false;
            config.accretion_spawn_rate = 0.0;
            config.inflow_strength = 0.0;
            config.restore_strength = 0.0;
            config.attract_interval = config.attract_interval.max(12);
            config.collision_interval = config.collision_interval.max(16);
            config.render_update_interval = config.render_update_interval.max(8);
            config.theta = config.theta.max(1.75);
            config.spacetime_dilation_factor = 0.0;
        }
 
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

            let mut bodies_a =
                accretion_template.generate_inclined(Vec2::zero(), Vec2::zero(), axis1, clockwise1);
            let mut bodies_b =
                accretion_template.generate_inclined(Vec2::zero(), Vec2::zero(), axis2, clockwise2);

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
                equatorial_plane_normal_a: axis1,
                equatorial_plane_center_a: center_a3,
                equatorial_plane_normal_b: axis2,
                equatorial_plane_center_b: center_b3,
            });
        }

        let octree = Octree::new(theta, epsilon);

        let mut simulation = Self {
            dt,
            frame: 0,
            bodies,
            octree,
            accretion_template,
            config,
            pairs,
            density_buffer: Vec::new(),
            smoothing_lengths: Vec::new(),
            sph_acc_buffer: Vec::new(),
        };
        simulation.ensure_scratch_buffers();
        simulation
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

    fn sph_kernel_weight(r: f32, h: f32) -> f32 {
        if r <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        let q = r / h;
        let sigma = 1.0 / (std::f32::consts::PI * h * h * h);
        if q < 1.0 {
            sigma * (1.0 - 1.5 * q * q + 0.75 * q * q * q)
        } else if q < 2.0 {
            let t = 2.0 - q;
            sigma * 0.25 * t * t * t
        } else {
            0.0
        }
    }

    fn sph_kernel_grad(r: f32, h: f32) -> f32 {
        if r <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        let q = r / h;
        let sigma = 1.0 / (std::f32::consts::PI * h * h * h);
        let inv_h = 1.0 / h;
        if q < 1.0 {
            sigma * (-3.0 * q + 2.25 * q * q) * inv_h
        } else if q < 2.0 {
            let t = 2.0 - q;
            sigma * (-0.75 * t * t) * inv_h
        } else {
            0.0
        }
    }

    fn build_neighbor_grid(&self, cell_size: f32) -> Option<NeighborGrid> {
        NeighborGrid::with_bodies(&self.bodies, cell_size)
    }

    fn compute_local_densities_with_grid(
        bodies: &[Body],
        grid: &NeighborGrid,
        base_smoothing: f32,
        densities: &mut [f32],
    ) -> bool {
        if densities.len() != bodies.len() {
            return false;
        }

        let cell_search = ((2.0 * base_smoothing) / grid.cell_size).ceil().max(1.0) as isize;
        densities
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, density)| {
                let body = &bodies[i];
                let (ix, iy, iz) = match grid.cell_index(body.pos) {
                    Some(coords) => coords,
                    None => {
                        *density = 1e-8;
                        return;
                    }
                };
                let mut value = 0.0_f32;
                for dx in -cell_search..=cell_search {
                    let nx = ix as isize + dx;
                    if nx < 0 || nx as usize >= grid.size_x {
                        continue;
                    }
                    let nx = nx as usize;
                    for dy in -cell_search..=cell_search {
                        let ny = iy as isize + dy;
                        if ny < 0 || ny as usize >= grid.size_y {
                            continue;
                        }
                        let ny = ny as usize;
                        for dz in -cell_search..=cell_search {
                            let nz = iz as isize + dz;
                            if nz < 0 || nz as usize >= grid.size_z {
                                continue;
                            }
                            let nz = nz as usize;
                            for &j in grid.cell_range(nx, ny, nz) {
                                let other = &bodies[j];
                                let r = (body.pos - other.pos).mag();
                                value += other.mass * Self::sph_kernel_weight(r, base_smoothing);
                            }
                        }
                    }
                }
                *density = value.max(1e-8);
            });

        true
    }

    fn compute_sph_accelerations_with_grid(
        bodies: &[Body],
        config: &InformationsConfig,
        grid: &NeighborGrid,
        densities: &[f32],
        smoothing_lengths: &[f32],
        sph_acc: &mut [Vec3],
    ) -> bool {
        if densities.len() != bodies.len()
            || smoothing_lengths.len() != bodies.len()
            || sph_acc.len() != bodies.len()
        {
            return false;
        }

        let pressure: Vec<f32> = densities
            .iter()
            .map(|&rho| config.hydro_pressure_strength * rho)
            .collect();

        sph_acc
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, acc)| {
                let body = &bodies[i];
                let (ix, iy, iz) = match grid.cell_index(body.pos) {
                    Some(coords) => coords,
                    None => {
                        *acc = Vec3::zero();
                        return;
                    }
                };
                let h_i = smoothing_lengths[i].max(1e-6);
                let rho_i = densities[i].max(1e-6);
                let p_i = pressure[i];
                let cell_search = ((config.hydro_kernel_factor * h_i) / grid.cell_size)
                    .ceil()
                    .min(4.0)
                    .max(1.0) as isize;
                let mut local_acc = Vec3::zero();

                for dx in -cell_search..=cell_search {
                    let nx = ix as isize + dx;
                    if nx < 0 || nx as usize >= grid.size_x {
                        continue;
                    }
                    let nx = nx as usize;
                    for dy in -cell_search..=cell_search {
                        let ny = iy as isize + dy;
                        if ny < 0 || ny as usize >= grid.size_y {
                            continue;
                        }
                        let ny = ny as usize;
                        for dz in -cell_search..=cell_search {
                            let nz = iz as isize + dz;
                            if nz < 0 || nz as usize >= grid.size_z {
                                continue;
                            }
                            let nz = nz as usize;
                            for &j in grid.cell_range(nx, ny, nz) {
                                if i == j {
                                    continue;
                                }
                                let other = &bodies[j];
                                let r_vec = body.pos - other.pos;
                                let r = r_vec.mag();
                                if r <= 0.0 {
                                    continue;
                                }
                                let h_j = smoothing_lengths[j].max(1e-6);
                                let h_ij = 0.5 * (h_i + h_j);
                                if r > config.hydro_kernel_factor * h_ij {
                                    continue;
                                }
                                let grad_w = Self::sph_kernel_grad(r, h_ij);
                                if grad_w == 0.0 {
                                    continue;
                                }
                                let rho_j = densities[j].max(1e-6);
                                let p_j = pressure[j];
                                let pressure_term =
                                    (p_i / (rho_i * rho_i)) + (p_j / (rho_j * rho_j));
                                local_acc += r_vec * (-other.mass * pressure_term * grad_w / r);
                            }
                        }
                    }
                }
                *acc = local_acc;
            });

        true
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
        if self.frame % self.effective_collision_interval() == 0 {
            if renderer::COLLISIONS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                self.collide();
            }
        }
        if self.frame % self.effective_attract_interval() == 0 {
            self.attract();
            // Raumzeitkrümmungs-Dilatation für Center-Partikel überschreibt Teil der Gravitation
            if !self.config.ultra_performance_mode {
                self.apply_center_spacetime_dilation();
            }
        }
        if renderer::SPAWN_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            self.spawn_accretion();
        }
        self.frame += 1;
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
 
        let base_smoothing = self.config.epsilon * self.config.softening_scale_factor;
        self.ensure_scratch_buffers();
 
        let grid = if self.config.adaptive_softening_enabled || self.config.sph_enabled {
            self.build_neighbor_grid(base_smoothing * 2.0)
        } else {
            None
        };
 
        let densities = if let Some(ref grid) = grid {
            if !Self::compute_local_densities_with_grid(
                &self.bodies,
                grid,
                base_smoothing,
                &mut self.density_buffer,
            ) {
                self.density_buffer.fill(1.0);
                &self.density_buffer
            } else {
                &self.density_buffer
            }
        } else {
            self.density_buffer.fill(1.0);
            &self.density_buffer
        };
 
        let average_density = densities.iter().copied().sum::<f32>() / self.bodies.len() as f32;
        let average_density = average_density.max(1e-6);
 
        self.smoothing_lengths
            .iter_mut()
            .enumerate()
            .for_each(|(i, h)| {
                let rho = densities[i];
                *h = if self.config.adaptive_softening_enabled {
                    let scale = (average_density / rho).powf(1.0 / 3.0);
                    self.config.epsilon
                        * scale.clamp(
                            self.config.softening_min_factor,
                            self.config.softening_max_factor,
                        )
                } else {
                    self.config.epsilon
                };
            });
 
        if let Some(grid) = grid {
            if self.config.sph_enabled {
                Self::compute_sph_accelerations_with_grid(
                    &self.bodies,
                    &self.config,
                    &grid,
                    densities,
                    &self.smoothing_lengths,
                    &mut self.sph_acc_buffer,
                );
            } else {
                self.sph_acc_buffer.fill(Vec3::zero());
            }
        } else {
            self.sph_acc_buffer.fill(Vec3::zero());
        }

        self.bodies
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, body)| {
                let softening_sq = self.smoothing_lengths[i] * self.smoothing_lengths[i];
                body.acc = self.octree.acc(body.pos, softening_sq);
                body.acc += self.sph_acc_buffer[i];
                body.acc = Simulation::sanitize_vec(body.acc);
            });
 
        if !self.config.ultra_performance_mode {
            self.apply_hydrodynamic_inflow();
        }
    }

    fn pair_plane_centers_and_normals(
        bodies: &[Body],
        pair: &GalaxyPairState,
    ) -> (Vec3, Vec3, Vec3, Vec3) {
        let center_a_pos = bodies
            .get(pair.center1_idx)
            .map(|body| body.pos)
            .unwrap_or(pair.equatorial_plane_center_a);
        let axis_a = bodies
            .get(pair.center1_idx)
            .map(|body| body.rotation_axis)
            .unwrap_or(pair.equatorial_plane_normal_a);

        let (center_b_pos, axis_b) = if pair.center2_idx != usize::MAX {
            (
                bodies
                    .get(pair.center2_idx)
                    .map(|body| body.pos)
                    .unwrap_or(pair.equatorial_plane_center_b),
                bodies
                    .get(pair.center2_idx)
                    .map(|body| body.rotation_axis)
                    .unwrap_or(pair.equatorial_plane_normal_b),
            )
        } else {
            (center_a_pos, axis_a)
        };

        (center_a_pos, axis_a, center_b_pos, axis_b)
    }

    fn apply_hydrodynamic_inflow(&mut self) {
        let outer_r = self.accretion_template.outer_radius.max(1.0);
        let inflow_strength = self.config.inflow_strength;
        let restore_strength = self.config.restore_strength;

        for pair in &mut self.pairs {
            let start = pair.range_start.min(self.bodies.len());
            let end = pair.range_end.min(self.bodies.len());
            if start >= end {
                continue;
            }

            let mid = pair.center2_idx.min(self.bodies.len());
            let (range_a, range_b) = if mid > start && mid < end {
                (start..mid, mid..end)
            } else {
                (start..end, end..end)
            };

            let (center_a, axis_a, center_b, axis_b) =
                Simulation::pair_plane_centers_and_normals(&self.bodies, pair);

            let apply_plane = |body: &mut Body, center: Vec3, normal: Vec3| {
                let normal = if normal == Vec3::zero() {
                    Vec3::new(0.0, 0.0, 1.0)
                } else {
                    normal.normalized()
                };
                let offset = body.pos - center;
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
                let normal_velocity = body.vel.dot(normal);
                let plane_velocity_damping = -normal * normal_velocity * 0.12;

                body.acc += plane_attraction + radial_inflow + plane_velocity_damping;
            };

            self.bodies[range_a].par_iter_mut().for_each(|body| {
                apply_plane(body, center_a, axis_a);
            });
            self.bodies[range_b].par_iter_mut().for_each(|body| {
                apply_plane(body, center_b, axis_b);
            });

            // Keep stored plane values in sync with the dynamic center body.
            pair.equatorial_plane_center_a = center_a;
            pair.equatorial_plane_normal_a = axis_a;
            pair.equatorial_plane_center_b = center_b;
            pair.equatorial_plane_normal_b = axis_b;
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

        let mut counts = vec![0_usize; total_cells];
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

        for body in &self.bodies {
            if let Some(idx) = cell_index(body.pos) {
                counts[idx] += 1;
            }
        }

        let mut cell_offsets = Vec::with_capacity(total_cells + 1);
        cell_offsets.push(0);
        for count in &counts {
            cell_offsets.push(cell_offsets.last().copied().unwrap_or(0) + *count);
        }
        let mut cells = vec![0_usize; *cell_offsets.last().unwrap_or(&0)];
        let mut cursor = cell_offsets[..total_cells].to_vec();

        for (index, body) in self.bodies.iter().enumerate() {
            if let Some(cell_idx) = cell_index(body.pos) {
                let write_pos = cursor[cell_idx];
                cells[write_pos] = index;
                cursor[cell_idx] += 1;
            }
        }

        for x in 0..size_x {
            for y in 0..size_y {
                for z in 0..size_z {
                    let cell_idx = (x * size_y + y) * size_z + z;
                    let start = cell_offsets[cell_idx];
                    let end = cell_offsets[cell_idx + 1];
                    if start == end {
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
                                let n_start = cell_offsets[neighbor_idx];
                                let n_end = cell_offsets[neighbor_idx + 1];
                                if n_start == n_end {
                                    continue;
                                }

                                for i_idx in start..end {
                                    let i = cells[i_idx];
                                    for j_idx in n_start..n_end {
                                        let j = cells[j_idx];
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

        let acc = self.octree.acc(spawn_pos, self.octree.e_sq);
        let orbital_speed = (acc.mag() * r).sqrt();
        let vel = tangent * orbital_speed;

        let angular_speed = (self.config.spawn_angular_speed_base
            + fastrand::f32() * self.config.spawn_angular_speed_range
            + orbital_speed * 0.02)
            * self.config.spin_speed_multiplier;
        self.bodies.push(Body::new(
            spawn_pos,
            vel,
            mass,
            radius,
            angular_speed,
            Vec3::new(0.0, 0.0, 1.0),
        ));
    }

    fn resolve(&mut self, i: usize, j: usize, center_merge_pair: &mut Option<(usize, usize)>) {
        let is_center_pair = self.pairs.iter().any(|pair| {
            pair.center2_idx != usize::MAX
                && ((i == pair.center1_idx && j == pair.center2_idx)
                    || (i == pair.center2_idx && j == pair.center1_idx))
        });
        if is_center_pair {
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

        let rel_pos1 = b1.pos - merged_pos;
        let rel_pos2 = b2.pos - merged_pos;
        let orbital_ang_mom = rel_pos1.cross(b1.vel) * b1.mass + rel_pos2.cross(b2.vel) * b2.mass;
        let spin_ang_mom = b1.rotation_axis * b1.angular_speed.abs() * b1.mass
            + b2.rotation_axis * b2.angular_speed.abs() * b2.mass;
        let total_ang_mom = orbital_ang_mom + spin_ang_mom;
        let axis = if total_ang_mom.mag_sq() > 1e-8 {
            total_ang_mom.normalized()
        } else {
            let fallback = b1.rotation_axis * b1.mass + b2.rotation_axis * b2.mass;
            if fallback.mag_sq() > 1e-8 {
                fallback.normalized()
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            }
        };

        let merged_radius = (b1.base_radius.powi(3) + b2.base_radius.powi(3)).cbrt();
        let merged_angular_speed =
            (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        self.bodies[first] = Body::new(
            merged_pos,
            merged_vel,
            total_mass,
            merged_radius,
            merged_angular_speed,
            axis,
        );
        self.bodies.remove(second);

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
                pair.equatorial_plane_normal_a = axis;
                pair.equatorial_plane_center_a = merged_pos;
                pair.equatorial_plane_normal_b = axis;
                pair.equatorial_plane_center_b = merged_pos;
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
