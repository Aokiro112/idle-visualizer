pub const VERTEX_SHADER_BODY: &str = r#"
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_tex_coord;
layout(location = 2) in vec4 a_color;

uniform mat4 u_mvp;

out vec2 v_tex_coord;
out vec4 v_color;

void main() {
    v_tex_coord = a_tex_coord;
    v_color = a_color;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;

pub const STANDARD_FRAGMENT_SHADER_BODY: &str = r#"
in vec2 v_tex_coord;
in vec4 v_color;

uniform sampler2D u_texture;
uniform float u_alpha;
uniform float u_glow;
uniform vec4 u_glow_color;
uniform vec4 u_color;

out vec4 frag_color;

void main() {
    vec4 tex = texture(u_texture, v_tex_coord);
    if (tex.a < 0.02) {
        discard;
    }
    vec4 tint = (u_color.a > 0.001) ? u_color : v_color;
    vec4 base = tint * tex;
    base.a *= (u_alpha > 0.0 ? u_alpha : 1.0);
    if (base.a < 0.01) {
        discard;
    }
    
    // Dynamic luminescence glow for active singing word
    if (u_glow > 0.01) {
        vec4 glow = u_glow_color * tex.a * u_glow * 0.55;
        base.rgb += glow.rgb;
    }
    
    frag_color = base;
}
"#;

pub const FISHEYE_FRAGMENT_SHADER_BODY: &str = r#"
in vec2 v_tex_coord;
in vec4 v_color;

uniform sampler2D u_texture;
uniform float u_alpha;
uniform float u_distortion; // Curvature strength (e.g. 0.35 to 0.85)
uniform float u_glow;
uniform vec4 u_glow_color;

out vec4 frag_color;

void main() {
    // Transform coordinates to [-1, 1] relative to viewport center
    vec2 p = (v_tex_coord - 0.5) * 2.0;
    float r = length(p);
    
    // Radial barrel/fisheye optical lens warp: center expands closest to glass
    if (r < 1.4) {
        float factor = 1.0 + u_distortion * (r * r);
        p = p * factor;
    }
    
    vec2 distorted_uv = p * 0.5 + 0.5;
    
    // Check UV bounds
    if (distorted_uv.x < 0.0 || distorted_uv.x > 1.0 || distorted_uv.y < 0.0 || distorted_uv.y > 1.0) {
        discard;
    }
    
    vec4 tex = texture(u_texture, distorted_uv);
    if (tex.a < 0.02) {
        discard;
    }
    vec4 base = v_color * tex;
    base.a *= (u_alpha > 0.0 ? u_alpha : 1.0);
    if (base.a < 0.01) {
        discard;
    }
    
    if (u_glow > 0.01) {
        vec4 glow = u_glow_color * tex.a * u_glow * 0.6;
        base.rgb += glow.rgb;
    }
    
    frag_color = base;
}
"#;
