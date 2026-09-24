use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    body::Body,
    chemistry::Molecule,
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
pub static MOLECULES: Lazy<Mutex<Vec<Molecule>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static PROCESS_EVENTS: Lazy<Mutex<Vec<ProcessEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessEventKind {
    Merge,
    Collision,
    Bond,
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessEvent {
    pub pos: Vec3,
    pub kind: ProcessEventKind,
    pub frame: usize,
    pub strength: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayMode {
    Default,
    ProcessDynamics,
    ActivityHeatmap,
}

impl OverlayMode {
    pub fn label(self) -> &'static str {
        match self {
            OverlayMode::Default => "Default (5)",
            OverlayMode::ProcessDynamics => "Process Dynamics (6)",
            OverlayMode::ActivityHeatmap => "Activity Heatmap (7)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerDragMode {
    None,
    GenericSpawn,
    ElementSample,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GasBillboardStyle {
    pub halo_color: [u8; 4],
    pub tail_color: [u8; 4],
    pub core_color: [u8; 4],
    pub center_color: [u8; 4],
    pub blur_offset: Vec2,
}

pub(crate) fn gas_billboard_style(
    base_color: [u8; 4],
    velocity: Vec3,
    blur_strength: f32,
) -> GasBillboardStyle {
    let alpha = (base_color[3] as f32 / 255.0).clamp(0.0, 1.0);
    let halo_alpha = (alpha * 0.32).clamp(0.06, 0.5);
    let core_alpha = (alpha * 0.88).clamp(0.1, 1.0);
    let tail_alpha = (alpha * 0.18).clamp(0.02, 0.38);
    let color_with_alpha = |alpha: f32| {
        [
            base_color[0],
            base_color[1],
            base_color[2],
            (alpha * 255.0) as u8,
        ]
    };

    GasBillboardStyle {
        halo_color: color_with_alpha(halo_alpha),
        tail_color: color_with_alpha(tail_alpha),
        core_color: color_with_alpha(core_alpha),
        center_color: color_with_alpha(alpha),
        blur_offset: Vec2::new(velocity.x, velocity.y) * blur_strength.clamp(0.0, 1.0) * 0.0007,
    }
}

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
    show_mass_dimension_coloring: bool,
    show_element_debug_labels: bool,
    overlay_mode: OverlayMode,

    depth_range: (usize, usize),

    sample_mode_open: bool,
    selected_element_atomic_number: u8,
    pointer_drag_mode: PointerDragMode,
    pointer_screen: Option<Vec2>,
    spawn_start_world: Vec3,
    spawn_current_world: Vec3,
    spawn_start_screen: Vec2,

    show_traces: bool,
    trace_paths: HashMap<u64, VecDeque<Vec3>>,

    bodies: Vec<Body>,
    quadtree: Vec<Node>,
    molecules: Vec<Molecule>,
    process_events: Vec<ProcessEvent>,
    current_frame: usize,
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
            show_mass_dimension_coloring: true,
            show_element_debug_labels: false,
            overlay_mode: OverlayMode::Default,
            show_traces: false,
            trace_paths: HashMap::new(),

            depth_range: (0, 0),

            sample_mode_open: false,
            selected_element_atomic_number: 1,
            pointer_drag_mode: PointerDragMode::None,
            pointer_screen: None,
            spawn_start_world: Vec3::zero(),
            spawn_current_world: Vec3::zero(),
            spawn_start_screen: Vec2::zero(),

            bodies: Vec::new(),
            quadtree: Vec::new(),
            molecules: Vec::new(),
            process_events: Vec::new(),
            current_frame: 0,
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        // Guard against a zero-size viewport (e.g. during window minimisation).
        if width == 0 || height == 0 {
            return;
        }

        self.viewport_size = Vec2::new(width as f32, height as f32);
        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::F1);

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

        if input.key_pressed(VirtualKeyCode::T) {
            self.show_traces = !self.show_traces;
        }

        if input.key_pressed(VirtualKeyCode::R) {
            RESET_REQUESTED.store(true, Ordering::Relaxed);
        }

        if input.key_pressed(VirtualKeyCode::Key5) {
            self.overlay_mode = OverlayMode::Default;
        }
        if input.key_pressed(VirtualKeyCode::Key6) {
            self.overlay_mode = OverlayMode::ProcessDynamics;
        }
        if input.key_pressed(VirtualKeyCode::Key7) {
            self.overlay_mode = OverlayMode::ActivityHeatmap;
        }

        if input.key_pressed(VirtualKeyCode::Grave) {
            self.sample_mode_open = !self.sample_mode_open;
            self.pointer_drag_mode = PointerDragMode::None;
        }

        if input.key_pressed(VirtualKeyCode::P) {
            if self.sample_mode_open {
                let pointer = self
                    .pointer_screen
                    .or_else(|| input.mouse().map(|(x, y)| Vec2::new(x, y)));
                if let Some(pointer) = pointer {
                    let world_pos = self.screen_to_world(pointer);
                    self.spawn_element_sample(world_pos, Vec3::zero());
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
    }

    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        {
            let mut lock = UPDATE_LOCK.lock();
            if *lock {
                std::mem::swap(&mut self.bodies, &mut BODIES.lock());
                std::mem::swap(&mut self.molecules, &mut MOLECULES.lock());
                if self.show_quadtree {
                    std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());
                }
            }
            *lock = false;
        }

        {
            let mut events = PROCESS_EVENTS.lock();
            if !events.is_empty() {
                self.process_events.extend(events.drain(..));
            }
        }
        self.current_frame += 1;
        let frame = self.current_frame;
        self.process_events
            .retain(|event| frame.saturating_sub(event.frame) < 60);

        if self.show_traces {
            self.update_traces();
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
            let chemistry_coloring = CURRENT_CONFIG.lock().enable_chemistry_compass_coloring;
            if self.show_traces {
                let body_colors: HashMap<u64, [u8; 4]> = self
                    .bodies
                    .iter()
                    .map(|body| (body.id, Self::trace_color(body, chemistry_coloring)))
                    .collect();
                for (body_id, path) in self.trace_paths.iter() {
                    let trace_color = body_colors
                        .get(body_id)
                        .copied()
                        .unwrap_or([0xff, 0xff, 0xff, 0xff]);
                    let len = path.len();
                    let mut prev_pos: Option<Vec2> = None;
                    for (index, world_pos) in path.iter().enumerate() {
                        if let Some((pos, _z)) = self.project_point_basis(
                            *world_pos, cam_pos, forward, right, up, tan_half, aspect,
                        ) {
                            if let Some(prev) = prev_pos {
                                let alpha =
                                    ((index + 1) as f32 / len as f32).powf(1.2) * 0.75 + 0.15;
                                let line_color = [
                                    trace_color[0],
                                    trace_color[1],
                                    trace_color[2],
                                    (trace_color[3] as f32 * alpha).clamp(0.0, 255.0) as u8,
                                ];
                                ctx.draw_line(prev, pos, line_color);
                            }
                            prev_pos = Some(pos);
                        } else {
                            prev_pos = None;
                        }
                    }
                }
            }

            if self.show_bodies {
                let config = CURRENT_CONFIG.lock();
                let body_count = self.bodies.len();
                let render_limit = if config.performance_render_body_limit > 0 {
                    config.performance_render_body_limit
                } else {
                    usize::MAX
                };
                let sample_ratio = config.performance_render_sample_ratio.clamp(0.01, 1.0);
                let effective_limit = if sample_ratio < 1.0 {
                    ((render_limit as f32) * sample_ratio).max(1.0) as usize
                } else {
                    render_limit
                };
                let render_step = if body_count > effective_limit {
                    ((body_count + effective_limit - 1) / effective_limit).max(1)
                } else {
                    1
                };
                let lod_distance_factor = config.render_lod_distance_factor.max(0.1);

                if self.overlay_mode == OverlayMode::ProcessDynamics {
                    self.draw_molecule_bonds(ctx, cam_pos, forward, right, up, tan_half, aspect);
                    self.draw_process_event_markers(ctx, cam_pos, forward, right, up, tan_half, aspect);
                }

                for body in self.bodies.iter().step_by(render_step) {
                    if let Some((pos, z)) = self.project_point_basis(
                        body.pos, cam_pos, forward, right, up, tan_half, aspect,
                    ) {
                        let radius = body.projected_radius() / (z * tan_half).max(0.01);
                        if render_step > 1 && z > 1.2 {
                            let min_visible_radius = 0.35 + (z - 1.2) * 0.03 * lod_distance_factor;
                            if radius < min_visible_radius {
                                continue;
                            }
                        }
                        let (base_color, glow) = self.body_style(body, &config);
                        let display_radius = match body.segment_type {
                            crate::body::ParticleSegmentType::Core => radius * 1.4,
                            crate::body::ParticleSegmentType::Bulge => radius * 1.2,
                            crate::body::ParticleSegmentType::Gas => radius * 1.6,
                            crate::body::ParticleSegmentType::Stellar => radius * 1.8,
                            crate::body::ParticleSegmentType::Satellite => radius * 0.9,
                            crate::body::ParticleSegmentType::Orbital => radius * 1.0,
                            _ => radius,
                        };
                        if body.segment_type == crate::body::ParticleSegmentType::Gas {
                            self.draw_gas_billboard(
                                ctx,
                                pos,
                                display_radius,
                                base_color,
                                body.vel,
                                config.gas_motion_blur_strength,
                            );
                        } else {
                            if let Some((glow_color, glow_scale)) = glow {
                                ctx.draw_circle(pos, display_radius * glow_scale, glow_color);
                            }
                            ctx.draw_circle(pos, display_radius, base_color);
                        }
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
            .interactable(false)
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
                ui.label(format!("Overlay: {}", self.overlay_mode.label()));
            });

        let settings_open = self.settings_window_open;
        egui::Window::new("Settings")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.show_bodies, "Show Bodies");
                ui.checkbox(&mut self.show_spin_axes, "Show Rotation Axis");
                ui.checkbox(&mut self.show_quadtree, "Show Quadtree");
                ui.checkbox(
                    &mut self.show_mass_dimension_coloring,
                    "Mass-dimension coloring",
                );
                ui.checkbox(&mut self.show_element_debug_labels, "Element debug labels");
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
                    "Settings Window: {}",
                    if settings_open { "OPEN" } else { "CLOSED" }
                ));
                ui.label("Toggle with F1");
                ui.label(format!(
                    "Collisions: {}",
                    if COLLISIONS_ENABLED.load(Ordering::Relaxed) {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                ui.label("Toggle with Y");
                let current_config = CURRENT_CONFIG.lock();
                ui.label(format!(
                    "Disk equilibrium mode: {}",
                    if current_config.enable_disk_equilibrium_mode {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                ui.label("Enable in informations.md with enable_disk_equilibrium_mode");
                ui.label(format!(
                    "Traces: {}",
                    if self.show_traces { "ON" } else { "OFF" }
                ));
                ui.label("Toggle with T");
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
                if let Some(element) =
                    elements::element_by_atomic_number(self.selected_element_atomic_number)
                {
                    ui.label(format!(
                        "Selected: {} ({})",
                        self.selected_element_atomic_number, element.symbol
                    ));
                    if self.show_element_debug_labels {
                        let electronegativity = if element.electronegativity_is_estimated() {
                            format!("{:.2} (estimated)", element.effective_electronegativity())
                        } else {
                            format!("{:.2}", element.effective_electronegativity())
                        };
                        ui.label(format!("Name: {}", element.name));
                        ui.label(format!("Family: {:?}", element.chemical_family()));
                        ui.label(format!(
                            "EN category: {:?}",
                            element.electronegativity_category()
                        ));
                        ui.label(format!("Electronegativity: {electronegativity}"));
                        ui.label(format!("Reactivity: {:.3}", element.reactivity_score()));
                        ui.label(format!("Prime dimension: {}", element.prime_dimension()));
                        ui.label(format!("Prime gap: {}", element.prime_gap()));
                        ui.label(format!(
                            "Mass dimension index: {:.2}",
                            element.mass_dimension()
                        ));
                    }
                } else {
                    ui.label(format!(
                        "Selected: {} (?)",
                        self.selected_element_atomic_number
                    ));
                }
                let current_config = CURRENT_CONFIG.lock();
                ui.separator();
                ui.label("Elemental Simulation Modes:");
                ui.label(format!(
                    "Mass-dimension visualization: {}",
                    if current_config.enable_elemental_mass_dimension_visualization {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));
                ui.label(format!(
                    "Elemental gravitation tuning: {}",
                    if current_config.enable_elemental_gravitation_tuning {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));
                ui.label(format!(
                    "Chemistry compass coloring: {}",
                    if current_config.enable_chemistry_compass_coloring {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                ));
                ui.separator();
                ui.label("Overlay Mode:");
                ui.label(self.overlay_mode.label());
                ui.label("Press 5 for Default, 6 for Process Dynamics, 7 for Activity Heatmap");
                match self.pointer_drag_mode {
                    PointerDragMode::ElementSample => {
                        let delta = self.spawn_current_world - self.spawn_start_world;
                        ui.label(format!(
                            "Drag velocity preview: {:.3},{:.3},{:.3}",
                            delta.x * 0.18,
                            delta.y * 0.18,
                            delta.z * 0.18
                        ));
                    }
                    PointerDragMode::GenericSpawn => {
                        let drag = (self.pointer_screen.unwrap_or(self.spawn_start_screen)
                            - self.spawn_start_screen)
                            .mag();
                        ui.label(format!("Spawn mass drag: {drag:.1} px"));
                    }
                    PointerDragMode::None => {}
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
        self.handle_scene_pointer_input(ctx);
    }
}

impl Renderer {
    fn handle_scene_pointer_input(&mut self, ctx: &quarkstrom::egui::Context) {
        let (
            pointer_pos,
            pointer_delta,
            primary_pressed,
            primary_released,
            secondary_pressed,
            secondary_released,
            ctrl_down,
            scroll_y,
            screen_rect,
        ) = ctx.input(|input| {
            (
                input.pointer.hover_pos(),
                input.pointer.delta(),
                input.pointer.button_pressed(egui::PointerButton::Primary),
                input.pointer.button_released(egui::PointerButton::Primary),
                input.pointer.button_pressed(egui::PointerButton::Secondary),
                input
                    .pointer
                    .button_released(egui::PointerButton::Secondary),
                input.modifiers.ctrl,
                input.scroll_delta.y,
                input.screen_rect,
            )
        });

        let scale_x = self.viewport_size.x / screen_rect.width().max(1.0);
        let scale_y = self.viewport_size.y / screen_rect.height().max(1.0);
        if let Some(pointer_pos) = pointer_pos {
            self.pointer_screen = Some(Vec2::new(
                (pointer_pos.x - screen_rect.min.x) * scale_x,
                (pointer_pos.y - screen_rect.min.y) * scale_y,
            ));
        }

        let pointer_over_ui = ctx.is_pointer_over_area();
        if self.pointer_drag_mode == PointerDragMode::None && !pointer_over_ui {
            if ctrl_down {
                self.camera_yaw -= pointer_delta.x * scale_x * self.camera_rotate_speed;
                self.camera_pitch = (self.camera_pitch
                    - pointer_delta.y * scale_y * self.camera_rotate_speed)
                    .clamp(-PI * 0.42, PI * 0.42);
            }

            if scroll_y != 0.0 {
                let scroll_steps = scroll_y / 50.0;
                self.camera_distance =
                    (self.camera_distance * (-scroll_steps * 0.075).exp()).clamp(80.0, 2_500_000.0);
            }

            if self.sample_mode_open && secondary_pressed {
                self.begin_pointer_drag(PointerDragMode::ElementSample);
            } else if !self.sample_mode_open
                && SPAWN_ENABLED.load(Ordering::Relaxed)
                && primary_pressed
            {
                self.begin_pointer_drag(PointerDragMode::GenericSpawn);
            }
        }

        if let Some(pointer) = self.pointer_screen {
            match self.pointer_drag_mode {
                PointerDragMode::ElementSample => {
                    self.spawn_current_world = self.screen_to_world(pointer);
                    if secondary_released {
                        let velocity = (self.spawn_current_world - self.spawn_start_world) * 0.18;
                        self.spawn_element_sample(self.spawn_start_world, velocity);
                        self.pointer_drag_mode = PointerDragMode::None;
                        self.sample_mode_open = false;
                    }
                }
                PointerDragMode::GenericSpawn => {
                    self.spawn_current_world = self.screen_to_world(pointer);
                    if primary_released {
                        self.spawn_pointer_body();
                        self.pointer_drag_mode = PointerDragMode::None;
                    }
                }
                PointerDragMode::None => {}
            }
        }
    }

    fn begin_pointer_drag(&mut self, mode: PointerDragMode) {
        if let Some(pointer) = self.pointer_screen {
            self.pointer_drag_mode = mode;
            self.spawn_start_screen = pointer;
            self.spawn_start_world = self.screen_to_world(pointer);
            self.spawn_current_world = self.spawn_start_world;
        }
    }

    fn mass_from_drag_distance(mass_range: (f32, f32), drag_distance: f32) -> f32 {
        let mass_min = mass_range.0.max(f32::MIN_POSITIVE);
        let mass_max = mass_range.1.max(mass_min);
        let fraction = (drag_distance / 180.0).clamp(0.0, 1.0);
        if mass_max <= mass_min {
            mass_min
        } else {
            mass_min * (mass_max / mass_min).powf(fraction)
        }
    }

    fn spawn_pointer_body(&self) {
        let config = CURRENT_CONFIG.lock().clone();
        let drag_distance = (self.pointer_screen.unwrap_or(self.spawn_start_screen)
            - self.spawn_start_screen)
            .mag();
        let mass = Self::mass_from_drag_distance(config.particle_mass_range, drag_distance);
        let angular_speed = (config.spawn_angular_speed_base
            + config.spawn_angular_speed_range * 0.5)
            * config.spin_speed_multiplier;
        let rotation_axis = Vec3::new(0.0, 0.0, 1.0);

        let body = if config.enable_elemental_galaxy_pair_mode && config.n > 1000 {
            let atomic_number = self.selected_element_atomic_number.clamp(1, 118);
            let element = elements::element_by_atomic_number(atomic_number)
                .expect("selected element must be within the periodic table");
            let hydrogen_weight = elements::element_by_atomic_number(1)
                .map(|hydrogen| hydrogen.atomic_weight)
                .unwrap_or(1.0);
            let element_mass = (mass * element.atomic_weight / hydrogen_weight)
                .clamp(config.particle_mass_range.0, config.particle_mass_range.1);
            let radius = (element_mass.cbrt() * 0.8 + 0.02)
                * config.element_radius_scale
                * element.atomic_radius_scale()
                * 0.22;
            Body::new_element(
                self.spawn_start_world,
                Vec3::zero(),
                element_mass,
                radius,
                angular_speed * element.relative_velocity_scale(),
                rotation_axis,
                element.atomic_number,
                crate::body::ParticleSegmentType::Orbital,
                element.prime_dimension() as f32,
                element.reactivity_score(),
                element.magnetism(config.element_prime_alpha),
                element.effective_gravitation(
                    config.element_prime_beta,
                    config.enable_scientific_element_gravity,
                    (
                        config.element_gravity_mass_weight,
                        config.element_gravity_electronegativity_weight,
                        config.element_gravity_radius_weight,
                        config.element_gravity_reactivity_weight,
                    ),
                ),
                element.light_signature(config.element_light_energy),
                element.mass_dimension(),
            )
        } else {
            Body::new(
                self.spawn_start_world,
                Vec3::zero(),
                mass,
                mass.cbrt(),
                angular_speed,
                rotation_axis,
                crate::body::ParticleSegmentType::Orbital,
            )
        };

        SPAWN_QUEUE.lock().push(body);
    }

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
                sample_config.particle_mass_range,
            );

            for body in &mut bodies {
                if (body.pos - world_pos).mag() > 0.01 {
                    body.vel *= speed_factor;
                }
                body.vel += launch_velocity;
            }

            let mut queue = SPAWN_QUEUE.lock();
            queue.extend(bodies);
        }
    }

    fn chemistry_style(
        body: &Body,
        config: &config::InformationsConfig,
    ) -> Option<([u8; 4], Option<([u8; 4], f32)>)> {
        let element = elements::element_by_atomic_number(body.element_atomic_number)?;
        let chemistry_color = element.element_color();
        let mut lightness = chemistry_color.lightness;
        let mut saturation = chemistry_color.saturation;
        let mut alpha = 1.0;

        match body.segment_type {
            crate::body::ParticleSegmentType::Core => {
                lightness += 16.0;
                saturation *= 0.88;
            }
            crate::body::ParticleSegmentType::Bulge => {
                lightness += 8.0;
            }
            crate::body::ParticleSegmentType::Gas => {
                let density_score = (body.gas_density / (config.gas_rest_density * 3.0)
                    * config.gas_density_color_scale)
                    .clamp(0.0, 1.0);
                let temperature_score = ((body.gas_temperature - 0.1) / 1.3
                    * config.gas_temperature_color_scale)
                    .clamp(0.0, 1.0);
                lightness += temperature_score * 18.0 + density_score * 5.0;
                saturation += density_score * 8.0;
                alpha =
                    (config.gas_render_opacity * (0.4 + temperature_score * 0.5)).clamp(0.18, 0.96);
            }
            crate::body::ParticleSegmentType::Stellar => {
                lightness += 12.0;
                saturation *= 0.9;
            }
            crate::body::ParticleSegmentType::Satellite => {
                lightness -= 5.0;
            }
            _ => {}
        }

        let rgba: Rgba = Hsluv::new(
            chemistry_color.hue_degrees,
            saturation.clamp(0.0, 100.0),
            lightness.clamp(20.0, 92.0),
        )
        .into_color();
        let mut base_color: [u8; 4] = rgba.into_format().into();
        base_color[3] = (alpha * 255.0) as u8;

        let glow = match body.segment_type {
            crate::body::ParticleSegmentType::Core => {
                Some(([base_color[0], base_color[1], base_color[2], 0x58], 2.8))
            }
            crate::body::ParticleSegmentType::Bulge => {
                Some(([base_color[0], base_color[1], base_color[2], 0x34], 1.6))
            }
            crate::body::ParticleSegmentType::Gas => Some((
                [
                    base_color[0],
                    base_color[1],
                    base_color[2],
                    (alpha * 80.0) as u8,
                ],
                1.9,
            )),
            crate::body::ParticleSegmentType::Stellar => {
                Some(([base_color[0], base_color[1], base_color[2], 0x60], 2.2))
            }
            _ => None,
        };

        Some((base_color, glow))
    }

    fn draw_gas_billboard(
        &self,
        ctx: &mut quarkstrom::RenderContext,
        pos: Vec2,
        radius: f32,
        base_color: [u8; 4],
        velocity: Vec3,
        blur_strength: f32,
    ) {
        let style = gas_billboard_style(base_color, velocity, blur_strength);
        ctx.draw_circle(pos, radius * 1.75, style.halo_color);
        if blur_strength > 0.01 {
            ctx.draw_circle(pos - style.blur_offset, radius * 1.05, style.tail_color);
        }
        ctx.draw_circle(pos, radius * 1.05, style.core_color);
        ctx.draw_circle(pos, radius * 0.6, style.center_color);
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
        let denominator = dir.dot(forward);
        if denominator.abs() <= 1e-6 {
            origin + dir * self.camera_distance * 0.25
        } else {
            let t = (self.camera_target - origin).dot(forward) / denominator;
            if t <= 0.0 {
                origin + dir * self.camera_distance * 0.25
            } else {
                origin + dir * t
            }
        }
    }
}

impl Renderer {
    fn update_traces(&mut self) {
        for body in &self.bodies {
            let path = self.trace_paths.entry(body.id).or_default();
            path.push_back(body.pos);
            if path.len() > 60 {
                path.pop_front();
            }
        }
    }

    fn trace_color(body: &Body, chemistry_coloring: bool) -> [u8; 4] {
        if chemistry_coloring {
            if let Some(element) = elements::element_by_atomic_number(body.element_atomic_number) {
                let color = element.element_color();
                let rgba: Rgba =
                    Hsluv::new(color.hue_degrees, color.saturation, color.lightness).into_color();
                let color: [u8; 4] = rgba.into_format().into();
                return [color[0], color[1], color[2], 0xff];
            }
        }

        match body.segment_type {
            crate::body::ParticleSegmentType::Core => [0xff, 0xd8, 0x50, 0xff],
            crate::body::ParticleSegmentType::Bulge => [0xff, 0xa0, 0x46, 0xff],
            crate::body::ParticleSegmentType::Orbital => [0x70, 0xb8, 0xff, 0xff],
            crate::body::ParticleSegmentType::Satellite => [0xc6, 0x6b, 0xff, 0xff],
            crate::body::ParticleSegmentType::Stellar => [0xff, 0xf1, 0xcf, 0xff],
            crate::body::ParticleSegmentType::Gas => {
                let temp = (body.gas_temperature.clamp(0.05, 2.0) - 0.05) / 1.95;
                let hue = 190.0 - temp * 70.0;
                let rgba: Rgba = Hsluv::new(hue, 75.0, 65.0).into_color();
                let color: [u8; 4] = rgba.into_format().into();
                [color[0], color[1], color[2], 0xff]
            }
            _ => [0xff, 0xff, 0xff, 0xff],
        }
    }

    fn body_style(
        &self,
        body: &Body,
        config: &config::InformationsConfig,
    ) -> ([u8; 4], Option<([u8; 4], f32)>) {
        match self.overlay_mode {
            OverlayMode::ProcessDynamics => Self::process_dynamics_style(body),
            OverlayMode::ActivityHeatmap => Self::activity_heatmap_style(body, config.dt),
            OverlayMode::Default => Self::default_body_style(body, config, self.show_mass_dimension_coloring),
        }
    }

    fn default_body_style(
        body: &Body,
        config: &config::InformationsConfig,
        show_mass_dimension_coloring: bool,
    ) -> ([u8; 4], Option<([u8; 4], f32)>) {
        let chemistry_style = if config.enable_chemistry_compass_coloring {
            Self::chemistry_style(body, config)
        } else {
            None
        };
        if let Some(style) = chemistry_style {
            return style;
        }

        let enable_mass_visualization = show_mass_dimension_coloring
            && config.enable_elemental_mass_dimension_visualization;
        if enable_mass_visualization {
            let (mass_min, mass_max) = config.particle_mass_range;
            let mass_range = (mass_max - mass_min).max(1e-3);
            let mass_fraction = ((body.mass - mass_min) / mass_range).clamp(0.0, 1.0);
            let mass_hue = 220.0 - mass_fraction * 200.0;
            let mass_saturation = (30.0 + body.element_reactivity * 75.0).clamp(30.0, 100.0);
            let mass_lightness =
                (35.0 + mass_fraction * 40.0 + body.element_light * 1.1).clamp(25.0, 90.0);
            let mass_rgba: [u8; 4] = {
                let rgba: Rgba =
                    Hsluv::new(mass_hue, mass_saturation, mass_lightness).into_color();
                rgba.into_format().into()
            };
            let mut color = mass_rgba;
            color[3] = if body.segment_type == crate::body::ParticleSegmentType::Gas {
                0xcc
            } else {
                0xff
            };
            return (color, None);
        }

        if body.segment_type == crate::body::ParticleSegmentType::Gas {
            let density_score = (body.gas_density / (config.gas_rest_density * 3.0)
                * config.gas_density_color_scale)
                .clamp(0.0, 1.0);
            let temp_score = ((body.gas_temperature - 0.1) / 1.3
                * config.gas_temperature_color_scale)
                .clamp(0.0, 1.0);
            let hue = 215.0 - density_score * 100.0 - temp_score * 30.0;
            let saturation = 60.0 + density_score * 25.0;
            let lightness = 42.0 + temp_score * 32.0;
            let rgba: Rgba = Hsluv::new(hue, saturation, lightness).into_color();
            let mut base_color: [u8; 4] = rgba.into_format().into();
            let alpha = (config.gas_render_opacity * (0.35 + temp_score * 0.55)).clamp(0.18, 0.96);
            base_color[3] = (alpha * 255.0) as u8;
            let glow_color = [
                base_color[0],
                base_color[1],
                base_color[2],
                (alpha * 80.0) as u8,
            ];
            return (base_color, Some((glow_color, 1.9)));
        }

        let base = match body.segment_type {
            crate::body::ParticleSegmentType::Core => [0xff, 0xd8, 0x50, 0xff],
            crate::body::ParticleSegmentType::Bulge => [0xff, 0xa0, 0x46, 0xff],
            crate::body::ParticleSegmentType::Orbital => [0x70, 0xb8, 0xff, 0xff],
            crate::body::ParticleSegmentType::Satellite => [0xc6, 0x6b, 0xff, 0xff],
            crate::body::ParticleSegmentType::Stellar => [0xff, 0xf1, 0xcf, 0xff],
            _ => [0xff, 0xff, 0xff, 0xff],
        };
        let glow = match body.segment_type {
            crate::body::ParticleSegmentType::Core => {
                Some(([0xff, 0xe4, 0x82, 0x40], 2.8))
            }
            crate::body::ParticleSegmentType::Bulge => {
                Some(([0xff, 0xc4, 0x70, 0x40], 1.6))
            }
            crate::body::ParticleSegmentType::Stellar => {
                Some(([0xff, 0xf1, 0xcf, 0x60], 2.2))
            }
            _ => None,
        };
        (base, glow)
    }

    fn process_dynamics_style(body: &Body) -> ([u8; 4], Option<([u8; 4], f32)>) {
        if body.process_flash > 0.2 {
            let intensity = (body.process_flash * 255.0) as u8;
            let flash_color = [0xff, 0xff, 0xcc, intensity];
            let glow = Some(([0xff, 0xff, 0x88, (intensity / 3).max(16)], 2.5));
            return (flash_color, glow);
        }
        if body.molecule_id.is_some() {
            return ([0x40, 0xe0, 0xd0, 0xff], Some(([0x40, 0xe0, 0xd0, 0x40], 2.0)));
        }
        // Inaktive Partikel werden zurückgenommen, damit Ereignisse hervorstechen.
        let dim = [0x60, 0x60, 0x60, 0x60];
        (dim, None)
    }

    fn activity_heatmap_style(
        body: &Body,
        dt: f32,
    ) -> ([u8; 4], Option<([u8; 4], f32)>) {
        #[derive(Clone, Copy)]
        enum ProcessCategory {
            Molecular,
            Gas,
            Collision,
            AdaptiveStress,
            Gravity,
        }

        let category = if body.molecule_id.is_some() {
            ProcessCategory::Molecular
        } else if body.is_gas() || body.gas_density > 0.1 {
            ProcessCategory::Gas
        } else if body.density > 0.35 {
            ProcessCategory::Collision
        } else if body.energy_error > 0.25 || (dt > 0.0 && body.local_dt < dt * 0.5) {
            ProcessCategory::AdaptiveStress
        } else {
            ProcessCategory::Gravity
        };

        let (color, glow) = match category {
            ProcessCategory::Molecular => ([0xd0, 0x50, 0xff, 0xff], Some(([0xd0, 0x50, 0xff, 0x50], 2.0))),
            ProcessCategory::Gas => ([0x40, 0xd0, 0x60, 0xff], Some(([0x40, 0xd0, 0x60, 0x50], 1.8))),
            ProcessCategory::Collision => ([0xff, 0x60, 0x30, 0xff], Some(([0xff, 0x60, 0x30, 0x50], 1.8))),
            ProcessCategory::AdaptiveStress => ([0xff, 0xd0, 0x30, 0xff], Some(([0xff, 0xd0, 0x30, 0x50], 1.8))),
            ProcessCategory::Gravity => ([0x50, 0x90, 0xff, 0xff], None),
        };

        // Kurzzeitige Blitze (z. B. frische Merges) heben sich zusätzlich ab.
        if body.process_flash > 0.15 {
            let flash = (body.process_flash * 255.0) as u8;
            return ([0xff, 0xff, 0xee, flash], Some(([0xff, 0xff, 0xcc, flash / 2], 2.5)));
        }
        (color, glow)
    }

    fn draw_molecule_bonds(
        &self,
        ctx: &mut quarkstrom::RenderContext,
        cam_pos: Vec3,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        tan_half: f32,
        aspect: f32,
    ) {
        let bond_color = [0x40, 0xe0, 0xd0, 0xb0];
        for molecule in &self.molecules {
            let indices = &molecule.body_indices;
            if indices.len() < 2 {
                continue;
            }
            for window in indices.windows(2) {
                let (i, j) = (window[0], window[1]);
                let (Some(a), Some(b)) = (self.bodies.get(i), self.bodies.get(j)) else {
                    continue;
                };
                let Some((pos_a, _)) =
                    self.project_point_basis(a.pos, cam_pos, forward, right, up, tan_half, aspect)
                else {
                    continue;
                };
                let Some((pos_b, _)) =
                    self.project_point_basis(b.pos, cam_pos, forward, right, up, tan_half, aspect)
                else {
                    continue;
                };
                ctx.draw_line(pos_a, pos_b, bond_color);
            }
        }
    }

    fn draw_process_event_markers(
        &self,
        ctx: &mut quarkstrom::RenderContext,
        cam_pos: Vec3,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        tan_half: f32,
        aspect: f32,
    ) {
        for event in &self.process_events {
            let age = self.current_frame.saturating_sub(event.frame) as f32;
            let alpha = (1.0 - age / 60.0).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }
            let Some((pos, z)) =
                self.project_point_basis(event.pos, cam_pos, forward, right, up, tan_half, aspect)
            else {
                continue;
            };
            let base_color = match event.kind {
                ProcessEventKind::Merge => [0xff, 0xff, 0x44],
                ProcessEventKind::Collision => [0xff, 0x88, 0x44],
                ProcessEventKind::Bond => [0x44, 0xff, 0xcc],
            };
            let color = [base_color[0], base_color[1], base_color[2], (alpha * 200.0) as u8];
            let strength_scale = (1.0 + event.strength).log10().clamp(0.0, 2.0);
            let radius = ((6.0 + strength_scale * 4.0) * alpha).max(2.0) / (z * tan_half).max(0.01);
            ctx.draw_circle(pos, radius, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::body::{Body, ParticleSegmentType};
    use quarkstrom::egui;
    use quarkstrom::Renderer as _;
    use ultraviolet::Vec3;

    #[test]
    fn drag_distance_maps_monotonically_to_spawn_mass() {
        let range = (1e-6, 1.0);
        let click_mass = Renderer::mass_from_drag_distance(range, 0.0);
        let medium_mass = Renderer::mass_from_drag_distance(range, 90.0);
        let maximum_mass = Renderer::mass_from_drag_distance(range, 180.0);

        assert!((click_mass - range.0).abs() < 1e-12);
        assert!(medium_mass > click_mass && medium_mass < maximum_mass);
        assert!((maximum_mass - range.1).abs() < 1e-6);
    }

    #[test]
    fn egui_pointer_input_orbits_and_zooms_scene() {
        let mut renderer = Renderer::new();
        renderer.viewport_size = ultraviolet::Vec2::new(900.0, 900.0);
        let initial_yaw = renderer.camera_yaw;
        let initial_distance = renderer.camera_distance;
        let ctx = egui::Context::default();
        let position = egui::pos2(200.0, 200.0);
        let moved_position = egui::pos2(230.0, 215.0);
        let initial_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 900.0),
            )),
            events: vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Middle,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };

        ctx.begin_frame(initial_input);
        renderer.handle_scene_pointer_input(&ctx);
        let _ = ctx.end_frame();

        let moved_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 900.0),
            )),
            events: vec![
                egui::Event::PointerMoved(moved_position),
                egui::Event::Scroll(egui::vec2(0.0, 50.0)),
            ],
            modifiers: egui::Modifiers { ctrl: true, ..Default::default() },
            ..Default::default()
        };
        ctx.begin_frame(moved_input);
        renderer.handle_scene_pointer_input(&ctx);
        let _ = ctx.end_frame();

        assert_ne!(renderer.camera_yaw, initial_yaw);
        assert!(renderer.camera_distance < initial_distance);
    }

    #[test]
    fn screen_to_world_uses_camera_facing_target_plane() {
        let mut renderer = Renderer::new();
        renderer.viewport_size = ultraviolet::Vec2::new(900.0, 900.0);
        renderer.camera_yaw = 1.1;
        renderer.camera_pitch = 0.3;

        let center_hit = renderer.screen_to_world(ultraviolet::Vec2::new(450.0, 450.0));
        assert!((center_hit - renderer.camera_target).mag() < 1e-3);

        let off_center_hit = renderer.screen_to_world(ultraviolet::Vec2::new(650.0, 300.0));
        let (forward, _, _) = renderer.camera_basis();
        assert!((off_center_hit - renderer.camera_target).dot(forward).abs() < 1e-3);
    }

    #[test]
    fn process_dynamics_style_highlights_molecules_and_flashes() {
        let mut body = Body::new(
            Vec3::zero(),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Orbital,
        );
        let (color, _) = Renderer::process_dynamics_style(&body);
        assert_eq!(color, [0x60, 0x60, 0x60, 0x60]);

        body.molecule_id = Some(1);
        let (color, _) = Renderer::process_dynamics_style(&body);
        assert_eq!(color, [0x40, 0xe0, 0xd0, 0xff]);

        body.process_flash = 0.5;
        let (color, _) = Renderer::process_dynamics_style(&body);
        assert_eq!(color[3], 127);
    }

    #[test]
    fn activity_heatmap_categorizes_dominant_process() {
        let mut gravity_body = Body::new(
            Vec3::zero(),
            Vec3::zero(),
            1.0,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Orbital,
        );
        gravity_body.acc = Vec3::new(0.1, 0.0, 0.0);
        gravity_body.local_dt = 1000.0;
        let (color, _) = Renderer::activity_heatmap_style(&gravity_body, 1000.0);
        assert_eq!(color, [0x50, 0x90, 0xff, 0xff]);

        let mut collision_body = gravity_body.clone();
        collision_body.density = 0.5;
        let (color, _) = Renderer::activity_heatmap_style(&collision_body, 1000.0);
        assert_eq!(color, [0xff, 0x60, 0x30, 0xff]);

        let mut molecular_body = gravity_body.clone();
        molecular_body.molecule_id = Some(1);
        let (color, _) = Renderer::activity_heatmap_style(&molecular_body, 1000.0);
        assert_eq!(color, [0xd0, 0x50, 0xff, 0xff]);
    }

    #[test]
    fn default_body_style_respects_mass_dimension_flag() {
        let mut config = crate::config::InformationsConfig::default();
        config.enable_elemental_mass_dimension_visualization = true;
        config.particle_mass_range = (0.1, 1.0);

        let body = Body::new(
            Vec3::zero(),
            Vec3::zero(),
            0.55,
            1.0,
            0.0,
            Vec3::new(0.0, 0.0, 1.0),
            ParticleSegmentType::Orbital,
        );

        let (with_mass, _) =
            Renderer::default_body_style(&body, &config, true);
        let (without_mass, _) =
            Renderer::default_body_style(&body, &config, false);
        assert_ne!(with_mass, without_mass);
    }
}
