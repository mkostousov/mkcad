//! 3D viewport panel (M3 stages 3b/3c): renders `mkcad_render::viewport::ViewportRenderer` on the
//! app's egui-wgpu device into a texture shown in the central panel.
//!
//! Mouse (Fusion-like, F-study CD rows): wheel = zoom about cursor, middle drag = pan,
//! Shift+middle or right drag = orbit, left click = select (ID-buffer pick).
//! View cube (top right): click a face for the named orthographic view.

use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use glam::Vec3;
use mkcad_render::camera::ProjectionMode;
use mkcad_render::viewport::{frame_bounds, MeshData, ViewportRenderer};
use mkcad_render::Camera;

/// Something the user did in the viewport that the app should log.
pub enum ViewportEvent {
    Selected(Option<u32>),
}

struct SceneItem {
    id: u32,
    name: String,
}

pub struct ViewportPanel {
    rs: RenderState,
    renderer: ViewportRenderer,
    texture: egui::TextureId,
    pub camera: Camera,
    perspective: bool,
    items: Vec<SceneItem>,
    bounds: (Vec3, Vec3),
    hovered_face: Option<usize>,
}

const CUBE_FACES: [(&str, Vec3, &str); 6] = [
    ("TOP", Vec3::Z, "top"),
    ("BOTTOM", Vec3::NEG_Z, "bottom"),
    ("FRONT", Vec3::NEG_Y, "front"),
    ("BACK", Vec3::Y, "back"),
    ("RIGHT", Vec3::X, "right"),
    ("LEFT", Vec3::NEG_X, "left"),
];

impl ViewportPanel {
    pub fn new(rs: &RenderState) -> Self {
        let renderer = ViewportRenderer::new(&rs.device, 64, 64);
        let texture = rs
            .renderer
            .write()
            .register_native_texture(&rs.device, renderer.color_view(), wgpu::FilterMode::Linear);
        let mut panel = Self {
            rs: rs.clone(),
            renderer,
            texture,
            camera: Camera::default(),
            perspective: true,
            items: Vec::new(),
            bounds: (Vec3::splat(-50.0), Vec3::splat(50.0)),
            hovered_face: None,
        };
        panel.load_demo_scene();
        panel
    }

    /// Placeholder content until documents exist (M4/M10): three boxes in mm.
    pub fn load_demo_scene(&mut self) {
        self.renderer.clear_meshes();
        self.items.clear();
        let boxes = [
            ("Base 80x60x10", Vec3::new(-40.0, -30.0, 0.0), Vec3::new(40.0, 30.0, 10.0), [0.72, 0.74, 0.78, 1.0]),
            ("Block 30x30x25", Vec3::new(-30.0, -20.0, 10.0), Vec3::new(0.0, 10.0, 35.0), [0.35, 0.5, 0.85, 1.0]),
            ("Pillar 10x10x40", Vec3::new(15.0, 5.0, 10.0), Vec3::new(25.0, 15.0, 50.0), [0.45, 0.72, 0.5, 1.0]),
        ];
        for (i, (name, min, max, color)) in boxes.into_iter().enumerate() {
            let id = u32::try_from(i).unwrap_or(0) + 1;
            self.renderer.add_mesh(&self.rs.device, id, &MeshData::cuboid(min, max), color);
            self.items.push(SceneItem { id, name: name.to_string() });
        }
        self.bounds = (Vec3::new(-40.0, -30.0, 0.0), Vec3::new(40.0, 30.0, 50.0));
        self.fit();
    }

    /// Load test: ~1M triangles (VW performance row) in four tiles.
    pub fn load_stress_scene(&mut self) {
        self.renderer.clear_meshes();
        self.items.clear();
        let tiles = [(-100.0, -100.0), (0.0, -100.0), (-100.0, 0.0), (0.0, 0.0)];
        for (k, (x, y)) in tiles.into_iter().enumerate() {
            let id = u32::try_from(k).unwrap_or(0) + 1;
            let mesh = MeshData::plane_grid(Vec3::new(x, y, 0.0), 100.0, 354);
            self.renderer.add_mesh(&self.rs.device, id, &mesh, [0.7, 0.72, 0.75, 1.0]);
            self.items.push(SceneItem { id, name: format!("Tile {id} (250k triangles)") });
        }
        self.bounds = (Vec3::new(-100.0, -100.0, 0.0), Vec3::new(100.0, 100.0, 1.0));
        self.fit();
    }

    pub fn triangle_count(&self) -> u64 {
        self.renderer.triangle_count()
    }

    pub fn item_name(&self, id: u32) -> Option<&str> {
        self.items.iter().find(|i| i.id == id).map(|i| i.name.as_str())
    }

    /// Home: frame everything from the iso direction.
    pub fn fit(&mut self) {
        let mut cam = frame_bounds(self.bounds.0, self.bounds.1);
        if !self.perspective {
            cam.projection = ProjectionMode::Orthographic(cam.distance * 0.4);
        }
        self.camera = cam;
    }

    pub fn named_view(&mut self, name: &str) {
        let mut cam = Camera::orthographic_view(name, self.camera.target, self.camera.distance);
        if self.perspective {
            cam.projection = ProjectionMode::Perspective(45.0);
        } else if let ProjectionMode::Orthographic(h) = self.camera.projection {
            cam.projection = ProjectionMode::Orthographic(h);
        }
        self.camera = cam;
    }

    pub const fn is_perspective(&self) -> bool {
        self.perspective
    }

    pub fn set_perspective(&mut self, on: bool) {
        if on == self.perspective {
            return;
        }
        self.perspective = on;
        self.camera.projection = if on {
            ProjectionMode::Perspective(45.0)
        } else {
            // Same visible height at the target as the 45° perspective view.
            ProjectionMode::Orthographic(self.camera.distance * (22.5_f32).to_radians().tan())
        };
    }

    /// Draw the viewport into the remaining space of `ui`.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<ViewportEvent> {
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let ppp = ui.ctx().pixels_per_point();
        // High-DPI: render at physical resolution.
        let size_px = ((rect.width() * ppp).round() as u32, (rect.height() * ppp).round() as u32);
        if self.renderer.resize(&self.rs.device, size_px.0, size_px.1) {
            self.rs.renderer.write().update_egui_texture_from_wgpu_texture(
                &self.rs.device,
                self.renderer.color_view(),
                wgpu::FilterMode::Linear,
                self.texture,
            );
        }
        let vp = self.renderer.size();
        let to_px = |p: Pos2| ((p.x - rect.min.x) * ppp, (p.y - rect.min.y) * ppp);

        let cube_rect = Rect::from_min_size(Pos2::new(rect.max.x - 110.0, rect.min.y + 10.0), Vec2::splat(100.0));
        let pointer = response.hover_pos();
        let over_cube = pointer.is_some_and(|p| cube_rect.contains(p));
        let faces = cube_faces(&self.camera, cube_rect);
        self.hovered_face = pointer.filter(|_| over_cube).and_then(|p| face_at(&faces, p));

        let mut event = None;
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                if let Some(p) = pointer {
                    self.camera.zoom_about_cursor(scroll / 50.0, to_px(p), vp);
                }
            }
        }
        let d = response.drag_delta() * ppp;
        let shift = ui.input(|i| i.modifiers.shift);
        if response.dragged_by(egui::PointerButton::Middle) {
            if shift {
                self.camera.orbit(d.x, d.y, vp);
            } else {
                self.camera.pan_by_pixels(d.x, d.y, vp);
            }
        } else if response.dragged_by(egui::PointerButton::Secondary) {
            self.camera.orbit(d.x, d.y, vp);
        }
        if response.clicked() {
            if let Some(face) = self.hovered_face {
                self.named_view(CUBE_FACES[faces[face].index].2);
            } else if let Some(p) = response.interact_pointer_pos() {
                let (x, y) = to_px(p);
                let hit = self.renderer.pick(&self.rs.device, &self.rs.queue, &self.camera, x as u32, y as u32);
                self.renderer.set_highlight(&self.rs.queue, hit);
                event = Some(ViewportEvent::Selected(hit));
            }
        }

        self.renderer.render(&self.rs.device, &self.rs.queue, &self.camera);
        let painter = ui.painter_at(rect);
        painter.image(
            self.texture,
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        draw_cube(&painter, &faces, self.hovered_face);
        event
    }
}

struct CubeFace {
    index: usize,
    corners: [Pos2; 4],
    depth: f32,
}

/// Projected, front-facing view-cube faces, sorted back to front.
fn cube_faces(camera: &Camera, rect: Rect) -> Vec<CubeFace> {
    let view = camera.view_matrix();
    let c = rect.center();
    let s = rect.width() * 0.3;
    let mut faces = Vec::new();
    for (index, (_, n, _)) in CUBE_FACES.iter().enumerate() {
        let vn = view.transform_vector3(*n);
        if vn.z <= 1e-3 {
            continue;
        }
        // Two in-plane axes for the face.
        let (u, v) = if n.z.abs() > 0.5 { (Vec3::X, Vec3::Y * n.z.signum()) } else { (n.cross(Vec3::Z) * -1.0, Vec3::Z) };
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(a, b)| {
            let p = view.transform_vector3(*n + u * a + v * b);
            Pos2::new(c.x + p.x * s, c.y - p.y * s)
        });
        faces.push(CubeFace { index, corners, depth: vn.z });
    }
    faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    faces
}

fn face_at(faces: &[CubeFace], p: Pos2) -> Option<usize> {
    // Front-most face containing p (faces are sorted back to front).
    faces.iter().enumerate().rev().find(|(_, f)| point_in_quad(p, &f.corners)).map(|(i, _)| i)
}

fn point_in_quad(p: Pos2, q: &[Pos2; 4]) -> bool {
    let mut sign = 0.0_f32;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if cross.abs() < f32::EPSILON {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

fn draw_cube(painter: &egui::Painter, faces: &[CubeFace], hovered: Option<usize>) {
    for (i, f) in faces.iter().enumerate() {
        let shade = (150.0 + 90.0 * f.depth.clamp(0.0, 1.0)) as u8;
        let fill = if hovered == Some(i) { Color32::from_rgb(255, 170, 60) } else { Color32::from_rgb(shade, shade, shade + 8) };
        painter.add(egui::Shape::convex_polygon(f.corners.to_vec(), fill, Stroke::new(1.0_f32, Color32::from_gray(60))));
        if f.depth > 0.35 {
            let center = Pos2::new(
                f.corners.iter().map(|p| p.x).sum::<f32>() / 4.0,
                f.corners.iter().map(|p| p.y).sum::<f32>() / 4.0,
            );
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                CUBE_FACES[f.index].0,
                egui::FontId::proportional(10.0),
                Color32::from_gray(30),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0))
    }

    #[test]
    fn top_view_shows_only_top_face_under_centre() {
        let cam = Camera::orthographic_view("top", Vec3::ZERO, 100.0);
        let faces = cube_faces(&cam, rect());
        let i = face_at(&faces, Pos2::new(50.0, 50.0)).expect("a face at the centre");
        assert_eq!(CUBE_FACES[faces[i].index].0, "TOP");
    }

    #[test]
    fn iso_view_shows_top_front_right() {
        let cam = Camera::orthographic_view("iso", Vec3::ZERO, 100.0);
        let names: Vec<&str> = cube_faces(&cam, rect()).iter().map(|f| CUBE_FACES[f.index].0).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        for n in ["TOP", "FRONT", "RIGHT"] {
            assert!(names.contains(&n), "{names:?}");
        }
    }

    #[test]
    fn front_view_face_click_maps_to_front() {
        let cam = Camera::orthographic_view("front", Vec3::ZERO, 100.0);
        let faces = cube_faces(&cam, rect());
        let i = face_at(&faces, Pos2::new(50.0, 50.0)).expect("face");
        assert_eq!(CUBE_FACES[faces[i].index].2, "front");
        assert!(face_at(&faces, Pos2::new(2.0, 2.0)).is_none());
    }
}
