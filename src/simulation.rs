use crate::{
    body::Body,
    config::InformationsConfig,
    elements,
    galaxy_templates::GalaxyTemplate,
    gas::GasSystem,
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
    /// Counts and offsets for a reusable flat collision bucket grid.
    collision_cell_counts: Vec<usize>,
    collision_cell_offsets: Vec<usize>,
    collision_cell_indices: Vec<usize>,
    collision_non_empty_cells: Vec<usize>,
    gas_system: GasSystem,
    gas_volume_center: Vec3,
    gas_volume_half_size: f32,
    /// Persistent molecular bonds between element particles.
    pub molecules: Vec<crate::chemistry::Molecule>,
    /// Counter for stable molecule ids.
    next_molecule_id: u64,
    /// Known-compound lookup for formula/naming.
    compound_library: crate::chemistry::CompoundLibrary,
    /// Bond candidates collected during collision resolution.
    pending_bonds: Vec<(usize, usize)>,
    /// GPDM runtime (disabled by default in Phase 0)
    pub gpdm: crate::gpdm::GpdmRuntime,
    /// Events recorded by host operations for GPDM to consume (drained each step)
    gpdm_event_buffer: Vec<crate::gpdm::host::HostEvent>,
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

    /// Record a GPDM host event. Hosts MUST call this before mutating the
    /// bodies Vec to preserve deterministic ordering guarantees.
    fn record_gpdm_event(&mut self, event: crate::gpdm::host::HostEvent) {
        self.gpdm_event_buffer.push(event);
    }

    /// Create a HostSnapshot capturing the current body snapshot for GPDM use.
    fn gpdm_snapshot(&self) -> crate::gpdm::host::HostSnapshot {
        let bodies = self
            .bodies
            .iter()
            .map(|b| crate::gpdm::host::HostBody {
                id: b.id,
                pos: b.pos,
                vel: b.vel,
                mass: b.mass,
            })
            .collect();
        crate::gpdm::host::HostSnapshot {
            frame: self.frame,
            dt: self.dt,
            bodies,
        }
    }

    /// Apply HostEffects produced by GPDM. Basic sanity checks are performed
    /// before mutating bodies. Effects are only applied when `enable_gpdm` is
    /// set in the host config to avoid surprising side-effects while GPDM is
    /// disabled.
    fn apply_gpdm_effects(&mut self, effects: crate::gpdm::host::HostEffects) {
        if !self.config.enable_gpdm {
            return;
        }

        // Velocity impulses
        for imp in effects.velocity_impulses {
            // find the body by id
            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == imp.participant_id) {
                // sanitize impulse components
                let impulse = Self::sanitize_vec(imp.impulse);
                body.vel += impulse;
            }
        }

        // Gas heating
        for heat in effects.gas_heat {
            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == heat.participant_id) {
                if body.segment_type == crate::body::ParticleSegmentType::Gas {
                    let added = (heat.heat as f32).max(0.0);
                    body.gas_temperature = (body.gas_temperature + added).max(0.0);
                }
            }
        }

        // Mass transfers
        for mt in effects.mass_transfers {
            let amount = mt.mass as f32;
            if !amount.is_finite() || amount <= 0.0 {
                continue;
            }
            let from_idx = self.bodies.iter().position(|b| b.id == mt.from);
            let to_idx = self.bodies.iter().position(|b| b.id == mt.to);
            if let Some(fi) = from_idx {
                if self.bodies[fi].mass >= amount - 1e-9 {
                    self.bodies[fi].mass -= amount;
                    if let Some(ti) = to_idx {
                        self.bodies[ti].mass += amount;
                    }
                }
            } else if let Some(ti) = to_idx {
                // If from body missing, credit 'to' (best-effort)
                self.bodies[ti].mass += amount;
            }
        }
    }

    fn element_pair_body_mass(
        element: &elements::ElementDefinition,
        particle_mass_range: (f32, f32),
    ) -> f32 {
        let min_weight = elements::elements()
            .first()
            .map(|e| e.atomic_weight)
            .unwrap_or(1.0);
        let mass_min = particle_mass_range.0.max(f32::MIN_POSITIVE);
        let mass_max = particle_mass_range.1.max(mass_min);
        (mass_min * (element.atomic_weight / min_weight)).clamp(mass_min, mass_max)
    }

    fn element_pair_body_radius(
        element: &elements::ElementDefinition,
        mass: f32,
        radius_scale: f32,
    ) -> f32 {
        let base_radius = mass.cbrt() * 0.8 + 0.02;
        base_radius * radius_scale * element.atomic_radius_scale() * 0.22
    }

    fn gravitation_for_element(
        element: &elements::ElementDefinition,
        config: &InformationsConfig,
    ) -> f32 {
        element.effective_gravitation(
            config.element_prime_beta,
            config.enable_scientific_element_gravity,
            (
                config.element_gravity_mass_weight,
                config.element_gravity_electronegativity_weight,
                config.element_gravity_radius_weight,
                config.element_gravity_reactivity_weight,
            ),
        )
    }

    fn element_gravitation(&self, element: &elements::ElementDefinition) -> f32 {
        Self::gravitation_for_element(element, &self.config)
    }

    fn element_pair_counts(
        config: &InformationsConfig,
        total_particles: usize,
    ) -> Vec<(u8, usize)> {
        let atomic_numbers = config.selected_pair_atomic_numbers();
        let element_defs: Vec<_> = atomic_numbers
            .iter()
            .filter_map(|&z| elements::element_by_atomic_number(z))
            .collect();

        if element_defs.is_empty() {
            return Vec::new();
        }

        let element_count = element_defs.len();
        let guaranteed_per_element = usize::from(total_particles >= element_count);
        let guaranteed_total = guaranteed_per_element * element_count;
        let remaining = total_particles.saturating_sub(guaranteed_total);

        let mut counts: Vec<(u8, usize, f32)> = element_defs
            .iter()
            .map(|element| {
                let inv_weight = 1.0 / element.atomic_weight.max(1e-3);
                (element.atomic_number, guaranteed_per_element, inv_weight)
            })
            .collect();

        let total_inv_weight: f32 = counts
            .iter()
            .map(|(_, _, weight)| *weight)
            .sum::<f32>()
            .max(1e-6);
        let mut allocated = 0usize;
        for tuple in counts.iter_mut() {
            let extra = (tuple.2 / total_inv_weight) * remaining as f32;
            let extra_floor = extra.floor() as usize;
            tuple.1 += extra_floor;
            tuple.2 = extra - extra_floor as f32;
            allocated += extra_floor;
        }

        let mut leftovers = remaining.saturating_sub(allocated);
        counts.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        for tuple in counts.iter_mut() {
            if leftovers == 0 {
                break;
            }
            tuple.1 += 1;
            leftovers -= 1;
        }

        counts.into_iter().map(|(z, count, _)| (z, count)).collect()
    }

    fn weighted_random_element<F>(
        candidates: &[&'static elements::ElementDefinition],
        weight_for: F,
    ) -> &'static elements::ElementDefinition
    where
        F: Fn(&elements::ElementDefinition) -> f32,
    {
        let total_weight: f32 = candidates
            .iter()
            .map(|element| weight_for(element).max(0.0))
            .sum::<f32>()
            .max(1e-6);
        let mut target = fastrand::f32() * total_weight;

        for &element in candidates {
            let weight = weight_for(element).max(0.0);
            if target <= weight {
                return element;
            }
            target -= weight;
        }

        candidates
            .last()
            .copied()
            .expect("At least one pair element must exist")
    }

    fn element_pair_random_element(
        config: &InformationsConfig,
    ) -> &'static elements::ElementDefinition {
        let atomic_numbers = config.selected_pair_atomic_numbers();
        let candidates: Vec<_> = atomic_numbers
            .iter()
            .filter_map(|&z| elements::element_by_atomic_number(z))
            .collect();
        Self::weighted_random_element(&candidates, |element| 1.0 / element.atomic_weight.max(1e-3))
    }

    fn element_pair_random_core_element(
        config: &InformationsConfig,
    ) -> &'static elements::ElementDefinition {
        let atomic_numbers = config.selected_pair_core_atomic_numbers();
        let candidates: Vec<_> = atomic_numbers
            .iter()
            .filter_map(|&z| elements::element_by_atomic_number(z))
            .collect();
        Self::weighted_random_element(&candidates, |element| element.core_suitability())
    }

    fn apply_elemental_galaxy_pair_templates(
        mut bodies: Vec<Body>,
        config: &InformationsConfig,
        particle_mass_range: (f32, f32),
        core_element: Option<&'static elements::ElementDefinition>,
    ) -> Vec<Body> {
        if bodies.is_empty() {
            return bodies;
        }

        let selected_elements: Vec<_> = config
            .selected_pair_atomic_numbers()
            .iter()
            .filter_map(|&z| elements::element_by_atomic_number(z))
            .collect();
        if selected_elements.is_empty() {
            return bodies;
        }

        let total_assign = if core_element.is_some() {
            bodies.len().saturating_sub(1)
        } else {
            bodies.len()
        };

        let element_counts = Self::element_pair_counts(config, total_assign);
        let mut assignments: Vec<u8> = Vec::new();
        for (atomic_number, count) in element_counts.iter() {
            assignments.extend(std::iter::repeat(*atomic_number).take(*count));
        }
        if assignments.len() != total_assign {
            let fallback = selected_elements[0].atomic_number;
            assignments.resize(total_assign, fallback);
        }
        fastrand::shuffle(&mut assignments);

        if let Some(core_element) = core_element {
            let center = bodies[0].pos;
            let velocity = bodies[0].vel;
            let rotation_axis = bodies[0].rotation_axis;
            let center_mass = bodies[0].mass;
            // Body::new applies the renderer radius scale once; compensate when
            // rebuilding an existing center so its volume is not scaled twice.
            let center_radius = bodies[0].base_radius * 4.0;
            bodies[0] = Body::new_element(
                center,
                velocity,
                center_mass,
                center_radius,
                bodies[0].angular_speed,
                rotation_axis,
                core_element.atomic_number,
                crate::body::ParticleSegmentType::Core,
                core_element.prime_dimension() as f32,
                core_element.core_reactivity_score(),
                core_element.magnetism(config.element_prime_alpha),
                Self::gravitation_for_element(core_element, config),
                core_element.light_signature(config.element_light_energy),
                core_element.mass_dimension(),
            );
        }

        let start_index = if core_element.is_some() { 1 } else { 0 };
        for idx in start_index..bodies.len() {
            let atomic_number = assignments[idx - start_index];
            if let Some(element) = elements::element_by_atomic_number(atomic_number) {
                let mass = Self::element_pair_body_mass(element, particle_mass_range);
                let radius =
                    Self::element_pair_body_radius(element, mass, config.element_radius_scale);
                let angular_speed = bodies[idx].angular_speed * element.relative_velocity_scale();
                let (prime_dim, magnetism, gravitation, light, mass_dimension) = (
                    element.prime_dimension() as f32,
                    element.magnetism(config.element_prime_alpha),
                    Self::gravitation_for_element(element, config),
                    element.light_signature(config.element_light_energy),
                    element.mass_dimension(),
                );
                bodies[idx] = Body::new_element(
                    bodies[idx].pos,
                    bodies[idx].vel,
                    mass,
                    radius,
                    angular_speed,
                    bodies[idx].rotation_axis,
                    element.atomic_number,
                    bodies[idx].segment_type,
                    prime_dim,
                    element.reactivity_score(),
                    magnetism,
                    gravitation,
                    light,
                    mass_dimension,
                );
            }
        }

        bodies
    }

    pub(crate) fn rebalance_elemental_galaxy_orbits(
        bodies: &mut [Body],
        config: &InformationsConfig,
        clockwise: bool,
    ) {
        if bodies.len() < 2 {
            return;
        }

        let center = bodies[0].pos;
        let center_velocity = bodies[0].vel;
        let rotation_axis = if bodies[0].rotation_axis.mag_sq() > 1.0e-8 {
            bodies[0].rotation_axis.normalized()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        bodies[1..].sort_unstable_by(|first, second| {
            let first_offset = first.pos - center;
            let second_offset = second.pos - center;
            let first_radius =
                (first_offset - rotation_axis * first_offset.dot(rotation_axis)).mag_sq();
            let second_radius =
                (second_offset - rotation_axis * second_offset.dot(rotation_axis)).mag_sq();
            first_radius.total_cmp(&second_radius)
        });

        let mut enclosed_mass = bodies[0].mass;
        let outer_radius = config.outer_radius.max(1.0e-6);
        for body in bodies.iter_mut().skip(1) {
            let offset = body.pos - center;
            let in_plane = offset - rotation_axis * offset.dot(rotation_axis);
            let radius = in_plane.mag();
            if radius <= 1.0e-8 {
                enclosed_mass += body.mass;
                continue;
            }
            let tangent = if clockwise {
                -(rotation_axis.cross(in_plane)).normalized()
            } else {
                rotation_axis.cross(in_plane).normalized()
            };
            let circular_speed = GalaxyTemplate::softened_circular_speed(
                enclosed_mass,
                radius,
                config.effective_epsilon(),
            );
            let drift_factor = GalaxyTemplate::disk_drift_factor(radius / outer_radius, config);
            body.vel = center_velocity + tangent * circular_speed * drift_factor;
            enclosed_mass += body.mass;
        }
    }

    fn adaptive_force_enabled(&self) -> bool {
        self.config.effective_enable_adaptive_theta()
            || self.config.effective_enable_adaptive_multipole()
            || self.config.enable_mixed_precision
    }

    fn is_center_body(body: &Body) -> bool {
        body.segment_type == crate::body::ParticleSegmentType::Core
    }

    fn compute_fusion_repulsion_accel(
        octree: &Octree,
        body: &Body,
        gravitational_acc: Vec3,
        inserted_self_mass: f32,
    ) -> Vec3 {
        let grav_mag = gravitational_acc.mag();
        if grav_mag <= 0.0 || Self::is_center_body(body) {
            return Vec3::zero();
        }

        let direction = gravitational_acc.normalized();
        let local_radius = body.effective_radius().max(1.0e-4) * 6.0;
        let local_mass = octree.local_mass_within(body.pos, local_radius) - inserted_self_mass;
        let local_mass = if local_mass > 0.0 { local_mass } else { 0.0 };
        if local_mass <= 0.0 {
            return Vec3::zero();
        }

        let min_mass = body.mass.min(local_mass);
        let max_mass = body.mass.max(local_mass);
        let similarity = (min_mass / max_mass).clamp(0.0, 1.0);

        // The fusion-like pressure correction is intentionally much weaker than
        // standard gravity. It acts as a partial, analogical counter-force to
        // gravity, with a mass-similarity scaling so that unequal-mass encounters
        // remain more collision-prone than equal-mass ones.
        let pressure_scale = 0.012 * similarity;
        -direction * (grav_mag * pressure_scale)
    }

    fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }

    fn scaled_intervals(&self) -> (usize, usize) {
        let base_collision = self.config.effective_collision_interval();
        let base_attract = self.config.effective_attract_interval();
        if self.config.effective_performance_dynamic_interval_scaling()
            && self.bodies.len() > self.config.performance_body_count_threshold
        {
            let ratio =
                self.bodies.len() as f32 / self.config.performance_body_count_threshold as f32;
            let factor = ratio.sqrt().floor() as usize;
            let factor = factor
                .max(1)
                .min(self.config.performance_max_interval_scale);
            (
                base_collision.saturating_mul(factor),
                base_attract.saturating_mul(factor),
            )
        } else {
            (base_collision, base_attract)
        }
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

    fn prune_body_list_to_budget(
        bodies: Vec<Body>,
        budget: usize,
        distance_factor: f32,
    ) -> Vec<Body> {
        if bodies.len() <= budget || budget < 2 {
            return bodies;
        }

        let mut bodies = bodies;
        let center = bodies[0].pos;
        let center_body = bodies.remove(0);
        let keep_count = budget.saturating_sub(1).min(bodies.len());
        bodies.sort_unstable_by(|a, b| {
            let a_score = a.importance_score(center, distance_factor);
            let b_score = b.importance_score(center, distance_factor);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut pruned = Vec::with_capacity(keep_count + 1);
        pruned.push(center_body);
        pruned.extend(bodies.into_iter().take(keep_count));
        pruned.sort_unstable_by(|a, b| a.pos.mag_sq().total_cmp(&b.pos.mag_sq()));
        pruned
    }

    fn prune_elemental_body_list_to_budget(
        bodies: Vec<Body>,
        budget: usize,
        distance_factor: f32,
    ) -> Vec<Body> {
        if bodies.len() <= budget || budget < 2 {
            return bodies;
        }

        let center_body = bodies[0];
        let center = center_body.pos;
        let mut representatives: HashMap<u8, (Body, f32)> = HashMap::new();
        for body in bodies.iter().skip(1) {
            if body.element_atomic_number == 0 {
                continue;
            }
            let score = body.importance_score(center, distance_factor);
            let entry = representatives
                .entry(body.element_atomic_number)
                .or_insert((*body, score));
            if score > entry.1 {
                *entry = (*body, score);
            }
        }

        let mut required: Vec<Body> = representatives
            .into_values()
            .map(|(body, _)| body)
            .collect();
        required.sort_unstable_by_key(|body| body.element_atomic_number);
        required.truncate(budget.saturating_sub(1));

        let required_ids: std::collections::HashSet<u64> =
            required.iter().map(|body| body.id).collect();
        let remaining_capacity = budget.saturating_sub(1 + required.len());
        let mut optional: Vec<Body> = bodies
            .into_iter()
            .skip(1)
            .filter(|body| !required_ids.contains(&body.id))
            .collect();
        optional.sort_unstable_by(|a, b| {
            b.importance_score(center, distance_factor)
                .partial_cmp(&a.importance_score(center, distance_factor))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut pruned = Vec::with_capacity(budget);
        pruned.push(center_body);
        pruned.extend(required);
        pruned.extend(optional.into_iter().take(remaining_capacity));
        pruned[1..].sort_unstable_by(|a, b| {
            (a.pos - center)
                .mag_sq()
                .total_cmp(&(b.pos - center).mag_sq())
        });
        pruned
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

    fn spawn_accretion_for_galaxy(&mut self, galaxy_center_idx: usize) {
        if self.accretion_template.accretion_spawn_rate <= 0.0 {
            return;
        }

        if galaxy_center_idx >= self.bodies.len() {
            return;
        }
        let Some((_, _, galaxy_start, galaxy_end)) = self.galaxy_body_membership(galaxy_center_idx)
        else {
            return;
        };

        let center = self.bodies[galaxy_center_idx];
        let plane_normal = if center.rotation_axis.mag_sq() > 1e-8 {
            center.rotation_axis.normalized()
        } else {
            let normal = self.galaxy_plane_normal(
                galaxy_start,
                galaxy_end,
                center.pos,
                center.rotation_axis,
                galaxy_center_idx,
            );
            if normal.mag_sq() > 1e-8 {
                normal.normalized()
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            }
        };
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
        let in_plane_offset = (u * cos + v * sin) * r;
        let offset = in_plane_offset + plane_normal * ((fastrand::f32() - 0.5) * thickness);
        let spawn_pos = center.pos + offset;

        let tangent = if center.angular_speed >= 0.0 {
            -(plane_normal.cross(in_plane_offset)).normalized()
        } else {
            (plane_normal.cross(in_plane_offset)).normalized()
        };

        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;
        let acc = self.octree.acc(spawn_pos);
        let orbital_speed = (acc.mag() * r).sqrt().max(0.0);
        let velocity = center.vel + tangent * orbital_speed;
        let base_angular_speed = (self.config.spawn_angular_speed_base
            + fastrand::f32() * self.config.spawn_angular_speed_range
            + orbital_speed * 0.02)
            * self.config.spin_speed_multiplier;

        let mut body = if self.config.enable_elemental_galaxy_pair_mode && self.config.n > 1000 {
            let element = Self::element_pair_random_element(&self.config);
            let mass = Self::element_pair_body_mass(element, (mass_min, mass_max));
            let radius =
                Self::element_pair_body_radius(element, mass, self.config.element_radius_scale);
            Body::new_element(
                spawn_pos,
                velocity,
                mass,
                radius,
                base_angular_speed * element.relative_velocity_scale(),
                plane_normal,
                element.atomic_number,
                crate::body::ParticleSegmentType::Orbital,
                element.prime_dimension() as f32,
                element.reactivity_score(),
                element.magnetism(self.config.element_prime_alpha),
                self.element_gravitation(element),
                element.light_signature(self.config.element_light_energy),
                element.mass_dimension(),
            )
        } else {
            let mass =
                (mass_min + fastrand::f32() * (mass_max - mass_min)).clamp(mass_min, mass_max);
            Body::new(
                spawn_pos,
                velocity,
                mass,
                mass.cbrt(),
                base_angular_speed,
                plane_normal,
                crate::body::ParticleSegmentType::Orbital,
            )
        };
        Self::initialize_adaptive_state(&mut body, self.config.effective_theta(), self.dt);

        self.insert_body_at(galaxy_end.min(self.bodies.len()), body);
    }

    pub fn new(config: &InformationsConfig) -> Self {
        let dt = config.effective_dt();
        let theta = config.effective_theta();
        let epsilon = config.effective_epsilon();

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

        if config.enable_elemental_galaxy_pair_mode && config.n > 1000 {
            let reserved_gas_volume_budget =
                ((config.particle_budget as f32) * config.max_gas_particle_ratio).ceil() as usize;
            let minimum_elemental_galaxy_budget = config
                .selected_pair_atomic_numbers()
                .len()
                .saturating_add(1);
            let per_pair_budget = ((config
                .particle_budget
                .saturating_sub(reserved_gas_volume_budget))
                / num_pairs.max(1))
            .max(minimum_elemental_galaxy_budget);

            for _pair_idx in 0..num_pairs {
                let midpoint = if num_pairs == 1 {
                    Vec3::zero()
                } else {
                    Self::random_point_in_ball(scatter_radius)
                };

                let separation_dist = base_separation * (1.0 + fastrand::f32());
                let separation_dir = Self::random_direction();
                let half = separation_dir * (separation_dist * 0.5);
                let center_a3 = midpoint - half;
                let center_b3 = midpoint + half;

                let axis1 = Self::random_inclination_axis();
                let axis2 = Self::random_inclination_axis();
                let clockwise1 = fastrand::bool();
                let clockwise2 = fastrand::bool();
                let core_element_a = Self::element_pair_random_core_element(&config);
                let core_element_b = Self::element_pair_random_core_element(&config);

                let mut bodies_a = accretion_template.generate_inclined_with_config(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis1,
                    clockwise1,
                    &config,
                );
                let mut bodies_b = accretion_template.generate_inclined_with_config(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis2,
                    clockwise2,
                    &config,
                );

                bodies_a = Self::apply_elemental_galaxy_pair_templates(
                    bodies_a,
                    &config,
                    accretion_template.particle_mass_range,
                    Some(core_element_a),
                );
                bodies_b = Self::apply_elemental_galaxy_pair_templates(
                    bodies_b,
                    &config,
                    accretion_template.particle_mass_range,
                    Some(core_element_b),
                );

                if config.gas_enabled && !config.gas_volume_enabled {
                    let galaxy_count = galaxy_count.max(1);
                    let gas_per_galaxy =
                        (config.gas_meta_particle_count + galaxy_count - 1) / galaxy_count;
                    let mut gas_a = accretion_template.generate_gas_disk(
                        Vec2::zero(),
                        axis1,
                        clockwise1,
                        gas_per_galaxy,
                    );
                    let mut gas_b = accretion_template.generate_gas_disk(
                        Vec2::zero(),
                        axis2,
                        clockwise2,
                        gas_per_galaxy,
                    );
                    gas_a = Self::apply_elemental_galaxy_pair_templates(
                        gas_a,
                        &config,
                        accretion_template.particle_mass_range,
                        None,
                    );
                    gas_b = Self::apply_elemental_galaxy_pair_templates(
                        gas_b,
                        &config,
                        accretion_template.particle_mass_range,
                        None,
                    );
                    bodies_a.append(&mut gas_a);
                    bodies_b.append(&mut gas_b);
                }
                Self::rebalance_elemental_galaxy_orbits(&mut bodies_a, &config, clockwise1);
                Self::rebalance_elemental_galaxy_orbits(&mut bodies_b, &config, clockwise2);

                if bodies_a.len() > per_pair_budget {
                    bodies_a = Self::prune_elemental_body_list_to_budget(
                        bodies_a,
                        per_pair_budget,
                        config.render_lod_distance_factor,
                    );
                }
                if bodies_b.len() > per_pair_budget {
                    bodies_b = Self::prune_elemental_body_list_to_budget(
                        bodies_b,
                        per_pair_budget,
                        config.render_lod_distance_factor,
                    );
                }

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
        } else if config.enable_galactic_atom_simulation {
            let mut atomic_numbers = config.selected_atomic_numbers();
            let universe_radius =
                accretion_template.outer_radius * config.element_universe_scatter_factor;
            let center_mass_unit = config.element_center_mass_unit;
            let orbital_mass_unit = config.element_orbital_mass_unit;
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
                if config.particle_budget > 0 {
                    let element_budget = (config.particle_budget / atomic_count.max(1)).max(8);
                    if atom_bodies.len() > element_budget {
                        atom_bodies = Self::prune_body_list_to_budget(
                            atom_bodies,
                            element_budget,
                            config.render_lod_distance_factor,
                        );
                    }
                }
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
            let reserved_gas_volume_budget =
                ((config.particle_budget as f32) * config.max_gas_particle_ratio).ceil() as usize;
            let gas_volume_count = if config.gas_enabled && config.gas_volume_enabled {
                config
                    .gas_volume_count
                    .min(reserved_gas_volume_budget)
                    .max(1)
            } else {
                0
            };
            let per_pair_budget = ((config.particle_budget.saturating_sub(gas_volume_count))
                / num_pairs.max(1))
            .max(8);
            let _base_gas_volume_count = if num_pairs > 0 {
                gas_volume_count / num_pairs
            } else {
                gas_volume_count
            };
            let _extra_gas_volume = if num_pairs > 0 {
                gas_volume_count % num_pairs
            } else {
                0
            };
            for _pair_idx in 0..num_pairs {
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

                let mut bodies_a = accretion_template.generate_inclined_with_config(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis1,
                    clockwise1,
                    &config,
                );
                let mut bodies_b = accretion_template.generate_inclined_with_config(
                    Vec2::zero(),
                    Vec2::zero(),
                    axis2,
                    clockwise2,
                    &config,
                );

                if config.gas_enabled && !config.gas_volume_enabled {
                    let galaxy_count = galaxy_count.max(1);
                    let gas_per_galaxy =
                        (config.gas_meta_particle_count + galaxy_count - 1) / galaxy_count;
                    let mut gas_a = accretion_template.generate_gas_disk(
                        Vec2::zero(),
                        axis1,
                        clockwise1,
                        gas_per_galaxy,
                    );
                    let mut gas_b = accretion_template.generate_gas_disk(
                        Vec2::zero(),
                        axis2,
                        clockwise2,
                        gas_per_galaxy,
                    );
                    bodies_a.append(&mut gas_a);
                    bodies_b.append(&mut gas_b);
                }

                if bodies_a.len() > per_pair_budget {
                    bodies_a = Self::prune_body_list_to_budget(
                        bodies_a,
                        per_pair_budget,
                        config.render_lod_distance_factor,
                    );
                }
                if bodies_b.len() > per_pair_budget {
                    bodies_b = Self::prune_body_list_to_budget(
                        bodies_b,
                        per_pair_budget,
                        config.render_lod_distance_factor,
                    );
                }

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

        let (gas_volume_center, gas_volume_half_size) = if config.gas_enabled
            && config.gas_volume_enabled
        {
            let (center, half_size) = Self::compute_enclosing_gas_volume_from_bodies(
                &bodies,
                accretion_template.outer_radius,
                accretion_template.outer_radius * config.gas_volume_radius_factor,
            );
            let reserved_gas_volume_budget =
                ((config.particle_budget as f32) * config.max_gas_particle_ratio).ceil() as usize;
            let actual_gas_count = config
                .gas_volume_count
                .min(reserved_gas_volume_budget)
                .max(1);
            let mut volume_gas =
                accretion_template.generate_gas_volume(center, half_size, actual_gas_count);
            if config.enable_elemental_galaxy_pair_mode && config.n > 1000 {
                volume_gas = Self::apply_elemental_galaxy_pair_templates(
                    volume_gas,
                    &config,
                    accretion_template.particle_mass_range,
                    None,
                );
            }
            bodies.extend(volume_gas);
            (center, half_size)
        } else {
            (Vec3::zero(), 0.0)
        };

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
            collision_cell_counts: Vec::new(),
            collision_cell_offsets: Vec::new(),
            collision_cell_indices: Vec::new(),
            collision_non_empty_cells: Vec::new(),
            gas_system: GasSystem::new(),
            gas_volume_center,
            gas_volume_half_size,
            molecules: Vec::new(),
            next_molecule_id: 1,
            compound_library: crate::chemistry::CompoundLibrary::new(),
            pending_bonds: Vec::new(),
            gpdm: crate::gpdm::GpdmRuntime::disabled(),
            gpdm_event_buffer: Vec::new(),
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

    fn compute_enclosing_gas_volume_from_bodies(
        bodies: &[Body],
        galaxy_half_extent: f32,
        min_half_side: f32,
    ) -> (Vec3, f32) {
        if bodies.is_empty() {
            return (Vec3::zero(), min_half_side.max(galaxy_half_extent).max(1.0));
        }

        let mut min_corner = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max_corner = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        for body in bodies.iter() {
            let pos = body.pos;
            min_corner.x = min_corner.x.min(pos.x);
            min_corner.y = min_corner.y.min(pos.y);
            min_corner.z = min_corner.z.min(pos.z);
            max_corner.x = max_corner.x.max(pos.x);
            max_corner.y = max_corner.y.max(pos.y);
            max_corner.z = max_corner.z.max(pos.z);
        }

        let diff = max_corner - min_corner;
        let half_side = (diff.x.abs().max(diff.y.abs()).max(diff.z.abs()) * 0.5
            + galaxy_half_extent)
            .max(min_half_side)
            .max(1.0);
        let center = (min_corner + max_corner) * 0.5;
        (center, half_side)
    }

    fn sample_largest_empty_gas_position(&self, gas_positions: &[Vec3], samples: usize) -> Vec3 {
        let mut best_position = self.gas_volume_center;
        let mut best_score = -1.0_f32;
        let half_side = self.gas_volume_half_size.max(0.1);

        if gas_positions.is_empty() {
            return self.gas_volume_center;
        }

        for _ in 0..samples.max(8) {
            let candidate = Vec3::new(
                self.gas_volume_center.x + (fastrand::f32() * 2.0 - 1.0) * half_side,
                self.gas_volume_center.y + (fastrand::f32() * 2.0 - 1.0) * half_side,
                self.gas_volume_center.z + (fastrand::f32() * 2.0 - 1.0) * half_side,
            );
            let mut min_dist_sq = f32::MAX;
            for &pos in gas_positions {
                min_dist_sq = min_dist_sq.min((candidate - pos).mag_sq());
            }
            if min_dist_sq > best_score {
                best_score = min_dist_sq;
                best_position = candidate;
            }
        }
        best_position
    }

    fn gas_spawn_mass_from_position(&self, position: Vec3) -> f32 {
        let mut min_dist = f32::MAX;
        for body in self.bodies.iter() {
            if body.segment_type == crate::body::ParticleSegmentType::Core {
                let dist = (body.pos - position).mag();
                if dist < min_dist {
                    min_dist = dist;
                }
            }
        }

        let dist = min_dist.min(self.accretion_template.outer_radius);
        let ratio = if self.accretion_template.outer_radius > 0.0 {
            1.0 - (dist / self.accretion_template.outer_radius).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;
        mass_min + ratio * (mass_max - mass_min)
    }

    fn ingest_spawn_queue(&mut self) {
        let mut queue = renderer::SPAWN_QUEUE.lock();
        if !queue.is_empty() {
            for mut body in queue.drain(..) {
                Self::initialize_adaptive_state(&mut body, self.config.effective_theta(), self.dt);
                self.bodies.push(body);
            }
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
        let profile_components = std::env::var("PROFILE_STEP_COMPONENTS").is_ok();
        let mut iterate_time = 0u128;
        let mut collide_time = 0u128;
        let mut attract_time = 0u128;
        let mut classify_time = 0u128;
        let mut timestep_time = 0u128;

        if profile_components {
            let start = std::time::Instant::now();
            self.ingest_spawn_queue();
            self.iterate();
            iterate_time = start.elapsed().as_nanos();
        } else {
            self.ingest_spawn_queue();
            self.iterate();
        }

        let (collision_interval, attract_interval) = self.scaled_intervals();

        if self.frame % collision_interval == 0 {
            if renderer::COLLISIONS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                if profile_components {
                    let start = std::time::Instant::now();
                    self.collide();
                    collide_time = start.elapsed().as_nanos();
                } else {
                    self.collide();
                }
                self.process_pending_bonds();
            }
        }

        if self.frame % attract_interval == 0 {
            if profile_components {
                let start = std::time::Instant::now();
                self.attract();
                attract_time = start.elapsed().as_nanos();
            } else {
                self.attract();
            }
            // Raumzeitkrümmungs-Dilatation für Center-Partikel überschreibt Teil der Gravitation
            self.apply_center_spacetime_dilation();
            self.apply_molecular_constraints();
            self.break_overstressed_molecules();
            self.update_molecule_metadata();
        }

        if self.frame % self.config.gas_update_interval == 0 {
            self.update_gas(self.dt * self.config.gas_update_interval as f32);
        }

        if self.frame % attract_interval == 0 {
            let high_count_adaptive_skip = self.config.effective_high_count_mode()
                && self.bodies.len() > self.config.performance_body_count_threshold
                && self
                    .config
                    .effective_disable_adaptive_updates_in_high_count();

            if self.config.effective_enable_adaptive_theta()
                || self.config.effective_enable_adaptive_multipole()
            {
                if profile_components {
                    let start = std::time::Instant::now();
                    if !high_count_adaptive_skip {
                        self.classify_and_adapt();
                    }
                    classify_time = start.elapsed().as_nanos();
                } else if !high_count_adaptive_skip {
                    self.classify_and_adapt();
                }
            }
            if self.config.effective_enable_adaptive_timestep() {
                if profile_components {
                    let start = std::time::Instant::now();
                    if !high_count_adaptive_skip {
                        self.compute_adaptive_timesteps();
                    }
                    timestep_time = start.elapsed().as_nanos();
                } else if !high_count_adaptive_skip {
                    self.compute_adaptive_timesteps();
                }
            }
        }

        if self.frame % attract_interval != 0
            && self.frame % collision_interval != 0
            && profile_components
        {
            // Ensure at least the iterate time is captured even if no additional components ran.
        }

        if renderer::SPAWN_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            // Legacy paired galaxy disk mode now spawns accretion particles
            // automatically per merge within the same galaxy.
        }
        if self.config.enable_gpdm && self.gpdm.is_enabled() {
            // Forward events recorded by host to the GPDM runtime.
            let events = std::mem::take(&mut self.gpdm_event_buffer);
            for e in events {
                self.gpdm.record_event(e);
            }

            // Create a trimmed snapshot and tick the runtime. Effects are
            // applied deterministically and safely by the host.
            let snapshot = self.gpdm_snapshot();
            let effects = self.gpdm.tick(snapshot.frame, snapshot.dt);
            self.apply_gpdm_effects(effects);
        } else {
            // Keep buffer bounded when GPDM is disabled.
            self.gpdm_event_buffer.clear();
        }

        self.frame += 1;

        if profile_components {
            eprintln!(
                "[STEP_PROFILE] frame={} iterate_ms={:.3} collide_ms={:.3} attract_ms={:.3} classify_ms={:.3} timestep_ms={:.3}",
                self.frame,
                iterate_time as f64 / 1_000_000.0,
                collide_time as f64 / 1_000_000.0,
                attract_time as f64 / 1_000_000.0,
                classify_time as f64 / 1_000_000.0,
                timestep_time as f64 / 1_000_000.0
            );
        }
    }

    pub fn attract(&mut self) {
        let oct = Oct::new_containing(&self.bodies);
        let reserve_nodes = (self.bodies.len() as f32
            * self.config.performance_octree_reserve_factor)
            .ceil() as usize;
        let reserve_nodes = reserve_nodes.max(1);
        let reserve_parents = if self.config.effective_high_count_mode()
            && self.bodies.len() > self.config.performance_body_count_threshold
        {
            (self.bodies.len() / 2).max(1)
        } else {
            self.bodies.len()
        };
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
        let use_elemental_gravitation_tuning = self.config.enable_elemental_gravitation_tuning;
        let octree = &self.octree;
        self.bodies.par_iter_mut().for_each(|body| {
            body.prev_acc = body.acc;
            let base_acc = if use_adaptive_force {
                octree.acc_adaptive(body.pos, body.theta_i, body.multipole_order)
            } else {
                octree.acc(body.pos)
            };
            body.acc = Simulation::sanitize_vec(base_acc);
            if !Self::is_center_body(body) {
                let inserted_self_mass = if use_elemental_gravitation_tuning {
                    body.effective_gravitational_mass()
                } else {
                    body.mass
                };
                body.acc += Self::compute_fusion_repulsion_accel(
                    octree,
                    body,
                    body.acc,
                    inserted_self_mass,
                );
            }
        });

        self.apply_hydrodynamic_inflow();
    }

    fn classify_and_adapt(&mut self) {
        let base_theta = self.config.effective_theta().max(0.05);
        let enable_adaptive_theta = self.config.effective_enable_adaptive_theta();
        let enable_adaptive_multipole = self.config.effective_enable_adaptive_multipole();
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
        let dt_min = self.config.effective_dt_min();
        let dt_max = self.config.effective_dt_max();
        let eta_timestep = self.config.eta_timestep.max(1e-4);
        let adaptive_enabled = self.config.effective_enable_adaptive_timestep();
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

        let mut next_merge_contact_frames: HashMap<(usize, usize), usize> =
            HashMap::with_capacity(self.merge_contact_frames.len().max(32));
        let mut core_merge_candidates: Vec<(usize, usize)> = Vec::with_capacity(64);
        let mut particle_merge_pairs: Vec<(usize, usize)> = Vec::with_capacity(64);

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

        let size_x = ((max_x - min_x) / cell_size).ceil().max(0.0);
        let size_y = ((max_y - min_y) / cell_size).ceil().max(0.0);
        let size_z = ((max_z - min_z) / cell_size).ceil().max(0.0);
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
            let size_xf = ((max_x - min_x) / cell_size).ceil().max(0.0);
            let size_yf = ((max_y - min_y) / cell_size).ceil().max(0.0);
            let size_zf = ((max_z - min_z) / cell_size).ceil().max(0.0);
            if !size_xf.is_finite() || !size_yf.is_finite() || !size_zf.is_finite() {
                eprintln!("⚠ Collision skipped: invalid scaled grid dimensions");
                return;
            }
            size_x = size_xf as usize + 1;
            size_y = size_yf as usize + 1;
            size_z = size_zf as usize + 1;
        }

        size_x = size_x.max(1);
        size_y = size_y.max(1);
        size_z = size_z.max(1);

        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells > max_total_cells {
            eprintln!(
                "⚠ Collision skipped: grid cell count out of bounds ({}, {}, {})",
                size_x, size_y, size_z
            );
            return;
        }

        let mut cell_counts = std::mem::take(&mut self.collision_cell_counts);
        cell_counts.resize(total_cells, 0);
        for count in cell_counts.iter_mut() {
            *count = 0;
        }

        let mut cell_offsets = std::mem::take(&mut self.collision_cell_offsets);
        cell_offsets.resize(total_cells + 1, 0);
        for offset in cell_offsets.iter_mut() {
            *offset = 0;
        }

        let mut cell_indices = std::mem::take(&mut self.collision_cell_indices);
        let mut non_empty_cells = std::mem::take(&mut self.collision_non_empty_cells);
        non_empty_cells.clear();
        non_empty_cells.reserve(self.bodies.len().min(total_cells));

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
                cell_counts[idx] += 1;
            }
        }

        cell_offsets[0] = 0;
        for i in 0..total_cells {
            let next = cell_offsets[i] + cell_counts[i];
            cell_offsets[i + 1] = next;
            if cell_counts[i] > 0 {
                non_empty_cells.push(i);
            }
        }

        let total_indices = cell_offsets[total_cells];
        cell_indices.resize(total_indices, usize::MAX);
        if !cell_indices.is_empty() {
            cell_indices.fill(usize::MAX);
        }
        let mut write_positions = cell_offsets[..total_cells].to_vec();
        let mut total_written = 0;

        for (index, body) in self.bodies.iter().enumerate() {
            if let Some(idx) = cell_index(body.pos) {
                let write_pos = write_positions[idx];
                if write_pos >= total_indices {
                    eprintln!(
                        "⚠ Collision cell write_pos out of range: {} >= {} for body index {}",
                        write_pos, total_indices, index
                    );
                    continue;
                }
                cell_indices[write_pos] = index;
                write_positions[idx] += 1;
                total_written += 1;
            }
        }

        if total_written != total_indices {
            eprintln!(
                "⚠ Collision grid write mismatch: wrote {} of {} expected indices",
                total_written, total_indices
            );
        }

        let body_len = self.bodies.len();
        if cell_indices
            .iter()
            .any(|&idx| idx == usize::MAX || idx >= body_len)
        {
            eprintln!(
                "⚠ Collision grid contains stale or invalid cell indices; invalid entries will be skipped"
            );
        }

        let yz_stride = size_y * size_z;
        let center_pairs: Vec<(usize, usize)> = self
            .pairs
            .iter()
            .filter_map(|pair| {
                if pair.center2_idx != usize::MAX {
                    Some(Self::ordered_pair(pair.center1_idx, pair.center2_idx))
                } else {
                    None
                }
            })
            .collect();

        let mut invalid_pair_logged = false;
        for &cell_idx in non_empty_cells.iter() {
            let x = cell_idx / yz_stride;
            let yz = cell_idx % yz_stride;
            let y = yz / size_z;
            let z = yz % size_z;
            let start = cell_offsets[cell_idx];
            let end = cell_offsets[cell_idx + 1];
            let indices = &cell_indices[start..end];
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
                        let neighbor_start = cell_offsets[neighbor_idx];
                        let neighbor_end = cell_offsets[neighbor_idx + 1];
                        if neighbor_start == neighbor_end {
                            continue;
                        }
                        let neighbor_indices = &cell_indices[neighbor_start..neighbor_end];
                        for &i in indices {
                            if i == usize::MAX || i >= body_len {
                                if !invalid_pair_logged {
                                    eprintln!(
                                        "⚠ Collision invalid body pair detected and skipped; this may indicate stale collision cell data"
                                    );
                                    invalid_pair_logged = true;
                                }
                                continue;
                            }
                            for &j in neighbor_indices {
                                if j == usize::MAX || j >= body_len {
                                    if !invalid_pair_logged {
                                        eprintln!(
                                            "⚠ Collision invalid body pair detected and skipped; this may indicate stale collision cell data"
                                        );
                                        invalid_pair_logged = true;
                                    }
                                    continue;
                                }
                                if neighbor_idx == cell_idx && i >= j {
                                    continue;
                                }
                                self.resolve(
                                    i,
                                    j,
                                    &center_pairs,
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

        self.collision_cell_counts = cell_counts;
        self.collision_cell_offsets = cell_offsets;
        self.collision_cell_indices = cell_indices;
        self.collision_non_empty_cells = non_empty_cells;

        self.merge_contact_frames = next_merge_contact_frames;

        core_merge_candidates.sort_unstable();
        core_merge_candidates.dedup();
        particle_merge_pairs.sort_unstable();
        particle_merge_pairs.dedup();

        enum MergeAction {
            Center(usize, usize),
            Particle(usize, usize),
        }

        let mut merge_actions: Vec<MergeAction> = Vec::with_capacity(
            center_merge_candidates.len()
                + core_merge_candidates.len()
                + particle_merge_pairs.len(),
        );
        merge_actions.extend(
            center_merge_candidates
                .into_iter()
                .map(|(a, b)| MergeAction::Center(a, b)),
        );
        merge_actions.extend(
            core_merge_candidates
                .into_iter()
                .map(|(a, b)| MergeAction::Particle(a, b)),
        );
        merge_actions.extend(
            particle_merge_pairs
                .into_iter()
                .map(|(a, b)| MergeAction::Particle(a, b)),
        );
        merge_actions.sort_unstable_by(|a, b| {
            let (a1, a2) = match a {
                MergeAction::Center(x, y) | MergeAction::Particle(x, y) => (*x, *y),
            };
            let (b1, b2) = match b {
                MergeAction::Center(x, y) | MergeAction::Particle(x, y) => (*x, *y),
            };
            let max_a = a1.max(a2);
            let max_b = b1.max(b2);
            max_b
                .cmp(&max_a)
                .then_with(|| a2.cmp(&b2))
                .then_with(|| a1.cmp(&b1))
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

    pub fn update_gas(&mut self, dt: f32) {
        if !self.config.gas_enabled {
            return;
        }

        let gas_indices: Vec<usize> = self
            .bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| body.is_gas())
            .map(|(index, _)| index)
            .collect();

        if gas_indices.is_empty() {
            return;
        }

        if gas_indices.len() >= 2 {
            self.gas_system
                .update(&mut self.bodies, &gas_indices, &self.config, dt);

            let merge_pairs =
                self.gas_system
                    .collect_merge_candidates(&self.bodies, &gas_indices, &self.config);
            if !merge_pairs.is_empty() {
                self.merge_gas_pairs(merge_pairs);
            }
        }

        self.process_gas_ignition();
    }

    fn merge_gas_pairs(&mut self, mut merge_pairs: Vec<(usize, usize)>) {
        merge_pairs.sort_unstable_by(|&(a, b), &(c, d)| {
            let max_ab = a.max(b);
            let max_cd = c.max(d);
            max_cd
                .cmp(&max_ab)
                .then_with(|| a.cmp(&c))
                .then_with(|| b.cmp(&d))
        });
        merge_pairs.dedup();

        let mut merged = vec![false; self.bodies.len()];
        for (first, second) in merge_pairs {
            if first >= self.bodies.len() || second >= self.bodies.len() {
                continue;
            }
            if merged[first] || merged[second] {
                continue;
            }
            if self.bodies[first].is_gas() && self.bodies[second].is_gas() {
                self.merge_particle_bodies(first, second);
                merged[first] = true;
                merged[second] = true;
            }
        }
    }

    fn process_gas_ignition(&mut self) {
        let mut ignition_indices: Vec<usize> = self
            .bodies
            .iter()
            .enumerate()
            .filter_map(|(index, body)| {
                if !body.is_gas() {
                    return None;
                }
                if body.gas_density >= self.config.gas_ignition_density
                    && body.mass >= self.config.gas_ignition_mass
                    && body.gas_temperature >= self.config.gas_ignition_temperature
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        ignition_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in ignition_indices {
            if idx >= self.bodies.len() {
                continue;
            }
            let gas = self.bodies[idx];
            if !gas.is_gas() {
                continue;
            }

            let star_mass = if self
                .bodies
                .iter()
                .any(|b| b.segment_type == crate::body::ParticleSegmentType::Core)
            {
                self.gas_spawn_mass_from_position(gas.pos)
                    .clamp(self.config.gas_ignition_mass, gas.mass)
            } else {
                gas.mass
            };
            let star_radius = (star_mass.cbrt() * 0.75).max(0.02);
            let star_angular_speed =
                (gas.angular_speed.abs() * 0.75 + gas.vel.mag() * 0.1).max(0.02);
            let mut star = if gas.element_atomic_number > 0 {
                Body::new_element(
                    gas.pos,
                    gas.vel,
                    star_mass,
                    star_radius,
                    star_angular_speed,
                    gas.rotation_axis,
                    gas.element_atomic_number,
                    crate::body::ParticleSegmentType::Stellar,
                    gas.element_prime_energy,
                    gas.element_reactivity,
                    gas.element_magnetism,
                    gas.element_gravitation,
                    gas.element_light,
                    gas.element_mass_dimension,
                )
            } else {
                Body::new(
                    gas.pos,
                    gas.vel,
                    star_mass,
                    star_radius,
                    star_angular_speed,
                    gas.rotation_axis,
                    crate::body::ParticleSegmentType::Stellar,
                )
            };
            star.spin_angle = gas.spin_angle;
            let remaining_mass = gas.mass - star_mass;
            if remaining_mass <= self.config.gas_ignition_mass {
                self.bodies[idx] = star;
            } else {
                let mut remaining_gas = gas;
                remaining_gas.mass = remaining_mass;
                remaining_gas.base_radius = remaining_mass.cbrt() * 0.28;
                remaining_gas.equatorial_radius = remaining_gas.base_radius;
                remaining_gas.polar_radius = remaining_gas.base_radius;
                remaining_gas.gas_temperature = (remaining_gas.gas_temperature * 0.8).max(0.05);
                self.bodies[idx] = remaining_gas;
                self.bodies.push(star);
            }
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

        let acc = self.octree.acc(spawn_pos);
        let orbital_speed = (acc.mag() * r).sqrt();
        let vel = tangent * orbital_speed;

        let base_angular_speed = (self.config.spawn_angular_speed_base
            + fastrand::f32() * self.config.spawn_angular_speed_range
            + orbital_speed * 0.02)
            * self.config.spin_speed_multiplier;
        let mut body = if self.config.enable_elemental_galaxy_pair_mode && self.config.n > 1000 {
            let element = Self::element_pair_random_element(&self.config);
            let mass = Self::element_pair_body_mass(element, (mass_min, mass_max));
            let radius =
                Self::element_pair_body_radius(element, mass, self.config.element_radius_scale);
            Body::new_element(
                spawn_pos,
                vel,
                mass,
                radius,
                base_angular_speed * element.relative_velocity_scale(),
                Vec3::new(0.0, 0.0, 1.0),
                element.atomic_number,
                crate::body::ParticleSegmentType::Orbital,
                element.prime_dimension() as f32,
                element.reactivity_score(),
                element.magnetism(self.config.element_prime_alpha),
                self.element_gravitation(element),
                element.light_signature(self.config.element_light_energy),
                element.mass_dimension(),
            )
        } else {
            let mass = mass_min + fastrand::f32() * (mass_max - mass_min);
            Body::new(
                spawn_pos,
                vel,
                mass,
                mass.cbrt(),
                base_angular_speed,
                Vec3::new(0.0, 0.0, 1.0),
                crate::body::ParticleSegmentType::Orbital,
            )
        };
        Self::initialize_adaptive_state(&mut body, self.config.effective_theta(), self.dt);
        self.bodies.push(body);
    }

    fn resolve(
        &mut self,
        i: usize,
        j: usize,
        center_pairs: &[(usize, usize)],
        next_merge_contact_frames: &mut HashMap<(usize, usize), usize>,
        core_merge_candidates: &mut Vec<(usize, usize)>,
        particle_merge_pairs: &mut Vec<(usize, usize)>,
    ) {
        let ordered_pair = Self::ordered_pair(i, j);
        if center_pairs.contains(&ordered_pair) {
            return;
        }

        let len = self.bodies.len();
        if i >= len || j >= len {
            eprintln!(
                "⚠ Collision index skipped: invalid body pair ({}, {}) with len={}",
                i, j, len
            );
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
            // Same-element diatomic molecules (H2, N2, O2, ...) bond instead of
            // merging when molecular bonding is enabled. Core particles are
            // excluded so that galactic centers can still accrete.
            if self.config.enable_molecular_bonding
                && b1.element_atomic_number > 0
                && b1.element_atomic_number == b2.element_atomic_number
                && b1.segment_type != crate::body::ParticleSegmentType::Core
                && b2.segment_type != crate::body::ParticleSegmentType::Core
            {
                if let Some((first, second)) =
                    elements::element_by_atomic_number(b1.element_atomic_number)
                        .filter(|e| e.forms_diatomic_molecule())
                        .map(|_| (i.min(j), i.max(j)))
                {
                    self.pending_bonds.push((first, second));
                    return;
                }
            }

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

        // Molecular bonding: different elements with strong binding tendency
        // become bond candidates instead of bouncing elastically.
        if self.config.enable_molecular_bonding
            && b1.element_atomic_number > 0
            && b2.element_atomic_number > 0
            && b1.element_atomic_number != b2.element_atomic_number
        {
            if let Some((first, second)) =
                elements::element_by_atomic_number(b1.element_atomic_number)
                    .zip(elements::element_by_atomic_number(b2.element_atomic_number))
                    .and_then(|(ea, eb)| {
                        let tendency = ea.binding_tendency(eb);
                        let threshold = 0.25 / self.config.molecule_bond_strength_factor.max(0.1);
                        if tendency.strength >= threshold && tendency.kind != elements::BondKind::Inert
                        {
                            Some((i.min(j), i.max(j)))
                        } else {
                            None
                        }
                    })
            {
                self.pending_bonds.push((first, second));
                // Reduce elastic bounce for strong bonds so the bodies stay close.
                return;
            }
        }

        let v1 = b1.vel;
        let v2 = b2.vel;

        let v = v2 - v1;

        let d_dot_v = d.dot(v);

        let m1 = b1.mass;
        let m2 = b2.mass;
        let collision_response = if self.config.enable_elemental_binding_response
            && b1.element_atomic_number > 0
            && b2.element_atomic_number > 0
        {
            let binding_strength = elements::element_by_atomic_number(b1.element_atomic_number)
                .zip(elements::element_by_atomic_number(b2.element_atomic_number))
                .map(|(first, second)| {
                    let binding = first.binding_tendency(second);
                    let bond_factor = match binding.kind {
                        elements::BondKind::Ionic => 1.0,
                        elements::BondKind::PolarCovalent => 0.9,
                        elements::BondKind::NonpolarCovalent => 0.8,
                        elements::BondKind::Metallic => 0.75,
                        elements::BondKind::Inert => 0.2,
                    };
                    let complementary_factor = if first.is_complementary_pair(second) {
                        1.08
                    } else {
                        1.0
                    };
                    (binding.strength
                        * bond_factor
                        * (0.8 + binding.polarity * 0.2)
                        * complementary_factor)
                        .clamp(0.0, 1.0)
                })
                .unwrap_or(0.0);
            (1.5 - binding_strength * self.config.elemental_binding_response_strength * 0.45)
                .clamp(1.05, 1.5)
        } else {
            1.5
        };

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
            let tmp = d * (collision_response * d_dot_v / d_sq);
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

        let collision_pos = (self.bodies[i].pos + self.bodies[j].pos) * 0.5;
        self.bodies[i].process_flash = self.bodies[i].process_flash.max(0.6);
        self.bodies[j].process_flash = self.bodies[j].process_flash.max(0.6);
        renderer::PROCESS_EVENTS.lock().push(renderer::ProcessEvent {
            pos: collision_pos,
            kind: renderer::ProcessEventKind::Collision,
            frame: self.frame,
            strength: (m1 + m2) * 0.5,
        });
    }

    fn merge_particle_bodies(&mut self, i: usize, j: usize) {
        let mut first = i;
        let mut second = j;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }

        let merge_membership = if let (
            Some((pair_idx_a, center_idx_a, start_a, end_a)),
            Some((pair_idx_b, center_idx_b, _start_b, _end_b)),
        ) = (
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
        let merged_angular_speed =
            (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        let mut merged_body = Body::new_element(
            merged_pos,
            merged_vel,
            total_mass,
            merged_radius * 4.0,
            merged_angular_speed,
            axis,
            b1.element_atomic_number,
            b1.segment_type,
            (b1.element_prime_energy * b1.mass + b2.element_prime_energy * b2.mass) / total_mass,
            (b1.element_reactivity * b1.mass + b2.element_reactivity * b2.mass) / total_mass,
            (b1.element_magnetism * b1.mass + b2.element_magnetism * b2.mass) / total_mass,
            (b1.element_gravitation * b1.mass + b2.element_gravitation * b2.mass) / total_mass,
            (b1.element_light * b1.mass + b2.element_light * b2.mass) / total_mass,
            (b1.element_mass_dimension * b1.mass + b2.element_mass_dimension * b2.mass)
                / total_mass,
        );
        Self::initialize_adaptive_state(&mut merged_body, self.config.effective_theta(), self.dt);
        merged_body.process_flash = 1.0;
        self.bodies.remove(second);
        self.shift_pair_indices_after_removal(second);
        self.shift_contact_frames_after_removal(second);
        self.shift_molecule_indices_after_removal(second);

        renderer::PROCESS_EVENTS.lock().push(renderer::ProcessEvent {
            pos: merged_pos,
            kind: renderer::ProcessEventKind::Merge,
            frame: self.frame,
            strength: total_mass,
        });

        if let Some((_pair_idx, center_idx, _galaxy_start, _galaxy_end)) = merge_membership {
            let adjusted_center_idx = if center_idx > second {
                center_idx - 1
            } else {
                center_idx
            };
            self.spawn_accretion_for_galaxy(adjusted_center_idx);
        }

        if self.config.gas_enabled && self.config.gas_volume_enabled && merged_body.is_gas() {
            let gas_positions: Vec<Vec3> = self
                .bodies
                .iter()
                .filter(|b| b.is_gas())
                .map(|b| b.pos)
                .collect();
            let replacement_mass = (total_mass * 0.12)
                .clamp(
                    self.config.gas_ignition_mass,
                    self.accretion_template.particle_mass_range.1 * 0.25,
                )
                .min(total_mass * 0.35);
            if replacement_mass > self.config.gas_ignition_mass
                && merged_body.mass > replacement_mass * 1.05
            {
                merged_body.mass -= replacement_mass;
                merged_body.base_radius = merged_body.mass.cbrt() * 0.28;
                merged_body.equatorial_radius = merged_body.base_radius;
                merged_body.polar_radius = merged_body.base_radius;

                let spawn_pos = self.sample_largest_empty_gas_position(&gas_positions, 32);
                let mut spawn_gas = if merged_body.element_atomic_number > 0 {
                    Body::new_element(
                        spawn_pos,
                        Vec3::zero(),
                        replacement_mass,
                        replacement_mass.cbrt() * 0.28,
                        0.0,
                        merged_body.rotation_axis,
                        merged_body.element_atomic_number,
                        crate::body::ParticleSegmentType::Gas,
                        merged_body.element_prime_energy,
                        merged_body.element_reactivity,
                        merged_body.element_magnetism,
                        merged_body.element_gravitation,
                        merged_body.element_light,
                        merged_body.element_mass_dimension,
                    )
                } else {
                    Body::new(
                        spawn_pos,
                        Vec3::zero(),
                        replacement_mass,
                        replacement_mass.cbrt() * 0.28,
                        0.0,
                        Vec3::new(0.0, 0.0, 1.0),
                        crate::body::ParticleSegmentType::Gas,
                    )
                };
                spawn_gas.gas_temperature = 1.0;
                spawn_gas.gas_smoothing_radius = self.gas_volume_half_size.max(1.0) * 0.06;
                self.bodies.push(spawn_gas);
            }
        }

        self.bodies[first] = merged_body;
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

    fn shift_molecule_indices_after_removal(&mut self, removed_idx: usize) {
        for molecule in self.molecules.iter_mut() {
            molecule.body_indices.retain(|&idx| idx != removed_idx);
            for idx in molecule.body_indices.iter_mut() {
                if *idx > removed_idx {
                    *idx -= 1;
                }
            }
        }
        self.molecules.retain(|m| m.body_indices.len() >= 2);
        for body in self.bodies.iter_mut() {
            if body.molecule_id.is_some() {
                let still_exists = self
                    .molecules
                    .iter()
                    .any(|m| m.id == body.molecule_id.unwrap());
                if !still_exists {
                    body.molecule_id = None;
                }
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
        let merged_angular_speed =
            (b1.angular_speed.abs() * b1.mass + b2.angular_speed.abs() * b2.mass) / total_mass;
        let first_element = elements::element_by_atomic_number(b1.element_atomic_number);
        let second_element = elements::element_by_atomic_number(b2.element_atomic_number);
        let merged_element = match (first_element, second_element) {
            (Some(first), Some(second)) => {
                let first_score = first.core_suitability() * b1.mass;
                let second_score = second.core_suitability() * b2.mass;
                Some(if first_score >= second_score {
                    first
                } else {
                    second
                })
            }
            (Some(element), None) | (None, Some(element)) => Some(element),
            (None, None) => None,
        };
        let mut merged_body = if let Some(element) = merged_element {
            Body::new_element(
                merged_pos,
                merged_vel,
                total_mass,
                merged_radius * 4.0,
                merged_angular_speed,
                axis,
                element.atomic_number,
                crate::body::ParticleSegmentType::Core,
                element.prime_dimension() as f32,
                element.core_reactivity_score(),
                element.magnetism(self.config.element_prime_alpha),
                self.element_gravitation(element),
                element.light_signature(self.config.element_light_energy),
                element.mass_dimension(),
            )
        } else {
            Body::new(
                merged_pos,
                merged_vel,
                total_mass,
                merged_radius * 4.0,
                merged_angular_speed,
                axis,
                crate::body::ParticleSegmentType::Core,
            )
        };
        Self::initialize_adaptive_state(&mut merged_body, self.config.effective_theta(), self.dt);
        merged_body.process_flash = 1.0;
        self.bodies[first] = merged_body;
        self.bodies.remove(second);
        self.shift_contact_frames_after_removal(second);
        self.shift_molecule_indices_after_removal(second);

        renderer::PROCESS_EVENTS.lock().push(renderer::ProcessEvent {
            pos: merged_pos,
            kind: renderer::ProcessEventKind::Merge,
            frame: self.frame,
            strength: total_mass,
        });

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

    // -----------------------------------------------------------------------
    // Molecular chemistry subsystem
    // -----------------------------------------------------------------------

    /// Converts pending collision bond candidates into persistent molecules.
    fn process_pending_bonds(&mut self) {
        if self.pending_bonds.is_empty() {
            return;
        }

        // Deduplicate and order candidates deterministically.
        let mut candidates: Vec<(usize, usize)> = self.pending_bonds.drain(..).collect();
        candidates.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        candidates.dedup();

        // Build a map from body index to molecule id for fast membership tests.
        let mut body_to_molecule: std::collections::HashMap<usize, u64> =
            std::collections::HashMap::new();
        for molecule in &self.molecules {
            for &idx in &molecule.body_indices {
                body_to_molecule.insert(idx, molecule.id);
            }
        }

        for (i, j) in candidates {
            if i >= self.bodies.len() || j >= self.bodies.len() {
                continue;
            }
            let b1 = &self.bodies[i];
            let b2 = &self.bodies[j];
            let same_element = b1.element_atomic_number == b2.element_atomic_number;
            let diatomic_pair = same_element
                && elements::element_by_atomic_number(b1.element_atomic_number)
                    .map(|e| e.forms_diatomic_molecule())
                    .unwrap_or(false);
            if b1.element_atomic_number == 0
                || b2.element_atomic_number == 0
                || (same_element && !diatomic_pair)
            {
                continue;
            }

            // Respect maximum molecule size.
            let combined_size = match (body_to_molecule.get(&i), body_to_molecule.get(&j)) {
                (Some(a), Some(b)) if a == b => continue,
                (Some(a), Some(b)) => {
                    let size_a = self
                        .molecules
                        .iter()
                        .find(|m| m.id == *a)
                        .map(|m| m.body_indices.len())
                        .unwrap_or(1);
                    let size_b = self
                        .molecules
                        .iter()
                        .find(|m| m.id == *b)
                        .map(|m| m.body_indices.len())
                        .unwrap_or(1);
                    size_a + size_b
                }
                (Some(a), None) | (None, Some(a)) => {
                    let size_a = self
                        .molecules
                        .iter()
                        .find(|m| m.id == *a)
                        .map(|m| m.body_indices.len())
                        .unwrap_or(1);
                    size_a + 1
                }
                (None, None) => 2,
            };
            if combined_size > self.config.molecule_max_bodies {
                continue;
            }

            if let Some((ea, eb)) = elements::element_by_atomic_number(b1.element_atomic_number)
                .zip(elements::element_by_atomic_number(b2.element_atomic_number))
            {
                let tendency = ea.binding_tendency(eb);
                if tendency.kind == elements::BondKind::Inert || tendency.strength < 0.05 {
                    continue;
                }

                let new_id = match (body_to_molecule.get(&i), body_to_molecule.get(&j)) {
                    (Some(a), Some(b)) => {
                        if a == b {
                            continue;
                        }
                        // Merge two existing molecules.
                        let mut merged_id = None;
                        let mut idx_a = None;
                        let mut idx_b = None;
                        for (idx, m) in self.molecules.iter().enumerate() {
                            if m.id == *a {
                                idx_a = Some(idx);
                            } else if m.id == *b {
                                idx_b = Some(idx);
                            }
                        }
                        if let (Some(ia), Some(ib)) = (idx_a, idx_b) {
                            let other = self.molecules[ib].clone();
                            self.molecules[ia].merge(&other);
                            let id = self.molecules[ia].id;
                            merged_id = Some(id);
                            // Remove the merged-away molecule (higher index first).
                            let remove_idx = ia.max(ib);
                            self.molecules.remove(remove_idx);
                        }
                        merged_id
                    }
                    (Some(id), None) | (None, Some(id)) => {
                        // Add a free body to an existing molecule.
                        let existing_id = *id;
                        if let Some(m) = self.molecules.iter_mut().find(|m| m.id == existing_id) {
                            let new_body = if m.body_indices.contains(&i) { j } else { i };
                            m.body_indices.push(new_body);
                            Some(existing_id)
                        } else {
                            None
                        }
                    }
                    (None, None) => {
                        // Create a brand new molecule.
                        let id = self.next_molecule_id;
                        self.next_molecule_id += 1;
                        self.molecules.push(crate::chemistry::Molecule::new(id, vec![i, j]));
                        Some(id)
                    }
                };

                if let Some(mol_id) = new_id {
                    self.bodies[i].molecule_id = Some(mol_id);
                    self.bodies[j].molecule_id = Some(mol_id);
                    self.bodies[i].process_flash = self.bodies[i].process_flash.max(0.85);
                    self.bodies[j].process_flash = self.bodies[j].process_flash.max(0.85);
                    let bond_pos = (self.bodies[i].pos + self.bodies[j].pos) * 0.5;
                    renderer::PROCESS_EVENTS.lock().push(renderer::ProcessEvent {
                        pos: bond_pos,
                        kind: renderer::ProcessEventKind::Bond,
                        frame: self.frame,
                        strength: tendency.strength,
                    });
                    // Rebuild membership map for subsequent candidates.
                    if let Some(m) = self.molecules.iter().find(|m| m.id == mol_id) {
                        for &idx in &m.body_indices {
                            body_to_molecule.insert(idx, mol_id);
                        }
                    }
                }
            }
        }
    }

    /// Applies soft spring and damping forces to keep bonded bodies together.
    fn apply_molecular_constraints(&mut self) {
        if self.molecules.is_empty() {
            return;
        }

        let _dt = self.dt.max(1e-6);
        let stiffness = 0.15 * self.config.molecule_bond_strength_factor;
        let damping = 0.08;

        for molecule in &self.molecules {
            for window in molecule.body_indices.windows(2) {
                let (i, j) = (window[0], window[1]);
                if i >= self.bodies.len() || j >= self.bodies.len() {
                    continue;
                }
                // Copy all read-only state locally, then mutate through a raw pointer.
                let (pos1, vel1, mass1, radius1) = {
                    let b = &self.bodies[i];
                    (b.pos, b.vel, b.mass, b.effective_radius())
                };
                let (pos2, vel2, mass2, radius2) = {
                    let b = &self.bodies[j];
                    (b.pos, b.vel, b.mass, b.effective_radius())
                };
                let delta = pos2 - pos1;
                let distance = delta.mag().max(1e-6);
                let target = radius1 + radius2;
                let displacement = distance - target;
                let direction = delta / distance;

                let relative_vel = vel2 - vel1;
                let spring_force = direction * (displacement * stiffness);
                let damping_force = relative_vel * damping;

                let force = spring_force - damping_force;
                let m1 = mass1.max(1e-6);
                let m2 = mass2.max(1e-6);

                let bodies_ptr = self.bodies.as_mut_ptr();
                unsafe {
                    (*bodies_ptr.add(i)).acc += force / m1;
                    (*bodies_ptr.add(j)).acc -= force / m2;
                }

                // Avg velocities toward molecule centre of mass to stabilise rotation.
                let cm = (pos1 * m1 + pos2 * m2) / (m1 + m2);
                let cm_vel = (vel1 * m1 + vel2 * m2) / (m1 + m2);
                let pull = (cm - (pos1 + pos2) * 0.5) * 0.01;
                unsafe {
                    (*bodies_ptr.add(i)).acc += pull / m1;
                    (*bodies_ptr.add(j)).acc += pull / m2;
                    (*bodies_ptr.add(i)).vel += (cm_vel - vel1) * 0.002;
                    (*bodies_ptr.add(j)).vel += (cm_vel - vel2) * 0.002;
                }
            }
        }
    }

    /// Splits molecules when internal kinetic energy exceeds the break threshold.
    fn break_overstressed_molecules(&mut self) {
        let threshold = self.config.molecule_break_energy_threshold;
        if threshold <= 0.0 {
            return;
        }

        let mut survivors = Vec::with_capacity(self.molecules.len());
        for molecule in self.molecules.drain(..) {
            let mut max_stress = 0.0f32;
            for window in molecule.body_indices.windows(2) {
                let (i, j) = (window[0], window[1]);
                if i >= self.bodies.len() || j >= self.bodies.len() {
                    continue;
                }
                let b1 = &self.bodies[i];
                let b2 = &self.bodies[j];
                let relative_speed_sq = (b1.vel - b2.vel).mag_sq();
                let reduced_mass = (b1.mass * b2.mass) / (b1.mass + b2.mass).max(1e-6);
                let kinetic_energy = 0.5 * reduced_mass * relative_speed_sq;
                max_stress = max_stress.max(kinetic_energy);
            }
            if max_stress <= threshold {
                survivors.push(molecule);
            } else {
                for &idx in &molecule.body_indices {
                    if idx < self.bodies.len() {
                        self.bodies[idx].molecule_id = None;
                    }
                }
            }
        }
        self.molecules = survivors;
    }

    /// Recomputes formula, mass, polarity and name for each molecule.
    fn update_molecule_metadata(&mut self) {
        if !self.config.molecule_identify_compounds {
            for molecule in &mut self.molecules {
                molecule.mass = molecule
                    .body_indices
                    .iter()
                    .filter_map(|&idx| self.bodies.get(idx).map(|b| b.mass))
                    .sum();
            }
            return;
        }

        for molecule in &mut self.molecules {
            let mut counts: std::collections::HashMap<u8, usize> =
                std::collections::HashMap::new();
            let mut mass = 0.0f32;
            let mut polarity_sum = 0.0f32;
            let mut polarity_count = 0usize;
            for &idx in &molecule.body_indices {
                if let Some(body) = self.bodies.get(idx) {
                    mass += body.mass;
                    if body.element_atomic_number > 0 {
                        *counts.entry(body.element_atomic_number).or_insert(0) += 1;
                        if let Some(element) =
                            elements::element_by_atomic_number(body.element_atomic_number)
                        {
                            polarity_sum += element.effective_electronegativity();
                            polarity_count += 1;
                        }
                    }
                }
            }
            molecule.mass = mass;

            if let Some(compound) = self.compound_library.identify(&counts) {
                molecule.formula = compound.formula.to_string();
                molecule.name = compound.name.to_string();
                molecule.state = compound.state;
                molecule.polarity = compound.polarity;
            } else {
                // Build a simple empirical formula string.
                let mut parts: Vec<(u8, usize)> = counts.into_iter().collect();
                parts.sort_by(|a, b| a.0.cmp(&b.0));
                let formula = parts
                    .iter()
                    .filter_map(|(z, count)| {
                        elements::element_by_atomic_number(*z).map(|e| {
                            if *count == 1 {
                                e.symbol.to_string()
                            } else {
                                format!("{}{}", e.symbol, count)
                            }
                        })
                    })
                    .collect::<String>();
                molecule.formula = formula;
                molecule.name.clear();
                molecule.state = 2;
                molecule.polarity = if polarity_count > 1 {
                    let avg = polarity_sum / polarity_count as f32;
                    let variance = molecule
                        .body_indices
                        .iter()
                        .filter_map(|&idx| {
                            self.bodies.get(idx).and_then(|b| {
                                elements::element_by_atomic_number(b.element_atomic_number)
                                    .map(|e| (e.effective_electronegativity() - avg).abs())
                            })
                        })
                        .sum::<f32>()
                        / polarity_count as f32;
                    (variance / 3.28).clamp(0.0, 1.0)
                } else {
                    0.0
                };
            }
        }
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
            assert_eq!(
                sim.bodies.len(),
                2,
                "core bodies should not merge before frame 6"
            );
        }

        sim.collide();
        assert_eq!(
            sim.bodies.len(),
            1,
            "core bodies should merge after sustained contact"
        );
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
        assert_eq!(
            sim.bodies.len(),
            1,
            "same segment and element particles should merge after sustained contact"
        );
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
        assert_eq!(
            sim.bodies.len(),
            2,
            "different segment particles should not merge on contact"
        );
    }

    #[test]
    fn collide_handles_reused_collision_grid_buffers() {
        let config = InformationsConfig::default();
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.collision_cell_counts = vec![1; 128];
        sim.collision_cell_offsets = vec![0; 129];
        sim.collision_cell_indices = vec![usize::MAX; 256];
        sim.collision_non_empty_cells = vec![0; 16];

        for i in 0..8 {
            let pos = Vec3::new(i as f32 * 1.5, 0.0, 0.0);
            let body = Body::new(
                pos,
                Vec3::zero(),
                1.0,
                0.5,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                ParticleSegmentType::Orbital,
            );
            sim.bodies.push(body);
        }

        sim.collide();
        assert_eq!(sim.bodies.len(), 8);
        assert!(sim
            .collision_cell_indices
            .iter()
            .all(|&idx| idx < sim.bodies.len()));
    }

    #[test]
    fn collide_handles_large_random_body_sets_without_crash() {
        let config = InformationsConfig::default();
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.collision_cell_counts.clear();
        sim.collision_cell_offsets.clear();
        sim.collision_cell_indices.clear();
        sim.collision_non_empty_cells.clear();

        fastrand::seed(12345);
        for _ in 0..500 {
            let pos = Vec3::new(
                fastrand::f32() * 200.0 - 100.0,
                fastrand::f32() * 200.0 - 100.0,
                fastrand::f32() * 200.0 - 100.0,
            );
            let body = Body::new(
                pos,
                Vec3::zero(),
                1.0,
                0.5,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                ParticleSegmentType::Orbital,
            );
            sim.bodies.push(body);
        }

        sim.collide();
        sim.collide();
        assert!(
            sim.collision_cell_indices.len() > 0,
            "Expected collision grid to be built"
        );
    }

    #[test]
    fn accretion_spawn_uses_galaxy_rotation_axis_plane() {
        let mut config = InformationsConfig::default();
        config.accretion_spawn_rate = 1.0;
        config.outer_radius = 10.0;
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.octree = Octree::new(config.theta, config.epsilon);

        let axis = Vec3::new(0.2, 0.8, 1.0).normalized();
        let center = Body::new(
            Vec3::zero(),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            axis,
            ParticleSegmentType::Core,
        );
        let orbital = Body::new(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::zero(),
            0.1,
            0.1,
            0.0,
            axis,
            ParticleSegmentType::Orbital,
        );
        sim.bodies.push(center);
        sim.bodies.push(orbital);
        sim.pairs.push(GalaxyPairState {
            center1_idx: 0,
            center2_idx: usize::MAX,
            range_start: 0,
            range_end: 2,
            center_contact_frames: 0,
        });

        let oct = Oct::new_containing(&sim.bodies);
        sim.octree.clear(oct);
        sim.octree.insert(sim.bodies[0].pos, sim.bodies[0].mass);
        sim.octree.insert(sim.bodies[1].pos, sim.bodies[1].mass);
        sim.octree.propagate();

        sim.spawn_accretion_for_galaxy(0);
        assert_eq!(sim.bodies.len(), 3);

        let spawned = &sim.bodies[2];
        let spawn_offset = spawned.pos - sim.bodies[0].pos;
        let distance_from_plane = spawn_offset.dot(axis).abs();
        assert!(
            distance_from_plane <= config.outer_radius * 0.05 + 1e-3,
            "spawned body must lie close to the assigned galaxy plane"
        );
        let velocity_plane_dot = spawned.vel.dot(axis).abs();
        assert!(
            velocity_plane_dot <= spawned.vel.mag() * 0.05 + 1e-4,
            "spawned body velocity should lie mostly in the galaxy plane"
        );
    }

    #[test]
    fn scaled_intervals_increase_for_high_body_counts() {
        let mut config = InformationsConfig::default();
        config.collision_interval = 1;
        config.attract_interval = 1;
        config.performance_dynamic_interval_scaling = true;
        config.performance_body_count_threshold = 100;
        config.performance_max_interval_scale = 4;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        for _ in 0..500 {
            sim.bodies.push(Body::new(
                Vec3::zero(),
                Vec3::zero(),
                1.0,
                1.0,
                0.0,
                Vec3::new(0.0, 0.0, 1.0),
                ParticleSegmentType::Orbital,
            ));
        }

        let (collision_interval, attract_interval) = sim.scaled_intervals();
        assert!(
            collision_interval >= 2 && attract_interval >= 2,
            "intervals should scale up for high body counts"
        );
    }

    #[test]
    fn collide_reuses_collision_grid_without_panic() {
        let config = InformationsConfig::default();
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.bodies.push(Body::new(
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Orbital,
        ));
        sim.bodies.push(Body::new(
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Orbital,
        ));

        sim.collide();
        sim.collide();
        assert!(
            sim.collision_cell_indices.len() > 0,
            "Collision grid buffer should exist after collide"
        );
    }

    #[test]
    fn gas_update_computes_density_and_cooling() {
        let mut config = InformationsConfig::default();
        config.gas_enabled = true;
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();

        sim.bodies.push(Body::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        ));
        sim.bodies.push(Body::new(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.2, 0.0),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        ));

        sim.update_gas(sim.dt);
        assert!(sim.bodies[0].gas_density > 0.0);
        assert!(sim.bodies[1].gas_density > 0.0);
        assert!(sim.bodies[0].gas_temperature <= 1.0);
        assert!(sim.bodies[1].gas_temperature <= 1.0);
    }

    #[test]
    fn gas_rotation_relaxation_damps_out_of_plane_velocity() {
        let mut config = InformationsConfig::default();
        config.gas_enabled = true;
        config.gas_relaxation_strength = 0.5;
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();

        let mut gas = Body::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        );
        gas.gas_temperature = 1.0;
        sim.bodies.push(gas);
        sim.bodies.push(Body::new(
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -0.1),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        ));

        sim.update_gas(sim.dt);
        assert!(sim.bodies[0].vel.z.abs() < 1.0);
    }

    #[test]
    fn gas_ignition_converts_dense_gas_to_stellar_body() {
        let mut config = InformationsConfig::default();
        config.dt = 1.0;
        config.gas_enabled = true;
        config.gas_ignition_density = 0.000001;
        config.gas_ignition_temperature = 0.01;
        config.gas_ignition_mass = 0.000001;
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();

        let mut gas = Body::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            0.00001,
            0.01,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        );
        gas.gas_density = 0.2;
        gas.gas_temperature = 0.5;
        sim.bodies.push(gas);
        sim.bodies.push(Body::new(
            Vec3::new(0.25, 0.0, 0.0),
            Vec3::zero(),
            0.00001,
            0.01,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Gas,
        ));

        sim.update_gas(sim.dt);
        assert!(sim.bodies[0].segment_type == ParticleSegmentType::Stellar);
    }

    #[test]
    fn galaxy_pair_mode_generates_gas_particles() {
        let mut config = InformationsConfig::default();
        config.gas_enabled = true;
        config.enable_galactic_atom_simulation = false;
        config.gas_meta_particle_count = 32;
        config.galaxy_count = 4;
        let sim = Simulation::new(&config);
        assert!(sim
            .bodies
            .iter()
            .any(|body| body.segment_type == ParticleSegmentType::Gas));
    }

    #[test]
    fn galaxy_pair_mode_generates_volume_gas_within_enclosing_cube() {
        let mut config = InformationsConfig::default();
        config.gas_enabled = true;
        config.gas_volume_enabled = true;
        config.gas_volume_count = 128;
        config.enable_galactic_atom_simulation = false;
        config.galaxy_count = 4;

        let sim = Simulation::new(&config);
        assert!(sim.gas_volume_half_size > 0.0);

        let cube_center = sim.gas_volume_center;
        let cube_half = sim.gas_volume_half_size;

        let gas_bodies: Vec<&Body> = sim.bodies.iter().filter(|b| b.is_gas()).collect();
        assert!(gas_bodies.len() >= config.gas_volume_count);

        for gas in gas_bodies {
            let offset = gas.pos - cube_center;
            assert!(offset.x.abs() <= cube_half + 1e-3);
            assert!(offset.y.abs() <= cube_half + 1e-3);
            assert!(offset.z.abs() <= cube_half + 1e-3);
        }
    }

    #[test]
    fn elemental_pair_distribution_is_inverse_to_atomic_weight() {
        let mut config = InformationsConfig::default();
        config.element_pair_atomic_numbers = "1,8,118".to_string();
        let counts = Simulation::element_pair_counts(&config, 10_000);
        let count_for = |atomic_number| {
            counts
                .iter()
                .find(|(number, _)| *number == atomic_number)
                .map(|(_, count)| *count)
                .unwrap()
        };

        assert!(count_for(1) > count_for(8));
        assert!(count_for(8) > count_for(118));
        assert_eq!(counts.iter().map(|(_, count)| count).sum::<usize>(), 10_000);
    }

    #[test]
    fn elemental_pair_mode_types_every_initial_body_and_uses_dedicated_cores() {
        let mut config = InformationsConfig::default();
        config.n = 1001;
        config.particle_budget = 500;
        config.galaxy_count = 2;
        config.gas_enabled = true;
        config.gas_volume_enabled = true;
        config.gas_volume_count = 24;
        config.enable_galactic_atom_simulation = false;
        config.enable_elemental_galaxy_pair_mode = true;
        config.element_pair_atomic_numbers = "1-118".to_string();
        config.element_pair_core_atomic_numbers = "26".to_string();

        let sim = Simulation::new(&config);
        assert!(!sim.bodies.is_empty());
        assert!(sim.bodies.iter().all(|body| body.element_atomic_number > 0));
        let pair = sim.pairs[0];
        for range in [
            pair.range_start..pair.center2_idx,
            pair.center2_idx..pair.range_end,
        ] {
            let represented: std::collections::HashSet<u8> = sim.bodies[range]
                .iter()
                .map(|body| body.element_atomic_number)
                .collect();
            for atomic_number in 1..=118 {
                assert!(
                    represented.contains(&atomic_number),
                    "element {atomic_number} must survive pruning in each galaxy"
                );
            }
        }

        let cores: Vec<_> = sim
            .bodies
            .iter()
            .filter(|body| body.segment_type == ParticleSegmentType::Core)
            .collect();
        assert_eq!(cores.len(), 2);
        assert!(cores.iter().all(|body| body.element_atomic_number == 26));
        assert!(cores.iter().all(|body| body.element_reactivity
            < elements::element_by_atomic_number(26)
                .unwrap()
                .reactivity_score()));
        assert!(sim
            .bodies
            .iter()
            .filter(|body| body.segment_type == ParticleSegmentType::Gas)
            .all(|body| body.element_atomic_number > 0));
    }

    #[test]
    fn elemental_accretion_spawn_uses_selected_element_template() {
        let mut config = InformationsConfig::default();
        config.n = 1001;
        config.particle_budget = 300;
        config.galaxy_count = 2;
        config.gas_enabled = false;
        config.accretion_spawn_rate = 1.0;
        config.enable_elemental_galaxy_pair_mode = true;
        config.element_pair_atomic_numbers = "8".to_string();
        config.element_pair_core_atomic_numbers = "26".to_string();

        let mut sim = Simulation::new(&config);
        let pair = sim.pairs[0];
        let before = sim.bodies.len();
        let expected_spawn_index = pair.center2_idx;
        let oct = Oct::new_containing(&sim.bodies);
        sim.octree.clear(oct);
        for body in &sim.bodies {
            sim.octree.insert(body.pos, body.mass);
        }
        sim.octree.propagate();
        sim.spawn_accretion_for_galaxy(pair.center1_idx);

        assert_eq!(sim.bodies.len(), before + 1);
        let membership = sim
            .galaxy_body_membership(expected_spawn_index)
            .expect("spawned body must remain in its source galaxy range");
        assert_eq!(membership.1, sim.pairs[0].center1_idx);
        assert!(sim.bodies.iter().all(|body| body.element_atomic_number > 0));
        assert!(sim
            .bodies
            .iter()
            .filter(|body| body.segment_type == ParticleSegmentType::Orbital)
            .all(|body| body.element_atomic_number == 8));
    }

    #[test]
    fn elemental_gas_ignition_preserves_element_identity() {
        let mut config = InformationsConfig::default();
        config.gas_ignition_density = 0.1;
        config.gas_ignition_temperature = 0.1;
        config.gas_ignition_mass = 0.1;
        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();

        let oxygen = elements::element_by_atomic_number(8).unwrap();
        let mut gas = Body::new_element(
            Vec3::zero(),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            oxygen.atomic_number,
            ParticleSegmentType::Gas,
            oxygen.prime_dimension() as f32,
            oxygen.reactivity_score(),
            oxygen.magnetism(config.element_prime_alpha),
            oxygen.gravitation(config.element_prime_beta),
            oxygen.light_signature(config.element_light_energy),
            oxygen.mass_dimension(),
        );
        gas.gas_density = 1.0;
        gas.gas_temperature = 1.0;
        sim.bodies.push(gas);

        sim.process_gas_ignition();

        assert_eq!(sim.bodies[0].segment_type, ParticleSegmentType::Stellar);
        assert_eq!(sim.bodies[0].element_atomic_number, 8);
    }

    #[test]
    fn scientific_gravity_changes_effective_gravitational_mass() {
        let mut config = InformationsConfig::default();
        config.enable_scientific_element_gravity = true;
        config.element_gravity_mass_weight = 1.0;
        config.element_gravity_electronegativity_weight = 0.4;
        config.element_gravity_radius_weight = 0.3;
        config.element_gravity_reactivity_weight = 0.2;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();

        let oxygen = elements::element_by_atomic_number(8).unwrap();
        let body = Body::new_element(
            Vec3::zero(),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            oxygen.atomic_number,
            ParticleSegmentType::Orbital,
            oxygen.prime_dimension() as f32,
            oxygen.reactivity_score(),
            oxygen.magnetism(config.element_prime_alpha),
            sim.element_gravitation(oxygen),
            oxygen.light_signature(config.element_light_energy),
            oxygen.mass_dimension(),
        );
        sim.bodies.push(body);

        let effective = sim.bodies[0].effective_gravitational_mass();
        assert!(
            effective != 1.0,
            "scientific gravity should produce a non-unity effective mass"
        );
    }

    #[test]
    fn molecular_bonding_forms_sodium_chloride() {
        let mut config = InformationsConfig::default();
        config.enable_molecular_bonding = true;
        config.molecule_bond_strength_factor = 1.0;
        config.molecule_break_energy_threshold = 10.0;
        config.molecule_max_bodies = 2;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.molecules.clear();
        sim.pending_bonds.clear();

        let sodium = elements::element_by_atomic_number(11).unwrap();
        let chlorine = elements::element_by_atomic_number(17).unwrap();

        let na = Body::new_element(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            sodium.atomic_number,
            ParticleSegmentType::Orbital,
            sodium.prime_dimension() as f32,
            sodium.reactivity_score(),
            sodium.magnetism(config.element_prime_alpha),
            sim.element_gravitation(sodium),
            sodium.light_signature(config.element_light_energy),
            sodium.mass_dimension(),
        );
        let cl = Body::new_element(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            chlorine.atomic_number,
            ParticleSegmentType::Orbital,
            chlorine.prime_dimension() as f32,
            chlorine.reactivity_score(),
            chlorine.magnetism(config.element_prime_alpha),
            sim.element_gravitation(chlorine),
            chlorine.light_signature(config.element_light_energy),
            chlorine.mass_dimension(),
        );
        sim.bodies.push(na);
        sim.bodies.push(cl);

        sim.collide();
        sim.process_pending_bonds();
        sim.update_molecule_metadata();

        assert_eq!(sim.molecules.len(), 1, "Na and Cl should form one molecule");
        assert_eq!(sim.molecules[0].body_indices.len(), 2);
        assert_eq!(sim.molecules[0].formula, "NaCl");
        assert!(sim.bodies.iter().all(|b| b.molecule_id.is_some()));
    }

    #[test]
    fn molecular_bonding_does_not_form_for_noble_gas() {
        let mut config = InformationsConfig::default();
        config.enable_molecular_bonding = true;
        config.molecule_bond_strength_factor = 1.0;
        config.molecule_break_energy_threshold = 10.0;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.molecules.clear();
        sim.pending_bonds.clear();

        let helium = elements::element_by_atomic_number(2).unwrap();
        let neon = elements::element_by_atomic_number(10).unwrap();

        let he = Body::new_element(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            helium.atomic_number,
            ParticleSegmentType::Orbital,
            helium.prime_dimension() as f32,
            helium.reactivity_score(),
            helium.magnetism(config.element_prime_alpha),
            sim.element_gravitation(helium),
            helium.light_signature(config.element_light_energy),
            helium.mass_dimension(),
        );
        let ne = Body::new_element(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            neon.atomic_number,
            ParticleSegmentType::Orbital,
            neon.prime_dimension() as f32,
            neon.reactivity_score(),
            neon.magnetism(config.element_prime_alpha),
            sim.element_gravitation(neon),
            neon.light_signature(config.element_light_energy),
            neon.mass_dimension(),
        );
        sim.bodies.push(he);
        sim.bodies.push(ne);

        sim.collide();
        sim.process_pending_bonds();

        assert!(
            sim.molecules.is_empty(),
            "noble gases should not form molecules"
        );
    }

    #[test]
    fn molecule_preserves_total_mass() {
        let mut config = InformationsConfig::default();
        config.enable_molecular_bonding = true;
        config.molecule_bond_strength_factor = 1.0;
        config.molecule_break_energy_threshold = 10.0;
        config.molecule_max_bodies = 2;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.molecules.clear();
        sim.pending_bonds.clear();

        let hydrogen = elements::element_by_atomic_number(1).unwrap();
        let h1 = Body::new_element(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            hydrogen.atomic_number,
            ParticleSegmentType::Orbital,
            hydrogen.prime_dimension() as f32,
            hydrogen.reactivity_score(),
            hydrogen.magnetism(config.element_prime_alpha),
            sim.element_gravitation(hydrogen),
            hydrogen.light_signature(config.element_light_energy),
            hydrogen.mass_dimension(),
        );
        let h2 = Body::new_element(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            hydrogen.atomic_number,
            ParticleSegmentType::Orbital,
            hydrogen.prime_dimension() as f32,
            hydrogen.reactivity_score(),
            hydrogen.magnetism(config.element_prime_alpha),
            sim.element_gravitation(hydrogen),
            hydrogen.light_signature(config.element_light_energy),
            hydrogen.mass_dimension(),
        );
        sim.bodies.push(h1);
        sim.bodies.push(h2);

        sim.collide();
        sim.process_pending_bonds();
        sim.update_molecule_metadata();

        assert!(!sim.molecules.is_empty(), "a molecule should have formed");
        let body_mass: f32 = sim.bodies.iter().map(|b| b.mass).sum();
        assert!(
            (sim.molecules[0].mass - body_mass).abs() < 1e-4,
            "molecule mass must equal sum of body masses"
        );
    }

    #[test]
    fn molecular_bonding_forms_diatomic_hydrogen() {
        let mut config = InformationsConfig::default();
        config.enable_molecular_bonding = true;
        config.molecule_bond_strength_factor = 1.0;
        config.molecule_break_energy_threshold = 10.0;
        config.molecule_max_bodies = 2;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.molecules.clear();
        sim.pending_bonds.clear();

        let hydrogen = elements::element_by_atomic_number(1).unwrap();
        let h1 = Body::new_element(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            hydrogen.atomic_number,
            ParticleSegmentType::Orbital,
            hydrogen.prime_dimension() as f32,
            hydrogen.reactivity_score(),
            hydrogen.magnetism(config.element_prime_alpha),
            sim.element_gravitation(hydrogen),
            hydrogen.light_signature(config.element_light_energy),
            hydrogen.mass_dimension(),
        );
        let h2 = Body::new_element(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            hydrogen.atomic_number,
            ParticleSegmentType::Orbital,
            hydrogen.prime_dimension() as f32,
            hydrogen.reactivity_score(),
            hydrogen.magnetism(config.element_prime_alpha),
            sim.element_gravitation(hydrogen),
            hydrogen.light_signature(config.element_light_energy),
            hydrogen.mass_dimension(),
        );
        sim.bodies.push(h1);
        sim.bodies.push(h2);

        sim.collide();
        sim.process_pending_bonds();
        sim.update_molecule_metadata();

        assert_eq!(sim.molecules.len(), 1, "two hydrogen atoms should form H2");
        assert_eq!(sim.molecules[0].body_indices.len(), 2);
        assert_eq!(sim.molecules[0].formula, "H2");
    }

    #[test]
    fn non_diatomic_same_element_still_merges_with_molecular_bonding() {
        let mut config = InformationsConfig::default();
        config.enable_molecular_bonding = true;
        config.molecule_bond_strength_factor = 1.0;

        let mut sim = Simulation::new(&config);
        sim.bodies.clear();
        sim.pairs.clear();
        sim.merge_contact_frames.clear();
        sim.molecules.clear();
        sim.pending_bonds.clear();

        let iron = elements::element_by_atomic_number(26).unwrap();
        let fe1 = Body::new_element(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            iron.atomic_number,
            ParticleSegmentType::Orbital,
            iron.prime_dimension() as f32,
            iron.reactivity_score(),
            iron.magnetism(config.element_prime_alpha),
            sim.element_gravitation(iron),
            iron.light_signature(config.element_light_energy),
            iron.mass_dimension(),
        );
        let fe2 = Body::new_element(
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::zero(),
            1.0,
            0.5,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            iron.atomic_number,
            ParticleSegmentType::Orbital,
            iron.prime_dimension() as f32,
            iron.reactivity_score(),
            iron.magnetism(config.element_prime_alpha),
            sim.element_gravitation(iron),
            iron.light_signature(config.element_light_energy),
            iron.mass_dimension(),
        );
        sim.bodies.push(fe1);
        sim.bodies.push(fe2);

        // Run enough collision steps to exceed the merge contact threshold.
        for _ in 0..10 {
            sim.collide();
            sim.process_pending_bonds();
        }

        assert!(
            sim.molecules.is_empty(),
            "iron should not form a molecule"
        );
        assert!(
            sim.bodies.len() < 2 || sim.merge_contact_frames.values().any(|&c| c > 0),
            "iron pair should accumulate merge contact frames"
        );
    }

    #[test]
    fn equilibrium_mode_with_molecular_bonding_steps_without_crash() {
        let mut config = InformationsConfig::default();
        config.enable_disk_equilibrium_mode = true;
        config.enable_molecular_bonding = true;
        config.n = 64;
        config.particle_budget = 128;
        config.galaxy_count = 2;
        config.gas_enabled = false;

        let mut sim = Simulation::new(&config);
        for _ in 0..8 {
            sim.step();
        }
        assert!(sim.frame > 0);
        assert!(!sim.bodies.is_empty());
    }
}
