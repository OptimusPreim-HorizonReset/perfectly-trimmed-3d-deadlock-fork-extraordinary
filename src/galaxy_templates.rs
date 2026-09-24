use crate::{body::Body, config::InformationsConfig};
use ultraviolet::{Vec2, Vec3};

/// Configuration for a galaxy template, controlling initial particle distribution,
/// mass ranges, and accretion spawn behaviour.
pub struct GalaxyTemplate {
    /// Number of disc particles to generate (excluding the central body).
    pub n: usize,
    /// Radius of the central mass (galactic nucleus / black hole).
    pub inner_radius: f32,
    /// Outer radius of the galactic disc.
    pub outer_radius: f32,
    /// Mass of the central body.
    pub central_mass: f32,
    /// Per-particle mass range `(min, max)` sampled uniformly at random.
    pub particle_mass_range: (f32, f32),
    /// Global multiplier applied to all initial particle spin speeds.
    pub spin_speed_multiplier: f32,
    /// Enables merge-triggered accretion spawning when `> 0`.
    /// In the legacy paired galaxy disk mode, a successful same-galaxy merge will
    /// spawn one new accretion particle in the host galaxy's outer disk.
    pub accretion_spawn_rate: f32,
    /// Inner radius of the outer ring spawn zone, measured from the galaxy center.
    pub outer_ring_spawn_zone_inner_radius: f32,
    /// Outer radius of the outer ring spawn zone, measured from the galaxy center.
    pub outer_ring_spawn_zone_outer_radius: f32,
}

impl GalaxyTemplate {
    pub(crate) fn softened_circular_speed(enclosed_mass: f32, radius: f32, epsilon: f32) -> f32 {
        if enclosed_mass <= 0.0 || radius <= 0.0 {
            return 0.0;
        }
        let softened_distance_sq = radius * radius + epsilon * epsilon;
        (enclosed_mass * radius * radius / softened_distance_sq.powf(1.5)).sqrt()
    }

    pub(crate) fn disk_drift_factor(normalized_radius: f32, config: &InformationsConfig) -> f32 {
        if !config.enable_disk_equilibrium_mode {
            return 1.0;
        }

        let radius = normalized_radius.clamp(0.0, 1.0);
        let q_scale = (config.disk_equilibrium_toomre_q / 1.3).clamp(0.75, 2.0);
        let surface_slope = 0.6 + 0.8 * radius;
        let dispersion_profile = 0.18 + 0.22 * (1.0 - radius);
        let raw_factor = 1.0
            - config.disk_equilibrium_asymmetric_drift_strength
                * q_scale
                * surface_slope
                * dispersion_profile
                * 0.45;
        raw_factor.clamp(0.5, 1.0)
    }

    /// Dense spiral galaxy: large central black hole, variable-mass disc particles.
    pub fn spiral(n: usize) -> Self {
        let outer_radius = (n as f32).sqrt() * 5.0;
        let inner_radius = 25.0;
        let spawn_inner =
            ((2.0 * outer_radius * outer_radius + inner_radius * inner_radius) / 3.0).sqrt();
        Self {
            n,
            inner_radius,
            outer_radius,
            central_mass: 1e6,
            particle_mass_range: (0.5, 2.0),
            spin_speed_multiplier: 1.0,
            accretion_spawn_rate: 0.1,
            outer_ring_spawn_zone_inner_radius: spawn_inner,
            outer_ring_spawn_zone_outer_radius: outer_radius,
        }
    }

    /// Diffuse elliptical galaxy: lighter central mass, heavier disc particles.
    #[allow(dead_code)]
    pub fn elliptical(n: usize) -> Self {
        let outer_radius = (n as f32).sqrt() * 7.0;
        let inner_radius = 15.0;
        let spawn_inner =
            ((2.0 * outer_radius * outer_radius + inner_radius * inner_radius) / 3.0).sqrt();
        Self {
            n,
            inner_radius,
            outer_radius,
            central_mass: 5e5,
            particle_mass_range: (1.0, 3.0),
            spin_speed_multiplier: 1.0,
            accretion_spawn_rate: 0.1,
            outer_ring_spawn_zone_inner_radius: spawn_inner,
            outer_ring_spawn_zone_outer_radius: outer_radius,
        }
    }

    pub fn from_config(config: &InformationsConfig) -> Self {
        let outer_radius = config.outer_radius;
        // Vereinfachte Priority: Nutze nur die Ratios aus der Config
        let outer_ring_spawn_zone_inner_radius =
            config.outer_ring_spawn_zone_inner_ratio * outer_radius;
        let outer_ring_spawn_zone_outer_radius =
            config.outer_ring_spawn_zone_outer_ratio * outer_radius;

        Self {
            n: config.n,
            inner_radius: config.inner_radius,
            outer_radius,
            central_mass: config.central_mass,
            particle_mass_range: config.particle_mass_range,
            spin_speed_multiplier: config.spin_speed_multiplier,
            accretion_spawn_rate: config.accretion_spawn_rate,
            outer_ring_spawn_zone_inner_radius,
            outer_ring_spawn_zone_outer_radius,
        }
    }

    /// Generates bodies from this template placed at `center` with bulk velocity `velocity`.
    ///
    /// Each particle receives a Keplerian orbital velocity based on the enclosed mass
    /// interior to its orbit, ensuring stable circular orbits around the central body.
    pub fn generate(&self, center: Vec2, velocity: Vec2) -> Vec<Body> {
        let center = Vec3::new(center.x, center.y, 0.0);
        let velocity = Vec3::new(velocity.x, velocity.y, 0.0);
        let mut bodies: Vec<Body> = Vec::with_capacity(self.n + 1);

        // Central massive body (black hole / galactic nucleus)
        bodies.push(Body::new(
            center,
            velocity,
            self.central_mass,
            self.inner_radius * 0.01,
            0.35 * self.spin_speed_multiplier,
            Vec3::new(0.0, 0.0, 1.0),
            crate::body::ParticleSegmentType::Core,
        ));

        let (mass_min, mass_max) = self.particle_mass_range;

        // Pass 1: place disc particles.  body.vel is temporarily set to the unit
        // tangent direction; the actual orbital velocity is assigned in pass 2.
        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let thickness = self.outer_radius * 0.01;
            let z = (fastrand::f32() - 0.5) * thickness;
            let offset =
                Vec3::new(cos, sin, 0.0) * self.outer_radius * r.sqrt() + Vec3::new(0.0, 0.0, z);
            // Unit tangent for a clockwise orbit.
            let tangent = Vec3::new(sin, -cos, 0.0);
            // Realistic mass distribution: heavier particles near the center,
            // lighter ones further out, with some random scatter around the
            // radius-dependent baseline.
            let radial_falloff = 1.0 - r.sqrt();
            let base_mass = mass_min + radial_falloff * (mass_max - mass_min);
            let mass = (base_mass + (fastrand::f32() - 0.5) * (mass_max - mass_min) * 0.3)
                .clamp(mass_min, mass_max);
            let radius = mass.cbrt();
            bodies.push(Body::new(
                center + offset,
                tangent,
                mass,
                radius,
                (0.4 + fastrand::f32() * 0.5) * self.spin_speed_multiplier,
                Vec3::new(0.0, 0.0, 1.0),
                crate::body::ParticleSegmentType::Orbital,
            ));
        }

        // Sort by distance from center so the enclosed-mass accumulation is correct.
        bodies.sort_by(|a, b| {
            (a.pos.xy() - center.xy())
                .mag_sq()
                .total_cmp(&(b.pos.xy() - center.xy()).mag_sq())
        });

        // Pass 2: assign Keplerian orbital velocities.
        let mut enclosed_mass = 0.0_f32;
        for body in &mut bodies {
            let offset = body.pos - center;
            if offset == Vec3::zero() {
                enclosed_mass += body.mass;
                continue;
            }
            let orbital_speed =
                Self::softened_circular_speed(enclosed_mass, offset.xy().mag(), 0.0);
            // body.vel currently holds the unit tangent direction assigned above.
            body.vel = velocity + body.vel * orbital_speed;
            body.angular_speed += orbital_speed * 0.025 * self.spin_speed_multiplier;
            enclosed_mass += body.mass;
        }

        bodies
    }

    pub fn generate_with_config(
        &self,
        center: Vec2,
        velocity: Vec2,
        config: &InformationsConfig,
    ) -> Vec<Body> {
        if config.enable_disk_equilibrium_mode {
            self.generate_disk_equilibrium(center, velocity, config)
        } else {
            self.generate(center, velocity)
        }
    }

    fn generate_disk_equilibrium(
        &self,
        center: Vec2,
        velocity: Vec2,
        config: &InformationsConfig,
    ) -> Vec<Body> {
        let center = Vec3::new(center.x, center.y, 0.0);
        let velocity = Vec3::new(velocity.x, velocity.y, 0.0);
        let mut bodies: Vec<Body> = Vec::with_capacity(self.n + 1);

        bodies.push(Body::new(
            center,
            velocity,
            self.central_mass,
            self.inner_radius * 0.01,
            0.35 * self.spin_speed_multiplier,
            Vec3::new(0.0, 0.0, 1.0),
            crate::body::ParticleSegmentType::Core,
        ));

        let (mass_min, mass_max) = self.particle_mass_range;
        let scale_height =
            self.outer_radius * config.disk_equilibrium_scale_height_factor.max(0.001);

        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let z = self.sample_sech2_height(scale_height);
            let offset =
                Vec3::new(cos, sin, 0.0) * self.outer_radius * r.sqrt() + Vec3::new(0.0, 0.0, z);
            let tangent = Vec3::new(sin, -cos, 0.0);
            let radial_falloff = 1.0 - r.sqrt();
            let base_mass = mass_min + radial_falloff * (mass_max - mass_min);
            let mass = (base_mass + (fastrand::f32() - 0.5) * (mass_max - mass_min) * 0.3)
                .clamp(mass_min, mass_max);
            let radius = mass.cbrt();
            bodies.push(Body::new(
                center + offset,
                tangent,
                mass,
                radius,
                (0.4 + fastrand::f32() * 0.5) * self.spin_speed_multiplier,
                Vec3::new(0.0, 0.0, 1.0),
                crate::body::ParticleSegmentType::Orbital,
            ));
        }

        bodies.sort_by(|a, b| {
            (a.pos.xy() - center.xy())
                .mag_sq()
                .total_cmp(&(b.pos.xy() - center.xy()).mag_sq())
        });

        let mut enclosed_mass = 0.0_f32;
        for body in &mut bodies {
            let offset = body.pos - center;
            if offset == Vec3::zero() {
                enclosed_mass += body.mass;
                continue;
            }
            let radius = offset.xy().mag();
            let orbital_speed =
                Self::softened_circular_speed(enclosed_mass, radius, config.effective_epsilon());
            let normalized_radius = (radius / self.outer_radius).clamp(0.0, 1.0);
            let drift_factor = Self::disk_drift_factor(normalized_radius, config);
            body.vel = velocity + body.vel * orbital_speed * drift_factor;
            body.angular_speed += orbital_speed * 0.025 * self.spin_speed_multiplier * drift_factor;
            enclosed_mass += body.mass;
        }

        bodies
    }

    pub fn generate_inclined_with_config(
        &self,
        center: Vec2,
        velocity: Vec2,
        rotation_axis: Vec3,
        clockwise: bool,
        config: &InformationsConfig,
    ) -> Vec<Body> {
        if config.enable_disk_equilibrium_mode {
            self.generate_inclined_disk_equilibrium(
                center,
                velocity,
                rotation_axis,
                clockwise,
                config,
            )
        } else {
            self.generate_inclined(center, velocity, rotation_axis, clockwise)
        }
    }

    fn generate_inclined_disk_equilibrium(
        &self,
        center: Vec2,
        velocity: Vec2,
        rotation_axis: Vec3,
        clockwise: bool,
        config: &InformationsConfig,
    ) -> Vec<Body> {
        let center = Vec3::new(center.x, center.y, 0.0);
        let velocity = Vec3::new(velocity.x, velocity.y, 0.0);
        let rotation_axis = if rotation_axis == Vec3::zero() {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            rotation_axis.normalized()
        };
        let (u, v) = self.plane_basis(rotation_axis);
        let mut bodies: Vec<Body> = Vec::with_capacity(self.n + 1);

        bodies.push(Body::new(
            center,
            velocity,
            self.central_mass,
            self.inner_radius * 0.01,
            if clockwise { 0.35 } else { -0.35 } * self.spin_speed_multiplier,
            rotation_axis,
            crate::body::ParticleSegmentType::Core,
        ));

        let (mass_min, mass_max) = self.particle_mass_range;
        let scale_height =
            self.outer_radius * config.disk_equilibrium_scale_height_factor.max(0.001);

        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let radius = self.outer_radius * r.sqrt();
            let z = self.sample_sech2_height(scale_height);
            let in_plane = (u * cos + v * sin) * radius;
            let offset = in_plane + rotation_axis * z;
            let raw_tangent = if clockwise {
                -(rotation_axis.cross(in_plane))
            } else {
                rotation_axis.cross(in_plane)
            };
            let tangent = if raw_tangent.mag_sq() > 1e-8 {
                raw_tangent.normalized()
            } else {
                Vec3::new(-sin, cos, 0.0)
            };
            let radial_falloff = 1.0 - r.sqrt();
            let base_mass = mass_min + radial_falloff * (mass_max - mass_min);
            let mass = (base_mass + (fastrand::f32() - 0.5) * (mass_max - mass_min) * 0.3)
                .clamp(mass_min, mass_max);
            let body_radius = mass.cbrt();
            bodies.push(Body::new(
                center + offset,
                tangent,
                mass,
                body_radius,
                (0.4 + fastrand::f32() * 0.5) * self.spin_speed_multiplier,
                rotation_axis,
                crate::body::ParticleSegmentType::Orbital,
            ));
        }

        bodies.sort_by(|a, b| {
            let a_offset = a.pos - center;
            let b_offset = b.pos - center;
            let a_in_plane = a_offset - rotation_axis * a_offset.dot(rotation_axis);
            let b_in_plane = b_offset - rotation_axis * b_offset.dot(rotation_axis);
            a_in_plane.mag_sq().total_cmp(&b_in_plane.mag_sq())
        });

        let mut enclosed_mass = 0.0_f32;
        for body in &mut bodies {
            let offset = body.pos - center;
            let radius = (offset - rotation_axis * offset.dot(rotation_axis)).mag();
            if radius <= 0.0 {
                enclosed_mass += body.mass;
                continue;
            }
            let orbital_speed =
                Self::softened_circular_speed(enclosed_mass, radius, config.effective_epsilon());
            let normalized_radius = (radius / self.outer_radius).clamp(0.0, 1.0);
            let drift_factor = Self::disk_drift_factor(normalized_radius, config);
            body.vel = velocity + body.vel * orbital_speed * drift_factor;
            body.angular_speed += orbital_speed * 0.025 * self.spin_speed_multiplier * drift_factor;
            enclosed_mass += body.mass;
        }

        bodies
    }

    pub(crate) fn sample_sech2_height(&self, scale_height: f32) -> f32 {
        let u = fastrand::f32();
        let x = (2.0 * u - 1.0).clamp(-0.999, 0.999);
        let z = 0.5 * ((1.0 + x) / (1.0 - x)).ln();
        z * scale_height
    }

    /// Generates an inclined disk with a random rotation axis and direction.
    pub fn generate_inclined(
        &self,
        center: Vec2,
        velocity: Vec2,
        rotation_axis: Vec3,
        clockwise: bool,
    ) -> Vec<Body> {
        let center = Vec3::new(center.x, center.y, 0.0);
        let velocity = Vec3::new(velocity.x, velocity.y, 0.0);
        let rotation_axis = if rotation_axis == Vec3::zero() {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            rotation_axis.normalized()
        };

        let (u, v) = self.plane_basis(rotation_axis);
        let mut bodies: Vec<Body> = Vec::with_capacity(self.n + 1);

        bodies.push(Body::new(
            center,
            velocity,
            self.central_mass,
            self.inner_radius * 0.01,
            if clockwise { 0.35 } else { -0.35 } * self.spin_speed_multiplier,
            rotation_axis,
            crate::body::ParticleSegmentType::Core,
        ));

        let (mass_min, mass_max) = self.particle_mass_range;

        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let thickness = self.outer_radius * 0.01;
            let depth = (fastrand::f32() - 0.5) * thickness;
            let in_plane_offset = (u * cos + v * sin) * self.outer_radius * r.sqrt();
            let offset = in_plane_offset + rotation_axis * depth;
            let tangent_dir = if clockwise {
                -(rotation_axis.cross(in_plane_offset).normalized())
            } else {
                rotation_axis.cross(in_plane_offset).normalized()
            };
            // Realistic mass distribution: heavier particles near the center,
            // lighter ones further out, with some random scatter around the
            // radius-dependent baseline.
            let radial_falloff = 1.0 - r.sqrt();
            let base_mass = mass_min + radial_falloff * (mass_max - mass_min);
            let mass = (base_mass + (fastrand::f32() - 0.5) * (mass_max - mass_min) * 0.3)
                .clamp(mass_min, mass_max);
            let radius = mass.cbrt();
            bodies.push(Body::new(
                center + offset,
                tangent_dir,
                mass,
                radius,
                (0.4 + fastrand::f32() * 0.5) * self.spin_speed_multiplier,
                rotation_axis,
                crate::body::ParticleSegmentType::Orbital,
            ));
        }

        bodies.sort_by(|a, b| {
            let a_offset = a.pos - center;
            let b_offset = b.pos - center;
            let a_in_plane = a_offset - rotation_axis * a_offset.dot(rotation_axis);
            let b_in_plane = b_offset - rotation_axis * b_offset.dot(rotation_axis);
            a_in_plane.mag_sq().total_cmp(&b_in_plane.mag_sq())
        });

        let mut enclosed_mass = 0.0_f32;
        for body in &mut bodies {
            let offset = body.pos - center;
            let radius = (offset - rotation_axis * offset.dot(rotation_axis)).mag();
            if radius <= 0.0 {
                enclosed_mass += body.mass;
                continue;
            }
            let orbital_speed = Self::softened_circular_speed(enclosed_mass, radius, 0.0);
            body.vel = velocity + body.vel * orbital_speed;
            body.angular_speed += orbital_speed
                * 0.025
                * if clockwise { 1.0 } else { -1.0 }
                * self.spin_speed_multiplier;
            enclosed_mass += body.mass;
        }

        bodies
    }

    /// Generates a lightweight gas disk aligned with the galaxy plane.
    pub fn generate_gas_disk(
        &self,
        center: Vec2,
        rotation_axis: Vec3,
        clockwise: bool,
        count: usize,
    ) -> Vec<Body> {
        let center = Vec3::new(center.x, center.y, 0.0);
        let rotation_axis = if rotation_axis == Vec3::zero() {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            rotation_axis.normalized()
        };

        let (u, v) = self.plane_basis(rotation_axis);
        let mut bodies: Vec<Body> = Vec::with_capacity(count);
        let gas_mass_scale = (self.particle_mass_range.0 + self.particle_mass_range.1) * 0.08;
        let mass_min = (self.particle_mass_range.0 * 0.08).max(1e-6);
        let mass_max = (self.particle_mass_range.1 * 0.18).max(mass_min * 1.5);
        let _radius_scale = 0.28 * self.outer_radius.max(1.0);

        for i in 0..count {
            let fraction = if count > 0 {
                (i as f32 + 0.5) / count as f32
            } else {
                0.5
            };
            let angle = fraction * std::f32::consts::TAU + fastrand::f32() * 0.35;
            let cos = angle.cos();
            let sin = angle.sin();
            let radius = self.outer_radius * (0.18 + 0.60 * fraction.sqrt()).clamp(0.12, 1.0);
            let thickness = self.outer_radius * 0.14;
            let depth = (fastrand::f32() - 0.5) * thickness;
            let in_plane = (u * cos + v * sin) * radius;
            let position = center + in_plane + rotation_axis * depth;
            let raw_tangent = if clockwise {
                -(rotation_axis.cross(in_plane))
            } else {
                rotation_axis.cross(in_plane)
            };
            let tangent = if raw_tangent.mag_sq() > 1e-8 {
                raw_tangent.normalized()
            } else {
                Vec3::new(-sin, cos, 0.0)
            };
            let orbital_speed = ((self.central_mass / radius.max(1.0)).max(0.001)).sqrt()
                * self.spin_speed_multiplier
                * 0.45
                * (0.8 + fraction * 0.4);
            let velocity =
                tangent * orbital_speed + rotation_axis * ((fastrand::f32() - 0.5) * 0.035);
            let mass = (gas_mass_scale * (0.6 + 0.8 * (1.0 - fraction))
                + (fastrand::f32() - 0.5) * (mass_max - mass_min) * 0.25)
                .clamp(mass_min, mass_max);
            let body_radius = (mass.cbrt() * 0.5 + 0.02) * self.spin_speed_multiplier.max(0.3);

            let mut body = Body::new(
                position,
                velocity,
                mass,
                body_radius,
                orbital_speed * 0.04 * self.spin_speed_multiplier,
                rotation_axis,
                crate::body::ParticleSegmentType::Gas,
            );
            body.gas_temperature = 1.0;
            body.gas_smoothing_radius = self.outer_radius * 0.06;
            bodies.push(body);
        }

        bodies
    }

    pub fn generate_gas_volume(&self, center: Vec3, half_side: f32, count: usize) -> Vec<Body> {
        let mut bodies = Vec::with_capacity(count);
        let mass_min = (self.particle_mass_range.0 * 0.04).max(1e-8);
        let mass_max = (self.particle_mass_range.1 * 0.12).max(mass_min * 1.5);
        let radius_scale = self.outer_radius.max(1.0) * 0.03;

        for _ in 0..count {
            let dx = (fastrand::f32() * 2.0 - 1.0) * half_side;
            let dy = (fastrand::f32() * 2.0 - 1.0) * half_side;
            let dz = (fastrand::f32() * 2.0 - 1.0) * half_side;
            let position = center + Vec3::new(dx, dy, dz);
            let raw_dir = Vec3::new(dx, dy, dz);
            let direction = if raw_dir.mag_sq() <= 1e-8 {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                raw_dir.normalized()
            };
            let mut tangent = direction.cross(Vec3::new(0.0, 0.0, 1.0));
            if tangent.mag_sq() <= 1e-8 {
                tangent = Vec3::new(1.0, 0.0, 0.0);
            }
            let tangent = tangent.normalized();
            let distance = Vec3::new(dx, dy, dz).mag();
            let speed = (0.02 + fastrand::f32() * 0.035)
                * (1.2 - (distance / half_side.max(1.0)).clamp(0.0, 1.0));
            let velocity = tangent * speed + direction * ((fastrand::f32() - 0.5) * 0.008);
            let mass =
                (mass_min + (fastrand::f32() * (mass_max - mass_min))).clamp(mass_min, mass_max);
            let body_radius = (mass.cbrt() * radius_scale).max(0.01);
            let mut body = Body::new(
                position,
                velocity,
                mass,
                body_radius,
                speed * 0.08,
                Vec3::new(0.0, 0.0, 1.0),
                crate::body::ParticleSegmentType::Gas,
            );
            body.gas_temperature = 1.0;
            body.gas_smoothing_radius = (half_side * 0.08).max(0.5);
            bodies.push(body);
        }
        bodies
    }

    fn plane_basis(&self, axis: Vec3) -> (Vec3, Vec3) {
        let reference = if axis.abs().x < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let u = axis.cross(reference).normalized();
        let v = axis.cross(u).normalized();
        (u, v)
    }

    fn random_direction() -> Vec3 {
        let theta = fastrand::f32() * std::f32::consts::PI;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let x = theta.sin() * phi.cos();
        let y = theta.sin() * phi.sin();
        let z = theta.cos();
        Vec3::new(x, y, z).normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_inclined_respects_rotation_axis() {
        let template = GalaxyTemplate::spiral(256);
        let axis = Vec3::new(1.0, 0.5, 1.2).normalized();
        let bodies = template.generate_inclined(Vec2::zero(), Vec2::zero(), axis, true);
        assert_eq!(bodies.len(), 257);

        let center = bodies[0];
        assert!(
            (center.rotation_axis - axis).mag() < 1e-6,
            "center rotation axis must preserve the galaxy inclination"
        );

        for body in bodies.iter().skip(1) {
            let offset = body.pos - center.pos;
            let distance_from_plane = offset.dot(axis).abs();
            assert!(
                distance_from_plane <= template.outer_radius * 0.03 + 1e-3,
                "spawned particle should remain close to the inclined disk plane"
            );
        }
    }

    #[test]
    fn generate_gas_volume_remains_within_cube() {
        let template = GalaxyTemplate::spiral(128);
        let center = Vec3::zero();
        let half_side = 45.0;
        let bodies = template.generate_gas_volume(center, half_side, 30);
        assert_eq!(bodies.len(), 30);
        for body in bodies {
            let offset = body.pos - center;
            assert!(
                offset.x.abs() <= half_side + 1e-3,
                "gas X coordinate exceeded volume half-side"
            );
            assert!(
                offset.y.abs() <= half_side + 1e-3,
                "gas Y coordinate exceeded volume half-side"
            );
            assert!(
                offset.z.abs() <= half_side + 1e-3,
                "gas Z coordinate exceeded volume half-side"
            );
            assert!(body.is_gas(), "volume particle must be gas");
        }
    }
}
