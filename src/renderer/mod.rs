pub mod fisheye;
pub mod gl_context;
pub mod shaders;
pub mod ship;
pub mod text_atlas;
pub mod visual;

use crate::app::state::AppState;
use crate::renderer::fisheye::FisheyeRenderer;
use crate::renderer::gl_context::{GlMesh, GlProgram};
use crate::renderer::shaders::{
    FISHEYE_FRAGMENT_SHADER_BODY, STANDARD_FRAGMENT_SHADER_BODY, VERTEX_SHADER_BODY,
};
use crate::renderer::ship::Mat4;
use crate::renderer::text_atlas::TextAtlas;
use crate::renderer::visual::VisualRenderer;
use crate::visualizer::VisualLyricEngine;
use std::time::Instant;
use tracing::{error, info};

pub struct LyricRenderer {
    standard_prog: Option<GlProgram>,
    fisheye_prog: Option<GlProgram>,
    mesh: Option<GlMesh>,
    pub text_atlas: TextAtlas,
    pub fisheye: FisheyeRenderer,
    pub visual: VisualRenderer,
    pub visual_engine: VisualLyricEngine,
    last_frame_instant: Instant,
}

impl LyricRenderer {
    pub fn new() -> Self {
        Self {
            standard_prog: None,
            fisheye_prog: None,
            mesh: None,
            text_atlas: TextAtlas::new(),
            fisheye: FisheyeRenderer::new(),
            visual: VisualRenderer::new(),
            visual_engine: VisualLyricEngine::new(),
            last_frame_instant: Instant::now(),
        }
    }

    /// Initialize OpenGL context and compile shaders (must be called inside GLArea realized/render context)
    pub fn init_gl(&mut self) {
        if self.standard_prog.is_some() {
            return;
        }

        static GL_LIB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

        gl::load_with(|s| {
            let handle_val = *GL_LIB.get_or_init(|| unsafe {
                let mut h = libc::dlopen(c"libGL.so.1".as_ptr(), libc::RTLD_LAZY);
                if h.is_null() {
                    h = libc::dlopen(c"libEGL.so.1".as_ptr(), libc::RTLD_LAZY);
                }
                h as usize
            });

            if handle_val == 0 {
                return std::ptr::null();
            }
            let handle = handle_val as *mut std::os::raw::c_void;

            let c_str = std::ffi::CString::new(s).unwrap();
            unsafe {
                let sym = libc::dlsym(handle, c_str.as_ptr());
                if !sym.is_null() {
                    return sym;
                }

                let glx_get_proc = libc::dlsym(handle, c"glXGetProcAddressARB".as_ptr());
                if !glx_get_proc.is_null() {
                    let func: extern "C" fn(*const u8) -> *mut std::os::raw::c_void =
                        std::mem::transmute(glx_get_proc);
                    let ptr = func(c_str.as_ptr() as *const u8);
                    if !ptr.is_null() {
                        return ptr;
                    }
                }

                let egl_get_proc = libc::dlsym(handle, c"eglGetProcAddress".as_ptr());
                if !egl_get_proc.is_null() {
                    let func: extern "C" fn(
                        *const std::os::raw::c_char,
                    ) -> *mut std::os::raw::c_void = std::mem::transmute(egl_get_proc);
                    let ptr = func(c_str.as_ptr());
                    if !ptr.is_null() {
                        return ptr;
                    }
                }

                std::ptr::null()
            }
        });

        unsafe {
            info!("Compiling OpenGL shaders for LyricRenderer...");
            match GlProgram::new(VERTEX_SHADER_BODY, STANDARD_FRAGMENT_SHADER_BODY) {
                Ok(prog) => self.standard_prog = Some(prog),
                Err(e) => error!("Failed to compile standard shader: {}", e),
            }

            match GlProgram::new(VERTEX_SHADER_BODY, FISHEYE_FRAGMENT_SHADER_BODY) {
                Ok(prog) => self.fisheye_prog = Some(prog),
                Err(e) => error!("Failed to compile fisheye shader: {}", e),
            }

            self.mesh = Some(GlMesh::new_quad());

            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.text_atlas.ensure_gl_initialized();
    }

    /// Render a frame of lyrics
    pub fn render(&mut self, app_state: &AppState, width: i32, height: i32) {
        self.init_gl();

        let dt = self.last_frame_instant.elapsed().as_secs_f32().min(0.1);
        self.last_frame_instant = Instant::now();

        let settings = app_state.settings.blocking_read().clone();

        let aspect = (width as f32) / (height.max(1) as f32);

        // Framebuffer clear: pure pitch-black #000000 canvas matching Verci design reference
        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        let mesh = match &self.mesh {
            Some(m) => m,
            None => return,
        };

        // Advance and retrieve rolling lyric items from RollingLyricEngine
        let rendered_words = {
            if let Ok(mut engine) = app_state.rolling_engine.try_lock() {
                engine.update(dt * settings.motion_speed);
                engine.get_rendered_words()
            } else {
                Vec::new()
            }
        };

        let proj = Mat4::perspective(42.0f32.to_radians(), aspect, 0.1, 100.0);

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::ActiveTexture(gl::TEXTURE0);

            if let Some(prog) = &self.standard_prog {
                prog.bind();
                gl::Uniform1i(prog.u_texture, 0);

                // Pass 1: Render rounded rectangular highlight badge behind active/lingering words
                for item in &rendered_words {
                    if item.has_highlight && item.highlight_opacity > 0.01 {
                        let word_aspect = if let Some(word_tex) = self.text_atlas.get_or_render_styled_word(
                            &item.text,
                            item.is_active,
                            item.is_italic,
                            &settings.font_family,
                            settings.font_size,
                        ) {
                            word_tex.aspect_ratio
                        } else {
                            2.0
                        };

                        let half_h = item.scale * 0.088;
                        let half_w = half_h * word_aspect;

                        // Precise box padding matching Verci reference video
                        let box_half_h = half_h + 0.016 * item.scale;
                        let box_half_w = half_w + 0.036 * item.scale;
                        let box_aspect = box_half_w / box_half_h.max(0.01);

                        if let Some(box_tex) = self.text_atlas.get_or_render_rounded_box(box_aspect) {
                            gl::BindTexture(gl::TEXTURE_2D, box_tex.texture.id);

                            let trans = Mat4::translation(item.x, item.y, item.z - 0.005);
                            let scl = Mat4::scale(box_half_w, box_half_h, 1.0);
                            let model = Mat4::mul(&trans, &scl);
                            let mvp = Mat4::mul(&proj, &model);

                            gl::UniformMatrix4fv(prog.u_mvp, 1, gl::FALSE, mvp.as_ptr());
                            gl::Uniform1f(prog.u_alpha, item.highlight_opacity * item.opacity);
                            gl::Uniform1f(prog.u_glow, 0.0);
                            gl::Uniform4f(prog.u_glow_color, 0.0, 0.0, 0.0, 0.0);
                            gl::Uniform4f(
                                prog.u_color,
                                item.highlight_color.r,
                                item.highlight_color.g,
                                item.highlight_color.b,
                                item.highlight_color.a,
                            );

                            mesh.draw();
                        }
                    }
                }

                // Pass 2: Render kinetic typography words
                for item in &rendered_words {
                    if let Some(word_tex) = self.text_atlas.get_or_render_styled_word(
                        &item.text,
                        item.is_active,
                        item.is_italic,
                        &settings.font_family,
                        settings.font_size,
                    ) {
                        gl::BindTexture(gl::TEXTURE_2D, word_tex.texture.id);

                        let half_h = item.scale * 0.088;
                        let half_w = half_h * word_tex.aspect_ratio;

                        let trans = Mat4::translation(item.x, item.y, item.z);
                        let scl = Mat4::scale(half_w, half_h, 1.0);
                        let model = Mat4::mul(&trans, &scl);
                        let mvp = Mat4::mul(&proj, &model);

                        gl::UniformMatrix4fv(prog.u_mvp, 1, gl::FALSE, mvp.as_ptr());
                        gl::Uniform1f(prog.u_alpha, item.opacity);
                        gl::Uniform1f(prog.u_glow, 0.0);
                        gl::Uniform4f(prog.u_glow_color, 0.0, 0.0, 0.0, 0.0);
                        gl::Uniform4f(
                            prog.u_color,
                            item.text_color.r,
                            item.text_color.g,
                            item.text_color.b,
                            item.text_color.a,
                        );

                        mesh.draw();
                    }
                }

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }
    }
}

impl Default for LyricRenderer {
    fn default() -> Self {
        Self::new()
    }
}
