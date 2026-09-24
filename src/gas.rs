use crate::{body::Body, config::InformationsConfig};
use ultraviolet::Vec3;

pub struct GasNeighborGrid {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    cell_size: f32,
    cell_offsets: Vec<usize>,
    cell_indices: Vec<usize>,
}

impl GasNeighborGrid {
    pub fn new() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            size_x: 0,
            size_y: 0,
            size_z: 0,
            cell_size: 1.0,
            cell_offsets: Vec::new(),
            cell_indices: Vec::new(),
        }
    }

    pub fn build(&mut self, bodies: &[Body], gas_indices: &[usize], smoothing: f32) {
        if gas_indices.is_empty() || !smoothing.is_finite() || smoothing <= 0.0 {
            self.size_x = 0;
            self.size_y = 0;
            self.size_z = 0;
            self.cell_offsets.clear();
            self.cell_indices.clear();
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;

        for &index in gas_indices {
            let pos = bodies[index].pos;
            if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
                continue;
            }
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            min_z = min_z.min(pos.z);
            max_x = max_x.max(pos.x);
            max_y = max_y.max(pos.y);
            max_z = max_z.max(pos.z);
        }

        if min_x == f32::MAX || !min_x.is_finite() {
            self.size_x = 0;
            self.size_y = 0;
            self.size_z = 0;
            self.cell_offsets.clear();
            self.cell_indices.clear();
            return;
        }

        let mut cell_size = smoothing.max(0.01);
        min_x -= cell_size;
        min_y -= cell_size;
        min_z -= cell_size;
        max_x += cell_size;
        max_y += cell_size;
        max_z += cell_size;

        let mut size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
        let mut size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
        let mut size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;

        const MAX_GRID_DIM: usize = 128;
        const MAX_TOTAL_CELLS: usize = MAX_GRID_DIM * MAX_GRID_DIM * MAX_GRID_DIM;
        while size_x > MAX_GRID_DIM || size_y > MAX_GRID_DIM || size_z > MAX_GRID_DIM {
            let scale = ((size_x.max(size_y).max(size_z)) as f32 / MAX_GRID_DIM as f32).ceil();
            cell_size *= scale;
            size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
            size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
            size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;
            if size_x == 0 || size_y == 0 || size_z == 0 {
                self.size_x = 0;
                self.size_y = 0;
                self.size_z = 0;
                self.cell_offsets.clear();
                self.cell_indices.clear();
                return;
            }
        }

        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells == 0 || total_cells > MAX_TOTAL_CELLS {
            self.size_x = 0;
            self.size_y = 0;
            self.size_z = 0;
            self.cell_offsets.clear();
            self.cell_indices.clear();
            return;
        }

        let mut cell_counts = vec![0usize; total_cells];
        let cell_index = |pos: Vec3| -> Option<usize> {
            if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
                return None;
            }
            let ix = ((pos.x - min_x) / cell_size).floor() as isize;
            let iy = ((pos.y - min_y) / cell_size).floor() as isize;
            let iz = ((pos.z - min_z) / cell_size).floor() as isize;
            if ix < 0 || iy < 0 || iz < 0 {
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

        for &index in gas_indices {
            if let Some(cell_idx) = cell_index(bodies[index].pos) {
                cell_counts[cell_idx] += 1;
            }
        }

        let mut cell_offsets = vec![0usize; total_cells + 1];
        for i in 0..total_cells {
            cell_offsets[i + 1] = cell_offsets[i] + cell_counts[i];
        }

        let total_indices = cell_offsets[total_cells];
        let mut cell_indices = vec![usize::MAX; total_indices];
        let mut write_positions = cell_offsets[..total_cells].to_vec();

        for &index in gas_indices {
            if let Some(cell_idx) = cell_index(bodies[index].pos) {
                let write_pos = write_positions[cell_idx];
                if write_pos < total_indices {
                    cell_indices[write_pos] = index;
                    write_positions[cell_idx] += 1;
                }
            }
        }

        self.min_x = min_x;
        self.min_y = min_y;
        self.min_z = min_z;
        self.size_x = size_x;
        self.size_y = size_y;
        self.size_z = size_z;
        self.cell_size = cell_size;
        self.cell_offsets = cell_offsets;
        self.cell_indices = cell_indices;
    }

    fn get_cell_coords(&self, cell_idx: usize) -> Option<(usize, usize, usize)> {
        if self.size_x == 0 || self.size_y == 0 || self.size_z == 0 {
            return None;
        }
        let yz_stride = self.size_y * self.size_z;
        let x = cell_idx / yz_stride;
        let yz = cell_idx % yz_stride;
        let y = yz / self.size_z;
        let z = yz % self.size_z;
        Some((x, y, z))
    }

    fn cell_index(&self, pos: Vec3) -> Option<usize> {
        if self.size_x == 0 || self.size_y == 0 || self.size_z == 0 {
            return None;
        }
        if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
            return None;
        }
        let ix = ((pos.x - self.min_x) / self.cell_size).floor();
        let iy = ((pos.y - self.min_y) / self.cell_size).floor();
        let iz = ((pos.z - self.min_z) / self.cell_size).floor();
        if ix.is_sign_negative() || iy.is_sign_negative() || iz.is_sign_negative() {
            return None;
        }
        let ix = ix as usize;
        let iy = iy as usize;
        let iz = iz as usize;
        if ix >= self.size_x || iy >= self.size_y || iz >= self.size_z {
            return None;
        }
        Some((ix * self.size_y + iy) * self.size_z + iz)
    }

    pub fn valid(&self) -> bool {
        self.size_x > 0 && self.size_y > 0 && self.size_z > 0 && !self.cell_indices.is_empty()
    }

    pub fn gather_neighbors(
        &self,
        bodies: &[Body],
        body_index: usize,
        smoothing: f32,
        out: &mut Vec<usize>,
    ) {
        out.clear();
        if !self.valid() || smoothing <= 0.0 {
            return;
        }
        let body = &bodies[body_index];
        if let Some(cell_idx) = self.cell_index(body.pos) {
            let (x, y, z) = match self.get_cell_coords(cell_idx) {
                Some(coords) => coords,
                None => return,
            };
            let max_x = self.size_x;
            let max_y = self.size_y;
            let max_z = self.size_z;
            let smoothing_sq = smoothing * smoothing;

            for dx in -1isize..=1 {
                for dy in -1isize..=1 {
                    for dz in -1isize..=1 {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        let nz = z as isize + dz;
                        if nx < 0 || ny < 0 || nz < 0 {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        let nz = nz as usize;
                        if nx >= max_x || ny >= max_y || nz >= max_z {
                            continue;
                        }
                        let neighbor_cell = (nx * max_y + ny) * max_z + nz;
                        let start = self.cell_offsets[neighbor_cell];
                        let end = self.cell_offsets[neighbor_cell + 1];
                        for &other_index in &self.cell_indices[start..end] {
                            if other_index == usize::MAX || other_index == body_index {
                                continue;
                            }
                            let offset = bodies[other_index].pos - body.pos;
                            if offset.mag_sq() <= smoothing_sq {
                                out.push(other_index);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct GasSystem {
    pub neighbor_grid: GasNeighborGrid,
    pub densities: Vec<f32>,
    pub pressures: Vec<f32>,
    pub neighbor_offsets: Vec<usize>,
    pub neighbor_indices: Vec<usize>,
    pub temp_neighbors: Vec<usize>,
}

impl GasSystem {
    pub fn new() -> Self {
        Self {
            neighbor_grid: GasNeighborGrid::new(),
            densities: Vec::new(),
            pressures: Vec::new(),
            neighbor_offsets: Vec::new(),
            neighbor_indices: Vec::new(),
            temp_neighbors: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        bodies: &mut [Body],
        gas_indices: &[usize],
        config: &InformationsConfig,
        dt: f32,
    ) {
        if gas_indices.len() < 2 || !config.gas_enabled {
            return;
        }

        let gas_target_neighbors = config.gas_target_neighbors.max(1);
        let effective_neighbors = if config.performance_high_count_mode
            && bodies.len() > config.performance_body_count_threshold
        {
            gas_target_neighbors.saturating_sub(4).max(4)
        } else {
            gas_target_neighbors
        };

        let ratio = gas_indices.len() as f32 / effective_neighbors as f32;
        let smoothing = (config.gas_h_min * ratio.sqrt()).clamp(config.gas_h_min, config.gas_h_max);
        self.neighbor_grid.build(bodies, gas_indices, smoothing);

        let body_count = bodies.len();
        if self.densities.len() < body_count {
            self.densities.resize(body_count, 0.0);
            self.pressures.resize(body_count, 0.0);
        }

        self.neighbor_offsets.clear();
        self.neighbor_indices.clear();
        self.neighbor_offsets
            .reserve(gas_indices.len().saturating_add(1));
        self.neighbor_offsets.push(0);

        for &index in gas_indices {
            self.neighbor_grid
                .gather_neighbors(bodies, index, smoothing, &mut self.temp_neighbors);
            self.neighbor_indices
                .extend_from_slice(&self.temp_neighbors);
            self.neighbor_offsets.push(self.neighbor_indices.len());
        }

        let smoothing_sq = smoothing * smoothing;
        let inv_poly6 = 315.0 / (64.0 * std::f32::consts::PI * smoothing.powi(9).max(1e-6));
        let inv_spiky = 45.0 / (std::f32::consts::PI * smoothing.powi(6).max(1e-6));
        let self_kernel = inv_poly6 * smoothing.powi(6);

        for (gas_slot, &index) in gas_indices.iter().enumerate() {
            let body = &bodies[index];
            let mut density = body.mass * self_kernel;
            let start = self.neighbor_offsets[gas_slot];
            let end = self.neighbor_offsets[gas_slot + 1];

            for &neighbor_index in &self.neighbor_indices[start..end] {
                let offset = body.pos - bodies[neighbor_index].pos;
                let r2 = offset.mag_sq();
                if r2 <= smoothing_sq {
                    let term = (smoothing_sq - r2).max(0.0);
                    density += bodies[neighbor_index].mass * inv_poly6 * term * term * term;
                }
            }

            density = density.max(1e-6);
            self.densities[index] = density;
            self.pressures[index] =
                config.gas_pressure_coefficient * (density - config.gas_rest_density).max(0.0);
        }

        for (gas_slot, &index) in gas_indices.iter().enumerate() {
            let density = self.densities[index];
            if density <= 0.0 {
                continue;
            }
            let mut force = Vec3::zero();
            let body = &bodies[index];
            let start = self.neighbor_offsets[gas_slot];
            let end = self.neighbor_offsets[gas_slot + 1];

            for &neighbor_index in &self.neighbor_indices[start..end] {
                let neighbor = &bodies[neighbor_index];
                let offset = body.pos - neighbor.pos;
                let r2 = offset.mag_sq();
                if r2 <= 0.0 || r2 > smoothing_sq {
                    continue;
                }
                let r = r2.sqrt();
                let direction = offset / r;
                let gradient = inv_spiky * (smoothing - r) * (smoothing - r);
                let pressure_term = (self.pressures[index] + self.pressures[neighbor_index])
                    / (2.0 * self.densities[neighbor_index].max(1e-6));
                force += direction * (-neighbor.mass * pressure_term * gradient);

                let density_diff = self.densities[neighbor_index] - density;
                if density_diff > 0.0 {
                    force += direction
                        * (density_diff
                            * config.gas_condensation_strength
                            * (1.0 - (r / smoothing).clamp(0.0, 1.0)));
                }
            }

            let acc_delta = force / density;
            bodies[index].acc += acc_delta;
        }

        for &index in gas_indices {
            let body = &mut bodies[index];
            let density = self.densities[index];
            body.gas_density = density;
            body.gas_pressure = self.pressures[index];
            body.gas_temperature =
                (body.gas_temperature - config.gas_cooling_rate * density * dt).max(0.05);

            if body.rotation_axis.mag_sq() > 1e-8 {
                let normal = body.rotation_axis.normalized();
                let perpendicular_velocity = normal * body.vel.dot(normal);
                body.vel -= perpendicular_velocity * config.gas_relaxation_strength;
            }

            body.gas_smoothing_radius = smoothing;
        }
    }

    pub fn collect_merge_candidates(
        &self,
        bodies: &[Body],
        gas_indices: &[usize],
        config: &InformationsConfig,
    ) -> Vec<(usize, usize)> {
        let mut merge_pairs = Vec::new();
        for (slot, &index) in gas_indices.iter().enumerate() {
            let start = self.neighbor_offsets[slot];
            let end = self.neighbor_offsets[slot + 1];
            let radius_factor = config.gas_merge_distance_factor.max(0.1);
            let max_speed = config.gas_merge_velocity_max.max(0.0);
            for &neighbor_index in &self.neighbor_indices[start..end] {
                if index >= neighbor_index {
                    continue;
                }
                if !bodies[index].is_gas() || !bodies[neighbor_index].is_gas() {
                    continue;
                }
                let radius_sum = (bodies[index].effective_radius()
                    + bodies[neighbor_index].effective_radius())
                    * radius_factor;
                let threshold_sq = radius_sum * radius_sum;
                let offset = bodies[index].pos - bodies[neighbor_index].pos;
                if offset.mag_sq() > threshold_sq {
                    continue;
                }
                let rel_speed = (bodies[index].vel - bodies[neighbor_index].vel).mag();
                if rel_speed > max_speed {
                    continue;
                }
                let density_score = self.densities[index].max(self.densities[neighbor_index]);
                if density_score < config.gas_rest_density * 1.25 {
                    continue;
                }
                merge_pairs.push((index, neighbor_index));
            }
        }
        merge_pairs.sort_unstable_by(|&(a, b), &(c, d)| {
            let max_ab = a.max(b);
            let max_cd = c.max(d);
            max_cd
                .cmp(&max_ab)
                .then_with(|| a.cmp(&c))
                .then_with(|| b.cmp(&d))
        });
        merge_pairs.dedup();
        merge_pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{Body, ParticleSegmentType};
    use ultraviolet::Vec3;

    #[test]
    fn neighbor_grid_scales_for_wide_gas_distribution() {
        let mut bodies = Vec::new();
        for i in 0..10 {
            bodies.push(Body::new(
                Vec3::new(i as f32 * 2000.0, 0.0, 0.0),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                ParticleSegmentType::Gas,
            ));
        }

        let gas_indices: Vec<usize> = (0..bodies.len()).collect();
        let mut grid = GasNeighborGrid::new();
        grid.build(&bodies, &gas_indices, 0.3);

        assert!(grid.valid());
        assert!(grid.size_x <= 128);
        assert!(grid.size_y <= 128);
        assert!(grid.size_z <= 128);
    }
}
