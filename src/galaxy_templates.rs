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
    /// Probability (0–1) that one new accretion particle is spawned near a
    /// detected galaxy center on any given simulation step.
    pub accretion_spawn_rate: f32,
    /// Inner radius of the outer ring spawn zone, measured from the galaxy center.
    pub outer_ring_spawn_zone_inner_radius: f32,
    /// Outer radius of the outer ring spawn zone, measured from the galaxy center.
    pub outer_ring_spawn_zone_outer_radius: f32,
}

impl GalaxyTemplate {
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
        ));

        let (mass_min, mass_max) = self.particle_mass_range;

        // Pass 1: place disc particles.  body.vel is temporarily set to the unit
        // tangent direction; the actual orbital velocity is assigned in pass 2.
        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let thickness = self.outer_radius * 0.002;
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
            enclosed_mass += body.mass;
            let offset = body.pos - center;
            if offset == Vec3::zero() {
                continue;
            }
            let orbital_speed = (enclosed_mass / offset.xy().mag()).sqrt();
            // body.vel currently holds the unit tangent direction assigned above.
            body.vel = velocity + body.vel * orbital_speed;
            body.angular_speed += orbital_speed * 0.025 * self.spin_speed_multiplier;
        }

        bodies
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
        ));

        let (mass_min, mass_max) = self.particle_mass_range;

        while bodies.len() <= self.n {
            let a = fastrand::f32() * std::f32::consts::TAU;
            let (sin, cos) = a.sin_cos();
            let t = self.inner_radius / self.outer_radius;
            let r = fastrand::f32() * (1.0 - t * t) + t * t;
            let thickness = self.outer_radius * 0.002;
            let depth = (fastrand::f32() - 0.5) * thickness;
            let offset = (u * cos + v * sin) * self.outer_radius * r.sqrt() + rotation_axis * depth;
            let tangent_dir = if clockwise {
                -rotation_axis.cross(offset).normalized()
            } else {
                rotation_axis.cross(offset).normalized()
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
            ));
        }

        bodies.sort_by(|a, b| {
            let da = (a.pos - center).mag_sq();
            let db = (b.pos - center).mag_sq();
            da.total_cmp(&db)
        });

        let mut enclosed_mass = 0.0_f32;
        for body in &mut bodies {
            enclosed_mass += body.mass;
            let offset = body.pos - center;
            if offset == Vec3::zero() {
                continue;
            }
            let orbital_speed = (enclosed_mass / offset.mag()).sqrt();
            body.vel = velocity + body.vel * orbital_speed;
            body.angular_speed += orbital_speed
                * 0.025
                * if clockwise { 1.0 } else { -1.0 }
                * self.spin_speed_multiplier;
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
}
