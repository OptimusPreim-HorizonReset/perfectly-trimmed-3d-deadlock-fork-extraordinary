use std::{
    f32::consts::PI,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    body::Body,
    config, elements,
    quadtree::{Node, Octree},
};

use quarkstrom::{egui, winit::event::VirtualKeyCode, winit_input_helper::WinitInputHelper};

use palette::{rgb::Rgba, Hsluv, IntoColor};
use ultraviolet::{Vec2, Vec3};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static PAUSED: Lazy<AtomicBool> = Lazy::new(|| false.into());
pub static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub static SPAWN_ENABLED: Lazy<AtomicBool> = Lazy::new(|| true.into());
pub static COLLISIONS_ENABLED: Lazy<AtomicBool> = Lazy::new(|| true.into());
pub static RESET_REQUESTED: Lazy<AtomicBool> = Lazy::new(|| false.into());

pub static BODIES: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static QUADTREE: Lazy<Mutex<Vec<Node>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static CURRENT_CONFIG: Lazy<Mutex<config::InformationsConfig>> =
    Lazy::new(|| Mutex::new(config::InformationsConfig::default()));
pub static SPAWN_QUEUE: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct Renderer {
    camera_target: Vec3,
    camera_distance: f32,
    camera_yaw: f32,
    camera_pitch: f32,
    camera_fov: f32,
    camera_speed: f32,
    camera_rotate_speed: f32,
    viewport_size: Vec2,

    settings_window_open: bool,

    show_bodies: bool,
    show_quadtree: bool,
    show_spin_axes: bool,

    depth_range: (usize, usize),

    sample_mode_open: bool,
    selected_element_atomic_number: u8,
    spawn_dragging: bool,
    spawn_start_world: Vec3,
    spawn_current_world: Vec3,
    spawn_start_screen: Vec2,

    bodies: Vec<Body>,
    quadtree: Vec<Node>,
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        Self {
            camera_target: Vec3::zero(),
            camera_distance: 600.0,
            camera_yaw: PI * 0.25,
            camera_pitch: -0.25,
            camera_fov: PI / 3.0,
            camera_speed: 24.0,
            camera_rotate_speed: 0.0035,
            viewport_size: Vec2::zero(),

            settings_window_open: false,

            show_bodies: true,
            show_quadtree: false,
            show_spin_axes: true,

            depth_range: (0, 0),

            sample_mode_open: false,
            selected_element_atomic_number: 1,
            spawn_dragging: false,
            spawn_start_world: Vec3::zero(),
            spawn_current_world: Vec3::zero(),
            spawn_start_screen: Vec2::zero(),

            bodies: Vec::new(),
            quadtree: Vec::new(),
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        // Guard against a zero-size viewport (e.g. during window minimisation).
        if width == 0 || height == 0 {
            return;
        }

        self.viewport_size = Vec2::new(width as f32, height as f32);
        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::E);

        if input.key_pressed(VirtualKeyCode::Space) {
            let val = PAUSED.load(Ordering::Relaxed);
            PAUSED.store(!val, Ordering::Relaxed)
        }

        if input.key_pressed(VirtualKeyCode::X) {
            let enabled = SPAWN_ENABLED.load(Ordering::Relaxed);
            SPAWN_ENABLED.store(!enabled, Ordering::Relaxed);
        }
        if input.key_pressed(VirtualKeyCode::Y) {
            let enabled = COLLISIONS_ENABLED.load(Ordering::Relaxed);
            COLLISIONS_ENABLED.store(!enabled, Ordering::Relaxed);
        }
        if input.key_pressed(VirtualKeyCode::R) {
            RESET_REQUESTED.store(true, Ordering::Relaxed);
        }

        if input.key_pressed(VirtualKeyCode::Grave) {
            self.sample_mode_open = !self.sample_mode_open;
            self.spawn_dragging = false;
        }

        if input.key_pressed(VirtualKeyCode::P) {
            if self.sample_mode_open {
                if let Some((x, y)) = input.mouse() {
                    let world_pos = self.screen_to_world(Vec2::new(x, y));
                    self.spawn_element_sample(world_pos, Vec3::zero());
                    self.sample_mode_open = false;
                }
            }
        }

        if self.sample_mode_open {
            if input.mouse_pressed(1) {
                if let Some((x, y)) = input.mouse() {
                    self.spawn_dragging = true;
                    self.spawn_start_screen = Vec2::new(x, y);
                    self.spawn_start_world = self.screen_to_world(self.spawn_start_screen);
                    self.spawn_current_world = self.spawn_start_world;
                }
            }
            if self.spawn_dragging {
                if let Some((x, y)) = input.mouse() {
                    self.spawn_current_world = self.screen_to_world(Vec2::new(x, y));
                }
                if input.mouse_released(1) {
                    let velocity = (self.spawn_current_world - self.spawn_start_world) * 0.18;
                    self.spawn_element_sample(self.spawn_start_world, velocity);
                    self.spawn_dragging = false;
                    self.sample_mode_open = false;
                }
            }
        }

        let move_delta = self.camera_speed * 0.04;
        let (forward, right, up) = self.camera_basis();
        let mut pan = Vec3::zero();
        if input.key_held(VirtualKeyCode::W) {
            pan += forward;
        }
        if input.key_held(VirtualKeyCode::S) {
            pan -= forward;
        }
        if input.key_held(VirtualKeyCode::A) {
            pan -= right;
        }
        if input.key_held(VirtualKeyCode::D) {
            pan += right;
        }
        if input.key_held(VirtualKeyCode::Q) {
            pan -= up;
        }
        if input.key_held(VirtualKeyCode::E) {
            pan += up;
        }
        if pan != Vec3::zero() {
            self.camera_target += pan.normalized() * move_delta;
        }

        if input.mouse_held(2) {
            let (mdx, mdy) = input.mouse_diff();
            self.camera_yaw -= mdx * self.camera_rotate_speed;
            self.camera_pitch =
                (self.camera_pitch - mdy * self.camera_rotate_speed).clamp(-PI * 0.42, PI * 0.42);
        }

        let scroll = input.scroll_diff();
        if scroll != 0.0 {
            self.camera_distance =
                (self.camera_distance * (-scroll * 0.075).exp()).clamp(80.0, 2_500_000.0);
        }
    }

    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        {
            let mut lock = UPDATE_LOCK.lock();
            if *lock {
                std::mem::swap(&mut self.bodies, &mut BODIES.lock());
                if self.show_quadtree {
                    std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());
                }
            }
            *lock = false;
        }

        ctx.clear_circles();
        ctx.clear_lines();
        ctx.clear_rects();
        ctx.set_view_pos(Vec2::zero());
        ctx.set_view_scale(1.0);

        let cam_pos = self.camera_pos();
        let (forward, right, up) = self.camera_basis();
        let tan_half = (self.camera_fov * 0.5).tan();
        let width = self.viewport_size.x.max(1.0);
        let height = self.viewport_size.y.max(1.0);
        let aspect = width / height;

        if !self.bodies.is_empty() {
            if self.show_bodies {
                for body in &self.bodies {
                    if let Some((pos, z)) = self.project_point_basis(
                        body.pos, cam_pos, forward, right, up, tan_half, aspect,
                    ) {
                        let radius = body.projected_radius() / (z * tan_half).max(0.01);
                        let hue = ((body.element_mass_dimension.max(1.0).ln() * 10.0) % 180.0) - 90.0;
                        let saturation = (body.element_reactivity.max(1.0).ln() * 10.0).clamp(20.0, 100.0);
                        let lightness = (60.0 + body.element_light.min(20.0)).clamp(30.0, 80.0);
                        let rgba: Rgba = Hsluv::new(hue, saturation, lightness).into_color();
                        let color = rgba.into_format().into();
                        ctx.draw_circle(pos, radius, color);
                        if self.show_spin_axes {
                            let axis_len = body.polar_radius / (z * tan_half).max(0.01) * 0.45;
                            let axis_tip = pos + Vec2::new(0.0, -axis_len);
                            ctx.draw_line(pos, axis_tip, [0x80, 0xff, 0xff, 0xff]);
                        }
                    }
                }
            }
        }

        if self.show_quadtree && !self.quadtree.is_empty() {
            let mut depth_range = self.depth_range;
            if depth_range.0 >= depth_range.1 {
                let mut stack = Vec::new();
                stack.push((Octree::ROOT, 0));

                let mut min_depth = usize::MAX;
                let mut max_depth = 0;
                while let Some((node, depth)) = stack.pop() {
                    let node = &self.quadtree[node];

                    if node.is_leaf() {
                        if depth < min_depth {
                            min_depth = depth;
                        }
                        if depth > max_depth {
                            max_depth = depth;
                        }
                    } else {
                        for i in 0..8 {
                            stack.push((node.children + i, depth + 1));
                        }
                    }
                }

                depth_range = (min_depth, max_depth);
            }
            let (min_depth, max_depth) = depth_range;

            let mut stack = Vec::new();
            stack.push((Octree::ROOT, 0));
            while let Some((node, depth)) = stack.pop() {
                let node = &self.quadtree[node];

                if node.is_branch() && depth < max_depth {
                    for i in 0..8 {
                        stack.push((node.children + i, depth + 1));
                    }
                } else if depth >= min_depth {
                    let oct = node.oct;
                    let half = Vec2::new(0.5, 0.5) * oct.size;
                    let min = oct.center.xy() - half;
                    let max = oct.center.xy() + half;

                    let t = ((depth - min_depth + !node.is_empty() as usize) as f32)
                        / (max_depth - min_depth + 1) as f32;

                    let start_h = -100.0;
                    let end_h = 80.0;
                    let h = start_h + (end_h - start_h) * t;
                    let s = 100.0;
                    let l = t * 100.0;

                    let c = Hsluv::new(h, s, l);
                    let rgba: Rgba = c.into_color();
                    let color = rgba.into_format().into();

                    ctx.draw_rect(min, max, color);
                }
            }
        }
    }

    fn gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Area::new("spawn_mode")
            .fixed_pos(egui::pos2(12.0, 12.0))
            .show(ctx, |ui| {
                ui.label(format!(
                    "Spawn Mode: {}",
                    if SPAWN_ENABLED.load(Ordering::Relaxed) {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                ui.label("Press X to toggle");
            });

        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.show_bodies, "Show Bodies");
                ui.checkbox(&mut self.show_spin_axes, "Show Rotation Axis");
                ui.checkbox(&mut self.show_quadtree, "Show Quadtree");
                ui.label(format!(
                    "Particle spawn: {}",
                    if SPAWN_ENABLED.load(Ordering::Relaxed) {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                ui.label("Toggle with X");
                ui.label(format!(
                    "Collisions: {}",
                    if COLLISIONS_ENABLED.load(Ordering::Relaxed) {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                ui.label("Toggle with Y");
                ui.label("Reset with R");
                ui.separator();
                ui.label("Sample Placement");
                ui.label("Press `^` (Grave) to open sample selector");
                ui.label("Press `P` while selector is open to place sample at cursor");
                ui.label("Right-click drag to place with launch velocity");
                ui.horizontal(|ui| {
                    ui.label("Element:");
                    ui.add(
                        egui::DragValue::new(&mut self.selected_element_atomic_number)
                            .clamp_range(1..=118)
                            .speed(1.0),
                    );
                });
                if let Some(element) = elements::element_by_atomic_number(self.selected_element_atomic_number) {
                    ui.label(format!(
                        "Selected: {} ({})",
                        self.selected_element_atomic_number,
                        element.symbol
                    ));
                    ui.label(format!(
                        "Prime dimension: {}",
                        element.prime_dimension()
                    ));
                    ui.label(format!(
                        "Prime gap: {}",
                        element.prime_gap()
                    ));
                    ui.label(format!(
                        "Mass dimension index: {:.2}",
                        element.mass_dimension()
                    ));
                } else {
                    ui.label(format!(
                        "Selected: {} (?)",
                        self.selected_element_atomic_number
                    ));
                }
                if self.spawn_dragging {
                    let delta = self.spawn_current_world - self.spawn_start_world;
                    ui.label(format!(
                        "Drag velocity preview: {:.3},{:.3},{:.3}",
                        delta.x * 0.18,
                        delta.y * 0.18,
                        delta.z * 0.18
                    ));
                }
                if self.show_quadtree {
                    let range = &mut self.depth_range;
                    ui.horizontal(|ui| {
                        ui.label("Depth Range:");
                        ui.add(egui::DragValue::new(&mut range.0).speed(0.05));
                        ui.label("to");
                        ui.add(egui::DragValue::new(&mut range.1).speed(0.05));
                    });
                }
            });
    }
}

impl Renderer {
    fn spawn_element_sample(&self, world_pos: Vec3, launch_velocity: Vec3) {
        let atomic_number = self.selected_element_atomic_number.clamp(1, 118);
        if let Some(element) = elements::element_by_atomic_number(atomic_number) {
            let sample_config = CURRENT_CONFIG.lock().clone();
            let radius_scale = sample_config.element_radius_scale
                * sample_config.element_sample_orbit_radius_scale;
            let speed_factor = sample_config.element_sample_orbit_speed_factor;

            let mut bodies = elements::generate_atomic_system(
                element,
                world_pos,
                Vec3::new(0.0, 1.0, 0.0),
                sample_config.element_center_mass_unit,
                sample_config.element_orbital_mass_unit,
                radius_scale,
                sample_config.element_prime_alpha,
                sample_config.element_prime_beta,
                sample_config.element_light_energy,
            );

            for body in &mut bodies {
                if (body.pos - world_pos).mag() > 0.01 {
                    body.vel *= speed_factor;
                    body.vel += launch_velocity;
                }
            }

            let mut queue = SPAWN_QUEUE.lock();
            queue.extend(bodies);
        }
    }

    fn camera_pos(&self) -> Vec3 {
        let cos_pitch = self.camera_pitch.cos();
        let x = self.camera_distance * cos_pitch * self.camera_yaw.cos();
        let y = self.camera_distance * self.camera_pitch.sin();
        let z = self.camera_distance * cos_pitch * self.camera_yaw.sin();
        self.camera_target + Vec3::new(x, y, z)
    }

    fn camera_basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = (self.camera_target - self.camera_pos()).normalized();
        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        (forward, right, up)
    }

    fn screen_to_ndc(&self, mouse: Vec2) -> Vec2 {
        let width = self.viewport_size.x.max(1.0);
        let height = self.viewport_size.y.max(1.0);
        Vec2::new(mouse.x / width * 2.0 - 1.0, 1.0 - mouse.y / height * 2.0)
    }

    fn project_point(&self, point: Vec3) -> Option<(Vec2, f32)> {
        let cam_pos = self.camera_pos();
        let (forward, right, up) = self.camera_basis();
        self.project_point_basis(
            point,
            cam_pos,
            forward,
            right,
            up,
            (self.camera_fov * 0.5).tan(),
            self.viewport_size.x.max(1.0) / self.viewport_size.y.max(1.0),
        )
    }

    fn project_point_basis(
        &self,
        point: Vec3,
        cam_pos: Vec3,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        tan_half: f32,
        aspect: f32,
    ) -> Option<(Vec2, f32)> {
        let rel = point - cam_pos;
        let z = rel.dot(forward);
        if z <= 0.1 {
            return None;
        }
        let x = rel.dot(right);
        let y = rel.dot(up);
        let proj_x = x / (z * tan_half);
        let proj_y = y / (z * tan_half);
        let pos = Vec2::new(proj_x * aspect, proj_y);
        Some((pos, z))
    }

    fn screen_to_world(&self, mouse: Vec2) -> Vec3 {
        let ndc = self.screen_to_ndc(mouse);
        let (forward, right, up) = self.camera_basis();
        let tan_half = (self.camera_fov * 0.5).tan();
        let aspect = self.viewport_size.x / self.viewport_size.y;
        let dir =
            (right * (ndc.x * aspect * tan_half) + up * (ndc.y * tan_half) + forward).normalized();
        let origin = self.camera_pos();
        let plane_z = self.camera_target.z;
        let t = (plane_z - origin.z) / dir.z;
        if t <= 0.0 {
            origin + dir * self.camera_distance * 0.25
        } else {
            origin + dir * t
        }
    }
}
