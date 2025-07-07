use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub fovy_degrees: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pitch: f32,
    yaw: f32,
}

impl Camera {
    pub fn new(aspect: f32, position: Vec3) -> Self {
        let mut camera = Self {
            position,
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            right: Vec3::X,
            fovy_degrees: 70.0,
            aspect,
            near: 0.1,
            far: 100.0,
            pitch: 0.0,  // Start level
            yaw: 0.0,    // Start facing forward
        };
        camera.update_vectors();
        camera
    }

    pub fn get_proj_view_matrix(&self) -> Mat4 {
        let proj = Mat4::perspective_rh(
            self.fovy_degrees.to_radians(),
            self.aspect,
            self.near,
            self.far,
        );
        let view = Mat4::look_at_rh(self.position, self.position + self.forward, self.up);
        proj * view
    }

    pub fn move_forward(&mut self, distance: f32) {
        self.position += self.forward * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        self.position += self.right * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.position += self.up * distance;
    }

    pub fn rotate_x(&mut self, degrees: f32) {
        self.pitch += degrees;
        self.pitch = self.pitch.clamp(-89.0, 89.0);
        self.update_vectors();
    }

    pub fn rotate_y(&mut self, degrees: f32) {
        self.yaw += degrees;
        self.update_vectors();
    }

    fn update_vectors(&mut self) {
        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();

        self.forward = Vec3::new(
            yaw_rad.cos() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.sin() * pitch_rad.cos(),
        )
        .normalize();

        self.right = self.forward.cross(Vec3::Y).normalize();
        self.up = self.right.cross(self.forward).normalize();
    }
}
