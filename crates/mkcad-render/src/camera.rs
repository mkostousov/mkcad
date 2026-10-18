use glam::{Mat4, Vec3, Vec4Swizzles};

/// Camera projection mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionMode {
    /// Perspective projection with field of view (vertical fov in degrees)
    Perspective(f32),
    /// Orthographic projection with half-height
    Orthographic(f32),
}

/// 3D Camera with orbit-style controls (Z-up coordinate system)
#[derive(Debug, Clone)]
pub struct Camera {
    /// The point the camera orbits around (world space)
    pub target: Vec3,
    /// Distance from target to camera
    pub distance: f32,
    /// Yaw angle in radians (rotation around world Z axis)
    pub yaw: f32,
    /// Pitch angle in radians (angle from XY plane, clamped to [-π/2, π/2])
    pub pitch: f32,
    /// Projection mode (perspective with fov or orthographic with half-height)
    pub projection: ProjectionMode,
}

impl Camera {
    /// Create a new camera
    pub fn new(target: Vec3, distance: f32, yaw: f32, pitch: f32, projection: ProjectionMode) -> Self {
        Self {
            target,
            distance,
            yaw,
            pitch: Self::clamp_pitch(pitch),
            projection,
        }
    }

    /// Clamp pitch to valid range [-π/2, π/2]
    fn clamp_pitch(pitch: f32) -> f32 {
        let max_pitch = std::f32::consts::PI / 2.0 - 0.001; // Slightly less than 90 degrees to avoid gimbal lock
        pitch.max(-max_pitch).min(max_pitch)
    }

    /// Get the camera position in world space (Z-up coordinate system)
    /// Position is calculated as: target + [d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)]
    /// Where pitch is angle from XY plane, yaw is rotation around Z axis
    pub fn position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        let cos_yaw = self.yaw.cos();
        let sin_yaw = self.yaw.sin();

        self.target + Vec3::new(
            self.distance * cos_pitch * cos_yaw,
            self.distance * cos_pitch * sin_yaw,
            self.distance * sin_pitch,
        )
    }

    /// Get the forward direction (pointing from camera to target)
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position()).normalize()
    }

    /// Get the up direction for this camera orientation (Z-up coordinate system)
    /// When looking mostly along Z (pitch near ±π/2), use Y as screen-up; otherwise use world Z
    pub fn up(&self) -> Vec3 {
        let forward = self.forward();
        // If looking mostly up or down along Z axis (top/bottom views)
        if forward.z.abs() > 0.99 {
            if forward.z > 0.0 {
                Vec3::NEG_Y  // Looking up, use -Y as screen-up
            } else {
                Vec3::Y      // Looking down, use +Y as screen-up
            }
        } else {
            Vec3::Z  // Normal case: world Z is screen-up
        }
    }

    /// Get the right direction (perpendicular to forward and up)
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up()).normalize()
    }

    /// Get the view matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, self.up())
    }

    /// Get the projection matrix
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        match self.projection {
            ProjectionMode::Perspective(fov_degrees) => {
                let fov_rad = fov_degrees * std::f32::consts::PI / 180.0;
                Mat4::perspective_rh(fov_rad, aspect_ratio, 0.01, 1000.0)
            }
            ProjectionMode::Orthographic(half_height) => {
                let half_width = half_height * aspect_ratio;
                Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.01, 1000.0)
            }
        }
    }

    /// Get the view-projection matrix
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Zoom by mouse wheel steps, keeping world point under cursor fixed
    /// wheel_steps: positive = zoom in, negative = zoom out
    /// cursor_px: pixel coordinates of the cursor in viewport space
    /// viewport: (width, height) of the viewport in pixels
    pub fn zoom_about_cursor(
        &mut self,
        wheel_steps: f32,
        cursor_px: (f32, f32),
        viewport: (u32, u32),
    ) {
        // Get world point under cursor before zoom
        let world_point_before = self.screen_to_world_on_plane(
            cursor_px,
            viewport,
            self.target,
            self.forward(),
        );

        // Zoom distance: each step is 10% of current distance
        let zoom_factor = (1.1_f32).powf(-wheel_steps);
        self.distance *= zoom_factor;

        // Keep minimum distance
        self.distance = self.distance.max(0.01);

        // Get world point under cursor after zoom
        let world_point_after = self.screen_to_world_on_plane(
            cursor_px,
            viewport,
            self.target,
            self.forward(),
        );

        // Pan to keep the world point fixed
        let delta = world_point_before - world_point_after;
        self.target += delta;
    }

    /// Pan by screen-space pixel offset
    /// dx: pixels right (positive = pan right)
    /// dy: pixels down (positive = pan down)
    /// viewport: (width, height) of the viewport in pixels
    pub fn pan_by_pixels(&mut self, dx: f32, dy: f32, viewport: (u32, u32)) {
        let right = self.right();
        let up = self.up();

        let pan_amount = match self.projection {
            ProjectionMode::Perspective(fov_degrees) => {
                // For perspective, pan distance depends on distance to target
                let fov_rad = fov_degrees * std::f32::consts::PI / 180.0;
                let half_height = self.distance * (fov_rad / 2.0).tan();
                half_height / (viewport.1 as f32 / 2.0)
            }
            ProjectionMode::Orthographic(half_height) => {
                // For ortho, pan distance depends on projection size
                half_height / (viewport.1 as f32 / 2.0)
            }
        };

        // The scene follows the cursor, so the camera target moves opposite to the drag;
        // screen y grows downwards, so a downward drag moves the target up.
        self.target -= right * dx * pan_amount;
        self.target += up * dy * pan_amount;
    }

    /// Orbit around the target
    /// dx: horizontal motion in screen pixels (positive = yaw right)
    /// dy: vertical motion in screen pixels (positive = pitch up)
    /// viewport: (width, height) of the viewport in pixels
    pub fn orbit(&mut self, dx: f32, dy: f32, _viewport: (u32, u32)) {
        // Convert pixel motion to angles
        // Typical: 1 pixel = 0.01 radians
        let angle_per_pixel = 0.01;

        self.yaw -= dx * angle_per_pixel;
        self.pitch += dy * angle_per_pixel;

        // Clamp pitch
        self.pitch = Self::clamp_pitch(self.pitch);
    }

    /// Set camera to look normal to a plane (Z-up coordinate system)
    /// origin: a point on the plane
    /// normal: the plane normal (will be normalized), camera forward will point along this direction
    pub fn look_normal_to_plane(&mut self, origin: Vec3, normal: Vec3) {
        let normal = normal.normalize();

        // position = target + [d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)]
        // forward = target - position = -[d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)] / d
        //         = [-cos(pitch)*cos(yaw), -cos(pitch)*sin(yaw), -sin(pitch)]
        // We want forward = normal, so:
        //   -sin(pitch) = normal.z  =>  pitch = asin(-normal.z)
        //   -cos(pitch)*cos(yaw) = normal.x  =>  cos(yaw) = -normal.x / cos(pitch)
        //   -cos(pitch)*sin(yaw) = normal.y  =>  sin(yaw) = -normal.y / cos(pitch)
        // From the last two: yaw = atan2(-normal.y, -normal.x)

        self.target = origin;
        self.pitch = (-normal.z).asin();
        self.yaw = (-normal.y).atan2(-normal.x);
    }

    /// Create a named orthographic view (Z-up coordinate system)
    /// Position formula: [d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)]
    /// Top: yaw=any, pitch=π/2 → position (0,0,d) looking down -Z
    /// Front: yaw=-π/2, pitch=0 → position (0,-d,0) looking along +Y
    /// Back: yaw=π/2, pitch=0 → position (0,d,0) looking along -Y
    /// Right: yaw=0, pitch=0 → position (d,0,0) looking along -X
    /// Left: yaw=π, pitch=0 → position (-d,0,0) looking along +X
    /// Bottom: yaw=any, pitch=-π/2 → position (0,0,-d) looking up +Z
    /// Iso: yaw=-π/4, pitch=atan(√2) ≈ 0.615 → position (d/2,-d/2,d/√2)
    pub fn orthographic_view(name: &str, origin: Vec3, distance: f32) -> Self {
        let projection = ProjectionMode::Orthographic(distance / 2.0);

        match name.to_lowercase().as_str() {
            "top" => Self::new(origin, distance, 0.0, std::f32::consts::PI / 2.0, projection),
            "bottom" => Self::new(origin, distance, 0.0, -std::f32::consts::PI / 2.0, projection),
            "front" => Self::new(origin, distance, -std::f32::consts::PI / 2.0, 0.0, projection),
            "back" => Self::new(origin, distance, std::f32::consts::PI / 2.0, 0.0, projection),
            "right" => Self::new(origin, distance, 0.0, 0.0, projection),
            "left" => Self::new(origin, distance, std::f32::consts::PI, 0.0, projection),
            "iso" => {
                // Isometric: yaw=-45°, pitch=atan(√2) to get position from (+X,-Y,+Z) direction
                let yaw = -std::f32::consts::PI / 4.0;
                let pitch = std::f32::consts::SQRT_2.atan();  // atan(√2) ≈ 0.615 rad ≈ 35.26°
                Self::new(origin, distance, yaw, pitch, projection)
            }
            _ => panic!("Unknown view name: {}", name),
        }
    }

    /// Convert screen coordinates to world coordinates on a plane
    fn screen_to_world_on_plane(
        &self,
        cursor_px: (f32, f32),
        viewport: (u32, u32),
        plane_origin: Vec3,
        plane_normal: Vec3,
    ) -> Vec3 {
        // Convert pixel coordinates to NDC (Normalized Device Coordinates)
        let ndc_x = 2.0 * cursor_px.0 / viewport.0 as f32 - 1.0;
        let ndc_y = 1.0 - 2.0 * cursor_px.1 / viewport.1 as f32; // Y is inverted

        let aspect_ratio = viewport.0 as f32 / viewport.1 as f32;

        // Create ray from camera through the pixel
        let view_matrix = self.view_matrix();
        let proj_matrix = self.projection_matrix(aspect_ratio);

        // Unproject: NDC -> World
        let view_proj_inv = (proj_matrix * view_matrix).inverse();

        let ndc_near = Vec3::new(ndc_x, ndc_y, -1.0);
        let ndc_far = Vec3::new(ndc_x, ndc_y, 1.0);

        let world_near = (view_proj_inv * ndc_near.extend(1.0)).xyz();
        let world_far = (view_proj_inv * ndc_far.extend(1.0)).xyz();

        let ray_dir = (world_far - world_near).normalize();

        // Ray-plane intersection
        // Ray: P(t) = world_near + t * ray_dir
        // Plane: (P - plane_origin) · plane_normal = 0
        // Solve: (world_near - plane_origin + t * ray_dir) · plane_normal = 0

        let denom = ray_dir.dot(plane_normal);
        if denom.abs() < 1e-6 {
            // Ray is parallel to plane
            return plane_origin;
        }

        let t = (plane_origin - world_near).dot(plane_normal) / denom;
        world_near + ray_dir * t
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    fn assert_vec3_approx_eq(a: Vec3, b: Vec3, tolerance: f32) {
        assert!((a - b).length() < tolerance, "Expected {:?}, got {:?}", b, a);
    }

    fn assert_f32_approx_eq(a: f32, b: f32, tolerance: f32) {
        assert!((a - b).abs() < tolerance, "Expected {:?}, got {:?}", b, a);
    }

    #[test]
    fn test_vw_01_zoom_about_cursor() {
        // VW-01: Zoom about cursor, keeping world point under cursor fixed
        let mut camera = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );

        let viewport = (512u32, 512u32);
        let cursor_px = (256.0, 256.0); // Center of screen

        // Get world point at center before zoom
        let point_before = camera.screen_to_world_on_plane(
            cursor_px,
            viewport,
            camera.target,
            camera.forward(),
        );

        // Zoom in (5 steps)
        camera.zoom_about_cursor(5.0, cursor_px, viewport);

        // Get world point at center after zoom
        let point_after = camera.screen_to_world_on_plane(
            cursor_px,
            viewport,
            camera.target,
            camera.forward(),
        );

        // Points should be approximately the same (within tolerance)
        assert_vec3_approx_eq(point_before, point_after, 1e-3);

        // Distance should have changed
        assert!(camera.distance < 10.0, "Distance should decrease when zooming in");
    }

    #[test]
    fn test_vw_02_pan_by_pixels() {
        // VW-02: Pan by pixels, world point under offset pixel should move by exactly that vector
        let mut camera = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );

        let viewport = (512u32, 512u32);
        let dx = 300.0;
        let dy = -100.0;
        let cursor_px = (256.0, 256.0);

        // Get world point at original position
        let point_before = camera.screen_to_world_on_plane(
            cursor_px,
            viewport,
            camera.target,
            camera.forward(),
        );

        // Pan
        camera.pan_by_pixels(dx, dy, viewport);

        // Get world point at offset position
        let offset_px = (cursor_px.0 + dx, cursor_px.1 + dy);
        let point_after = camera.screen_to_world_on_plane(
            offset_px,
            viewport,
            camera.target,
            camera.forward(),
        );

        // Screen-space exact: the world point that was under the original pixel is now under
        // the pixel offset by (dx, dy).
        assert_vec3_approx_eq(point_after, point_before, 1e-3);
        assert!(camera.target != Vec3::ZERO, "Target should have moved");
    }

    #[test]
    fn test_vw_03_orbit() {
        // VW-03: Orbit with pitch clamping
        let mut camera = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );

        let initial_yaw = camera.yaw;
        let initial_pitch = camera.pitch;

        let viewport = (512u32, 512u32);

        // Orbit right (positive dx)
        camera.orbit(100.0, 0.0, viewport);
        assert!(camera.yaw != initial_yaw, "Yaw should change");
        assert!(camera.pitch == initial_pitch, "Pitch should not change");

        // Orbit up (positive dy)
        camera.orbit(0.0, 100.0, viewport);
        assert!(camera.pitch > initial_pitch, "Pitch should increase");

        // Orbit down aggressively (should be clamped)
        camera.pitch = initial_pitch; // Reset pitch for this test
        camera.orbit(0.0, -2000.0, viewport);
        // Should be clamped to minimum (approximately -π/2 + 0.001)
        let max_pitch = std::f32::consts::PI / 2.0 - 0.001;
        assert!(camera.pitch < -0.001, "Pitch should be significantly negative");
        assert!(camera.pitch > -max_pitch - 0.01, "Pitch should be clamped");
    }

    #[test]
    fn test_vw_04_orthographic_views() {
        // VW-04: Named orthographic views
        let origin = Vec3::ZERO;
        let distance = 20.0;

        let views = vec!["front", "back", "left", "right", "top", "bottom", "iso"];

        for view_name in views {
            let camera = Camera::orthographic_view(view_name, origin, distance);

            // All should have orthographic projection
            match camera.projection {
                ProjectionMode::Orthographic(_) => {},
                _ => panic!("View {} should be orthographic", view_name),
            }

            // All should have target at origin
            assert_vec3_approx_eq(camera.target, origin, TOLERANCE);

            // All should have correct distance
            assert_f32_approx_eq(camera.distance, distance, TOLERANCE);
        }

        // Test specific view directions (Z-up system)
        let front = Camera::orthographic_view("front", origin, distance);
        assert_f32_approx_eq(front.yaw, -std::f32::consts::PI / 2.0, TOLERANCE);
        assert_f32_approx_eq(front.pitch, 0.0, TOLERANCE);

        let top = Camera::orthographic_view("top", origin, distance);
        // Top view pitch is π/2 (clamped to slightly less to avoid gimbal lock)
        assert_f32_approx_eq(top.pitch, std::f32::consts::PI / 2.0 - 0.001, 1e-4);
    }

    #[test]
    fn test_vw_05_look_normal_to_plane() {
        // VW-05: Look normal to plane
        let mut camera = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );

        let plane_origin = Vec3::new(5.0, 3.0, 2.0);
        let plane_normal = Vec3::new(1.0, 1.0, 1.0).normalize();

        camera.look_normal_to_plane(plane_origin, plane_normal);

        // Target should be at plane origin
        assert_vec3_approx_eq(camera.target, plane_origin, TOLERANCE);

        // Forward direction should match plane normal
        let forward = camera.forward();
        assert_vec3_approx_eq(forward, plane_normal, 1e-4);
    }

    #[test]
    fn test_vw_06_camera_basics() {
        // Test basic camera positioning in Z-up system
        // Front view: yaw=-π/2, pitch=0 should give position (0, -d, 0)
        let front = Camera::new(
            Vec3::ZERO,
            10.0,
            -std::f32::consts::PI / 2.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );
        assert_vec3_approx_eq(front.position(), Vec3::new(0.0, -10.0, 0.0), TOLERANCE);
        assert_vec3_approx_eq(front.forward(), Vec3::new(0.0, 1.0, 0.0), 1e-4);
        assert_vec3_approx_eq(front.right(), Vec3::new(1.0, 0.0, 0.0), 1e-4);
        assert_vec3_approx_eq(front.up(), Vec3::new(0.0, 0.0, 1.0), 1e-4);

        // Top view: yaw=0, pitch=π/2-ε should give position close to (0, 0, d)
        let pitch_top = std::f32::consts::PI / 2.0 - 0.001;
        let top = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            pitch_top,
            ProjectionMode::Perspective(45.0),
        );
        // Position: [d*cos(pitch)*cos(yaw), d*cos(pitch)*sin(yaw), d*sin(pitch)]
        // With pitch ≈ π/2, cos(pitch) ≈ 0.001, sin(pitch) ≈ 1
        // So position ≈ (0.01, 0, 10)
        let top_pos = top.position();
        assert!(top_pos.x.abs() < 0.02, "Top X should be near 0, got {}", top_pos.x);
        assert!(top_pos.y.abs() < 0.01, "Top Y should be near 0, got {}", top_pos.y);
        assert!((top_pos.z - 10.0).abs() < 0.001, "Top Z should be near 10, got {}", top_pos.z);

        // forward should be close to (0, 0, -1) (looking down)
        let top_forward = top.forward();
        assert!(top_forward.z < -0.9, "Top view should look down, forward={:?}", top_forward);
        // right should be close to (1, 0, 0)
        let top_right = top.right();
        assert!(top_right.x > 0.99, "Top right X should be ~1, got {}", top_right.x);
        assert!(top_right.y.abs() < 0.01, "Top right Y should be ~0, got {}", top_right.y);
    }

    #[test]
    fn test_view_projection_matrices() {
        let camera = Camera::new(
            Vec3::new(5.0, 5.0, 5.0),
            10.0,
            0.0,
            0.0,
            ProjectionMode::Perspective(45.0),
        );

        let aspect_ratio = 16.0 / 9.0;
        let view_matrix = camera.view_matrix();
        let proj_matrix = camera.projection_matrix(aspect_ratio);
        let vp_matrix = camera.view_projection_matrix(aspect_ratio);

        // View-projection should equal projection * view
        let expected_vp = proj_matrix * view_matrix;
        for i in 0..16 {
            assert_f32_approx_eq(vp_matrix.to_cols_array()[i], expected_vp.to_cols_array()[i], 1e-5);
        }
    }

    #[test]
    fn test_orthographic_projection() {
        let camera = Camera::new(
            Vec3::ZERO,
            10.0,
            0.0,
            0.0,
            ProjectionMode::Orthographic(5.0),
        );

        let aspect_ratio = 2.0;
        let proj = camera.projection_matrix(aspect_ratio);

        // For orthographic projection, corners of the view should be at specific distances
        // Just verify it's a valid matrix
        let determinant = proj.determinant();
        assert!(determinant != 0.0, "Projection matrix should be invertible");
    }
}
