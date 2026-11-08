use crate::renderer::ship::Mat4;

pub struct FisheyeRenderer {
    pub curvature: f32,
}

impl FisheyeRenderer {
    pub fn new() -> Self {
        Self { curvature: 0.55 }
    }

    pub fn compute_mvp(&self, screen_aspect: f32, tex_aspect: f32) -> [f32; 16] {
        // Orthographic projection mapped to screen quad maintaining natural text aspect ratio
        let scale_x = 1.85;
        let scale_y = (scale_x / tex_aspect.max(0.1)) * screen_aspect;
        Mat4::scale(scale_x, scale_y.clamp(0.3, 1.5), 1.0)
    }

    pub fn get_distortion_uniform(&self, intensity: f32) -> f32 {
        (self.curvature * intensity).clamp(0.2, 0.95)
    }
}

impl Default for FisheyeRenderer {
    fn default() -> Self {
        Self::new()
    }
}
