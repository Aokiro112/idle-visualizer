pub struct Mat4;

impl Mat4 {
    pub fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]
    }

    pub fn perspective(fov_rad: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
        let f = 1.0 / (fov_rad / 2.0).tan();
        let nf = 1.0 / (near - far);
        [
            f / aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            0.0,
            0.0,
            (far + near) * nf,
            -1.0,
            0.0,
            0.0,
            (2.0 * far * near) * nf,
            0.0,
        ]
    }

    pub fn translation(x: f32, y: f32, z: f32) -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            x, y, z, 1.0, //
        ]
    }

    pub fn scale(x: f32, y: f32, z: f32) -> [f32; 16] {
        [
            x, 0.0, 0.0, 0.0, //
            0.0, y, 0.0, 0.0, //
            0.0, 0.0, z, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]
    }

    pub fn rotation_x(rad: f32) -> [f32; 16] {
        let (s, c) = rad.sin_cos();
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, c, s, 0.0, //
            0.0, -s, c, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]
    }

    pub fn rotation_y(rad: f32) -> [f32; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, 0.0, -s, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            s, 0.0, c, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]
    }

    pub fn rotation_z(rad: f32) -> [f32; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, s, 0.0, 0.0, //
            -s, c, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]
    }

    pub fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut r = [0.0; 16];
        for c in 0..4 {
            for row in 0..4 {
                r[c * 4 + row] = a[row] * b[c * 4]
                    + a[4 + row] * b[c * 4 + 1]
                    + a[8 + row] * b[c * 4 + 2]
                    + a[12 + row] * b[c * 4 + 3];
            }
        }
        r
    }
}

pub struct ShipRenderer {
    pub drift_time: f32,
}

impl ShipRenderer {
    pub fn new() -> Self {
        Self { drift_time: 0.0 }
    }

    pub fn compute_mvp(
        &mut self,
        aspect: f32,
        tex_aspect: f32,
        line_offset: f32,
        motion_speed: f32,
        dt: f32,
    ) -> [f32; 16] {
        self.drift_time += dt * motion_speed;

        // Subtle 3D camera drift
        let drift_x = (self.drift_time * 0.35).sin() * 0.06;
        let drift_y = (self.drift_time * 0.28).cos() * 0.04;

        let proj = Mat4::perspective(45.0f32.to_radians(), aspect, 0.1, 100.0);

        // Position lyric in depth based on offset relative to active line
        let target_z = -2.6 - line_offset * 1.4;
        let target_y = -line_offset * 0.85 + drift_y;
        let target_x = drift_x;

        let view_trans = Mat4::translation(-target_x, -target_y, target_z);
        let scale_x = 2.4;
        let scale_y = (scale_x / tex_aspect.max(0.1)) * (aspect / 1.777);
        let model_scale = Mat4::scale(scale_x, scale_y.clamp(0.4, 1.2), 1.0);

        let vm = Mat4::mul(&view_trans, &model_scale);
        Mat4::mul(&proj, &vm)
    }
}

impl Default for ShipRenderer {
    fn default() -> Self {
        Self::new()
    }
}
