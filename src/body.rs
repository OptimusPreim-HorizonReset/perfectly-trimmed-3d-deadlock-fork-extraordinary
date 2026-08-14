use ultraviolet::Vec3;

#[derive(Clone, Copy)]
pub struct Body {
    pub pos: Vec3,
    pub vel: Vec3,
    pub acc: Vec3,
    pub prev_acc: Vec3,
    pub jerk: Vec3,
    pub mass: f32,
    pub base_radius: f32,
    pub equatorial_radius: f32,
    pub polar_radius: f32,
    pub rotation_axis: Vec3,
    pub angular_speed: f32,
    pub spin_angle: f32,
    pub theta_i: f32,
    pub multipole_order: u8,
    pub density: f32,
    pub energy_error: f32,
    pub local_dt: f32,
    pub element_atomic_number: u8,
    /// Analogwerte für die Prime/Massendimension der galaktischen Atom-Visualisierung.
    pub element_prime_energy: f32,
    /// Unterschiedliche Dimensionen im Massenspektrum, hier als Prime-Lücke kodiert.
    pub element_reactivity: f32,
    /// Analoge Magnetismusstärke basierend auf der Dimensionssignatur.
    pub element_magnetism: f32,
    /// Analoge Gravitationstuning-Variable, nicht als neues Naturgesetz zu verstehen.
    pub element_gravitation: f32,
    /// Licht-/Emissionsskala in der Analogie.
    pub element_light: f32,
    /// Massendomäne bzw. Frequenzsignatur der primzahlbasierten Dimensionszuordnung.
    pub element_mass_dimension: f32,
}

impl Body {
    fn sanitize_vec(v: Vec3) -> Vec3 {
        Vec3::new(
            if v.x.is_finite() { v.x } else { 0.0 },
            if v.y.is_finite() { v.y } else { 0.0 },
            if v.z.is_finite() { v.z } else { 0.0 },
        )
    }

    pub fn new(
        pos: Vec3,
        vel: Vec3,
        mass: f32,
        radius: f32,
        angular_speed: f32,
        rotation_axis: Vec3,
    ) -> Self {
        let rotation_axis = if rotation_axis == Vec3::zero() {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            rotation_axis.normalized()
        };

        let scaled_radius = radius * 0.25;
        let mut body = Self {
            pos: Self::sanitize_vec(pos),
            vel: Self::sanitize_vec(vel),
            acc: Vec3::zero(),
            prev_acc: Vec3::zero(),
            jerk: Vec3::zero(),
            mass: if mass.is_finite() && mass > 0.0 {
                mass
            } else {
                1.0
            },
            base_radius: if scaled_radius.is_finite() && scaled_radius > 0.0 {
                scaled_radius
            } else {
                1.0
            },
            equatorial_radius: if scaled_radius.is_finite() && scaled_radius > 0.0 {
                scaled_radius
            } else {
                1.0
            },
            polar_radius: if scaled_radius.is_finite() && scaled_radius > 0.0 {
                scaled_radius
            } else {
                1.0
            },
            rotation_axis,
            angular_speed: if angular_speed.is_finite() {
                angular_speed
            } else {
                0.0
            },
            spin_angle: 0.0,
            theta_i: 1.0,
            multipole_order: 1,
            density: 0.0,
            energy_error: 0.0,
            local_dt: 0.0,
            element_atomic_number: 0,
            element_prime_energy: 0.0,
            element_reactivity: 0.0,
            element_magnetism: 0.0,
            element_gravitation: 0.0,
            element_light: 0.0,
            element_mass_dimension: 0.0,
        };
        body.update_shape();
        body
    }

    pub fn new_element(
        pos: Vec3,
        vel: Vec3,
        mass: f32,
        radius: f32,
        angular_speed: f32,
        rotation_axis: Vec3,
        element_atomic_number: u8,
        element_prime_energy: f32,
        element_reactivity: f32,
        element_magnetism: f32,
        element_gravitation: f32,
        element_light: f32,
        element_mass_dimension: f32,
    ) -> Self {
        let mut body = Self::new(pos, vel, mass, radius, angular_speed, rotation_axis);
        body.element_atomic_number = element_atomic_number;
        body.element_prime_energy = element_prime_energy;
        body.element_reactivity = element_reactivity;
        body.element_magnetism = element_magnetism;
        body.element_gravitation = element_gravitation;
        body.element_light = element_light;
        body.element_mass_dimension = element_mass_dimension;
        body
    }

    pub fn update(&mut self, dt: f32) {
        self.spin_angle += self.angular_speed * dt;
        self.vel += self.acc * dt;
        self.pos += self.vel * dt;

        self.spin_angle = if self.spin_angle.is_finite() {
            self.spin_angle
        } else {
            0.0
        };
        self.acc = Self::sanitize_vec(self.acc);
        self.vel = Self::sanitize_vec(self.vel);
        self.pos = Self::sanitize_vec(self.pos);

        self.update_shape();
    }

    pub fn projected_radius(&self) -> f32 {
        let depth_scale = (1.0 - self.pos.z * 0.002).clamp(0.55, 1.4);
        self.equatorial_radius * depth_scale
    }

    pub fn effective_radius(&self) -> f32 {
        (self.equatorial_radius * 2.0 + self.polar_radius) / 3.0
    }

    pub fn effective_gravitational_mass(&self) -> f32 {
        self.mass * (1.0 + self.element_gravitation)
    }

    fn update_shape(&mut self) {
        let spin = self.angular_speed.abs();
        let axis_scale = self.rotation_axis.mag_sq().max(1.0);
        let equatorial_growth = 0.18 * spin * spin * axis_scale;
        let polar_flattening = 0.14 * spin * spin * axis_scale;

        self.equatorial_radius = self.base_radius * (1.0 + equatorial_growth).max(1.0);
        self.polar_radius =
            (self.base_radius * (1.0 - polar_flattening)).max(self.base_radius * 0.35);
    }
}
