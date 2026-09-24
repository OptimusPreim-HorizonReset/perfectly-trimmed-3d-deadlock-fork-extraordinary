use crate::body::Body;
use ultraviolet::Vec3;

#[derive(Clone, Copy)]
pub struct Oct {
    pub center: Vec3,
    pub size: f32,
}

impl Oct {
    pub fn new_containing(bodies: &[Body]) -> Self {
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

        let center = Vec3::new(
            (min_x + max_x) * 0.5,
            (min_y + max_y) * 0.5,
            (min_z + max_z) * 0.5,
        );
        let size = (max_x - min_x).max((max_y - min_y).max(max_z - min_z));

        Self { center, size }
    }

    pub fn find_octant(&self, pos: Vec3) -> usize {
        ((pos.z > self.center.z) as usize) << 2
            | ((pos.y > self.center.y) as usize) << 1
            | ((pos.x > self.center.x) as usize)
    }

    pub fn into_octant(mut self, octant: usize) -> Self {
        self.size *= 0.5;
        self.center.x += ((octant & 1) as f32 - 0.5) * self.size;
        self.center.y += (((octant >> 1) & 1) as f32 - 0.5) * self.size;
        self.center.z += (((octant >> 2) & 1) as f32 - 0.5) * self.size;
        self
    }

    pub fn subdivide(&self) -> [Oct; 8] {
        [0, 1, 2, 3, 4, 5, 6, 7].map(|i| self.into_octant(i))
    }
}

#[derive(Clone)]
pub struct Node {
    pub children: usize,
    pub next: usize,
    pub pos: Vec3,
    pub mass: f32,
    pub oct: Oct,
    pub quadrupole: [f32; 6],
}

impl Node {
    pub fn new(next: usize, oct: Oct) -> Self {
        Self {
            children: 0,
            next,
            pos: Vec3::zero(),
            mass: 0.0,
            oct,
            quadrupole: [0.0; 6],
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children == 0
    }

    pub fn is_branch(&self) -> bool {
        self.children != 0
    }

    pub fn is_empty(&self) -> bool {
        self.mass == 0.0
    }
}

pub struct Octree {
    pub t_sq: f32,
    pub e_sq: f32,
    pub adaptive_mixed_precision: bool,
    pub nodes: Vec<Node>,
    pub parents: Vec<usize>,
}

impl Octree {
    pub const ROOT: usize = 0;

    pub fn new(theta: f32, epsilon: f32) -> Self {
        Self {
            t_sq: theta * theta,
            e_sq: epsilon * epsilon,
            adaptive_mixed_precision: false,
            nodes: Vec::new(),
            parents: Vec::new(),
        }
    }

    pub fn reserve(&mut self, nodes: usize, parents: usize) {
        if self.nodes.capacity() < nodes {
            self.nodes.reserve(nodes - self.nodes.capacity());
        }
        if self.parents.capacity() < parents {
            self.parents.reserve(parents - self.parents.capacity());
        }
    }

    pub fn clear(&mut self, oct: Oct) {
        self.parents.clear();
        if self.nodes.is_empty() {
            self.nodes.push(Node::new(0, oct));
        } else {
            self.nodes.truncate(1);
            self.nodes[0] = Node::new(0, oct);
        }
    }

    fn subdivide(&mut self, node: usize) -> usize {
        self.parents.push(node);
        let children = self.nodes.len();
        self.nodes[node].children = children;

        let nexts = [
            children + 1,
            children + 2,
            children + 3,
            children + 4,
            children + 5,
            children + 6,
            children + 7,
            self.nodes[node].next,
        ];
        let octs = self.nodes[node].oct.subdivide();
        for i in 0..8 {
            self.nodes.push(Node::new(nexts[i], octs[i]));
        }

        children
    }

    pub fn insert(&mut self, pos: Vec3, mass: f32) {
        let mut node = Self::ROOT;

        while self.nodes[node].is_branch() {
            let octant = self.nodes[node].oct.find_octant(pos);
            node = self.nodes[node].children + octant;
        }

        if self.nodes[node].is_empty() {
            self.nodes[node].pos = pos;
            self.nodes[node].mass = mass;
            return;
        }

        let (p, m) = (self.nodes[node].pos, self.nodes[node].mass);
        if pos == p {
            self.nodes[node].mass += mass;
            return;
        }

        loop {
            let children = self.subdivide(node);

            let o1 = self.nodes[node].oct.find_octant(p);
            let o2 = self.nodes[node].oct.find_octant(pos);

            if o1 == o2 {
                node = children + o1;
            } else {
                let n1 = children + o1;
                let n2 = children + o2;

                self.nodes[n1].pos = p;
                self.nodes[n1].mass = m;
                self.nodes[n2].pos = pos;
                self.nodes[n2].mass = mass;
                return;
            }
        }
    }

    pub fn propagate(&mut self) {
        for &node in self.parents.iter().rev() {
            let i = self.nodes[node].children;
            let mut sum = Vec3::zero();
            let mut mass = 0.0;
            let mut quad = [0.0_f32; 6];

            for child_idx in i..i + 8 {
                let child = &self.nodes[child_idx];
                sum += child.pos * child.mass;
                mass += child.mass;
            }

            self.nodes[node].mass = mass;
            if mass > 0.0 {
                self.nodes[node].pos = sum / mass;
            } else {
                self.nodes[node].pos = Vec3::zero();
            }

            for child_idx in i..i + 8 {
                let child = &self.nodes[child_idx];
                let relative = child.pos - self.nodes[node].pos;
                quad = Self::add_quadrupoles(quad, child.quadrupole);
                quad = Self::add_quadrupoles(quad, Self::quadrupole_shift(child.mass, relative));
            }

            self.nodes[node].quadrupole = quad;
        }
    }

    fn quantize_scalar(value: f32, step: f32) -> f32 {
        if step.is_finite() && step > 0.0 {
            (value / step).round() * step
        } else {
            value
        }
    }

    fn quantize_vec3(value: Vec3, step: f32) -> Vec3 {
        Vec3::new(
            Self::quantize_scalar(value.x, step),
            Self::quantize_scalar(value.y, step),
            Self::quantize_scalar(value.z, step),
        )
    }

    fn add_quadrupoles(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
        [
            a[0] + b[0],
            a[1] + b[1],
            a[2] + b[2],
            a[3] + b[3],
            a[4] + b[4],
            a[5] + b[5],
        ]
    }

    fn quadrupole_shift(mass: f32, offset: Vec3) -> [f32; 6] {
        let x = offset.x;
        let y = offset.y;
        let z = offset.z;
        let r2 = offset.mag_sq();
        [
            mass * (3.0 * x * x - r2),
            mass * (3.0 * y * y - r2),
            mass * (3.0 * z * z - r2),
            mass * (3.0 * x * y),
            mass * (3.0 * x * z),
            mass * (3.0 * y * z),
        ]
    }

    fn force_from_delta(&self, delta: Vec3, mass: f32) -> Vec3 {
        let softened_distance_sq = delta.mag_sq() + self.e_sq;
        if softened_distance_sq <= 0.0 {
            return Vec3::zero();
        }

        let denominator = softened_distance_sq * softened_distance_sq.sqrt();
        delta * (mass / denominator).min(f32::MAX)
    }

    fn adaptive_theta_sq(&self, theta_override: f32, multipole_order: u8) -> f32 {
        let order_scale = match multipole_order.clamp(1, 4) {
            1 => 1.0,
            2 => 0.9,
            3 => 0.75,
            _ => 0.65,
        };
        let theta = theta_override.max(0.05) * order_scale;
        theta * theta
    }

    fn quadrupole_force(&self, delta: Vec3, quad: [f32; 6], multipole_order: u8) -> Vec3 {
        let d_sq = delta.mag_sq();
        if d_sq <= 0.0 {
            return Vec3::zero();
        }

        let d = d_sq.sqrt();
        let n = delta / d;

        let qn = quad[0] * n.x * n.x
            + quad[1] * n.y * n.y
            + quad[2] * n.z * n.z
            + 2.0 * quad[3] * n.x * n.y
            + 2.0 * quad[4] * n.x * n.z
            + 2.0 * quad[5] * n.y * n.z;

        let qn_vec = Vec3::new(
            quad[0] * n.x + quad[3] * n.y + quad[4] * n.z,
            quad[3] * n.x + quad[1] * n.y + quad[5] * n.z,
            quad[4] * n.x + quad[5] * n.y + quad[2] * n.z,
        );

        let inv_d5 = 1.0 / (d_sq * d_sq * d);
        let inv_d7 = inv_d5 / d_sq;
        let correction = qn_vec * (3.0 * inv_d5) - delta * (15.0 * qn * inv_d7);

        let scale = match multipole_order.clamp(1, 4) {
            2 => 0.10,
            3 => 0.20,
            _ => 0.35,
        };

        if correction.x.is_finite() && correction.y.is_finite() && correction.z.is_finite() {
            correction * scale
        } else {
            Vec3::zero()
        }
    }

    fn accumulate_force(
        &self,
        node: &Node,
        delta: Vec3,
        d_sq: f32,
        multipole_order: u8,
        use_mixed_precision: bool,
    ) -> Vec3 {
        if d_sq <= 0.0 {
            return Vec3::zero();
        }

        let size_sq = node.oct.size * node.oct.size;
        let distance_ratio = if size_sq > 0.0 {
            d_sq / size_sq
        } else {
            f32::INFINITY
        };

        let (delta, mass) = if use_mixed_precision && node.is_branch() {
            let distance = d_sq.sqrt();
            if distance_ratio >= 256.0 {
                // CPU path note: this is only a coarse, FP16-like quantization hook for
                // very distant cells, not real hardware FP16 acceleration.
                let step = (distance * 0.01).max(1e-4);
                (
                    Self::quantize_vec3(delta, step),
                    Self::quantize_scalar(node.mass, step),
                )
            } else if distance_ratio >= 64.0 {
                let step = (distance * 0.001).max(1e-5);
                (
                    Self::quantize_vec3(delta, step),
                    Self::quantize_scalar(node.mass, step),
                )
            } else {
                (delta, node.mass)
            }
        } else {
            (delta, node.mass)
        };

        let monopole = self.force_from_delta(delta, mass);
        match multipole_order.clamp(1, 4) {
            1 => monopole,
            2 | 3 | 4 => monopole + self.quadrupole_force(delta, node.quadrupole, multipole_order),
            _ => monopole,
        }
    }

    fn acc_with_params(
        &self,
        pos: Vec3,
        theta_sq: f32,
        multipole_order: u8,
        use_mixed_precision: bool,
    ) -> Vec3 {
        let mut acc = Vec3::zero();

        let mut node = Self::ROOT;
        loop {
            let n = &self.nodes[node];
            let d = n.pos - pos;
            let d_sq = d.mag_sq();

            if n.is_leaf() || n.oct.size * n.oct.size < d_sq * theta_sq {
                acc += self.accumulate_force(n, d, d_sq, multipole_order, use_mixed_precision);

                if n.next == 0 {
                    break;
                }
                node = n.next;
            } else {
                node = n.children;
            }
        }

        acc
    }

    fn distance_sq_to_oct(pos: Vec3, oct: &Oct) -> f32 {
        let half = oct.size * 0.5;
        let dx = (pos.x - oct.center.x).abs() - half;
        let dy = (pos.y - oct.center.y).abs() - half;
        let dz = (pos.z - oct.center.z).abs() - half;
        let dx = dx.max(0.0);
        let dy = dy.max(0.0);
        let dz = dz.max(0.0);
        dx * dx + dy * dy + dz * dz
    }

    fn oct_fully_within_sphere(pos: Vec3, radius: f32, oct: &Oct) -> bool {
        let half = oct.size * 0.5;
        let radius_sq = radius * radius;
        for &dx in &[half, -half] {
            for &dy in &[half, -half] {
                for &dz in &[half, -half] {
                    let corner = oct.center + Vec3::new(dx, dy, dz);
                    if (corner - pos).mag_sq() > radius_sq {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn local_mass_within(&self, pos: Vec3, radius: f32) -> f32 {
        if self.nodes.is_empty() || radius <= 0.0 {
            return 0.0;
        }

        let (mass, _) = self.local_mass_and_quadrupole_within(pos, radius);
        mass
    }

    pub fn local_mass_and_quadrupole_within(&self, pos: Vec3, radius: f32) -> (f32, [f32; 6]) {
        if self.nodes.is_empty() || radius <= 0.0 {
            return (0.0, [0.0; 6]);
        }

        let r_sq = radius * radius;
        let mut stack = vec![Self::ROOT];
        let mut mass = 0.0;
        let mut quad = [0.0_f32; 6];

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            let dist_sq = Self::distance_sq_to_oct(pos, &node.oct);
            if dist_sq > r_sq {
                continue;
            }

            if node.is_leaf() || Self::oct_fully_within_sphere(pos, radius, &node.oct) {
                mass += node.mass;
                quad = Self::add_quadrupoles(quad, node.quadrupole);
                continue;
            }

            for child in node.children..node.children + 8 {
                stack.push(child);
            }
        }

        (mass, quad)
    }

    pub fn acc(&self, pos: Vec3) -> Vec3 {
        self.acc_with_params(pos, self.t_sq, 1, false)
    }

    pub fn acc_adaptive(&self, pos: Vec3, theta_override: f32, multipole_order: u8) -> Vec3 {
        self.acc_with_params(
            pos,
            self.adaptive_theta_sq(theta_override, multipole_order),
            multipole_order,
            self.adaptive_mixed_precision,
        )
    }
}
