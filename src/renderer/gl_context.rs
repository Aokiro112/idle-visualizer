use std::ffi::CString;
use std::ptr;

pub struct GlProgram {
    pub id: u32,
    pub u_mvp: i32,
    pub u_texture: i32,
    pub u_alpha: i32,
    pub u_glow: i32,
    pub u_glow_color: i32,
    pub u_distortion: i32,
    pub u_color: i32,
}

impl GlProgram {
    /// Creates and links a new OpenGL shader program from vertex and fragment shader sources.
    ///
    /// # Safety
    /// Must be called with a valid, current OpenGL context bound to the calling thread.
    pub unsafe fn new(vertex_src: &str, fragment_src: &str) -> Result<Self, String> {
        let vert_shader = compile_shader(gl::VERTEX_SHADER, vertex_src)?;
        let frag_shader = compile_shader(gl::FRAGMENT_SHADER, fragment_src)?;

        let id = gl::CreateProgram();
        gl::AttachShader(id, vert_shader);
        gl::AttachShader(id, frag_shader);
        gl::LinkProgram(id);

        let mut success = 0;
        gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut len = 0;
            gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0u8; len as usize];
            gl::GetProgramInfoLog(id, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
            gl::DeleteShader(vert_shader);
            gl::DeleteShader(frag_shader);
            return Err(String::from_utf8_lossy(&buffer).to_string());
        }

        gl::DeleteShader(vert_shader);
        gl::DeleteShader(frag_shader);

        let get_loc = |name: &str| {
            let c_str = CString::new(name).unwrap();
            gl::GetUniformLocation(id, c_str.as_ptr())
        };

        Ok(Self {
            id,
            u_mvp: get_loc("u_mvp"),
            u_texture: get_loc("u_texture"),
            u_alpha: get_loc("u_alpha"),
            u_glow: get_loc("u_glow"),
            u_glow_color: get_loc("u_glow_color"),
            u_distortion: get_loc("u_distortion"),
            u_color: get_loc("u_color"),
        })
    }

    /// Binds this program as the active OpenGL shader program.
    ///
    /// # Safety
    /// Must be called with a valid, current OpenGL context bound to the calling thread.
    pub unsafe fn bind(&self) {
        gl::UseProgram(self.id);
    }
}

unsafe fn compile_shader(shader_type: u32, body: &str) -> Result<u32, String> {
    let version_ptr = gl::GetString(gl::VERSION);
    let is_es = if !version_ptr.is_null() {
        let s = std::ffi::CStr::from_ptr(version_ptr as *const _).to_string_lossy();
        s.contains("OpenGL ES")
    } else {
        true
    };

    let headers: &[&str] = if is_es {
        &[
            "#version 300 es\nprecision highp float;\n",
            "#version 330 core\n",
            "#version 100\nprecision highp float;\n",
        ]
    } else {
        &[
            "#version 330 core\n",
            "#version 300 es\nprecision highp float;\n",
            "#version 130\n",
        ]
    };

    let mut last_err = String::new();
    for header in headers {
        let full_src = format!("{}{}", header, body);
        let shader = gl::CreateShader(shader_type);
        let c_str = match CString::new(full_src) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success != 0 {
            return Ok(shader);
        }

        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = vec![0u8; len as usize];
        gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
        last_err = String::from_utf8_lossy(&buffer).to_string();
        gl::DeleteShader(shader);
    }

    Err(last_err)
}

pub struct GlMesh {
    pub vao: u32,
    pub vbo: u32,
    pub index_count: i32,
}

impl GlMesh {
    /// Generates vertex array and vertex buffer for a textured 2D quad mesh.
    ///
    /// # Safety
    /// Must be called with a valid, current OpenGL context bound to the calling thread.
    pub unsafe fn new_quad() -> Self {
        let mut vao = 0;
        let mut vbo = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        // Standard quad [-1, 1], texcoords [0, 1]
        // [x, y, z, u, v, r, g, b, a]
        #[rustfmt::skip]
        let vertices: [f32; 54] = [
            -1.0, -1.0, 0.0,  0.0, 1.0,  1.0, 1.0, 1.0, 1.0,
             1.0, -1.0, 0.0,  1.0, 1.0,  1.0, 1.0, 1.0, 1.0,
             1.0,  1.0, 0.0,  1.0, 0.0,  1.0, 1.0, 1.0, 1.0,

            -1.0, -1.0, 0.0,  0.0, 1.0,  1.0, 1.0, 1.0, 1.0,
             1.0,  1.0, 0.0,  1.0, 0.0,  1.0, 1.0, 1.0, 1.0,
            -1.0,  1.0, 0.0,  0.0, 0.0,  1.0, 1.0, 1.0, 1.0,
        ];

        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        let stride = (9 * std::mem::size_of::<f32>()) as i32;

        // a_pos: location 0
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl::EnableVertexAttribArray(0);

        // a_tex_coord: location 1
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (3 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(1);

        // a_color: location 2
        gl::VertexAttribPointer(
            2,
            4,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (5 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(2);

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        Self {
            vao,
            vbo,
            index_count: 6,
        }
    }

    /// Draws the quad mesh using active OpenGL state.
    ///
    /// # Safety
    /// Must be called with a valid, current OpenGL context bound to the calling thread.
    pub unsafe fn draw(&self) {
        gl::BindVertexArray(self.vao);
        gl::DrawArrays(gl::TRIANGLES, 0, self.index_count);
        gl::BindVertexArray(0);
    }
}
