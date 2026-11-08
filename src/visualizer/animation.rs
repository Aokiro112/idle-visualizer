/// Smoothstep hermite interpolation
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-5)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolation
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Exponential decay damping: frame-rate independent smooth convergence
pub fn exp_decay(current: f32, target: f32, decay_rate: f32, dt: f32) -> f32 {
    target + (current - target) * (-decay_rate * dt).exp()
}

/// Cubic ease-out
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    t * t * t + 1.0
}

/// Elastic back ease-out for energetic word activation
pub fn ease_out_back(t: f32, overshoot: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    t * t * ((overshoot + 1.0) * t + overshoot) + 1.0
}

/// Damped spring simulator
pub fn damped_spring(
    current: f32,
    target: f32,
    velocity: &mut f32,
    tension: f32,
    damping: f32,
    dt: f32,
) -> f32 {
    let dt = dt.min(0.05); // Prevent spring instability at frame drops
    let force = (target - current) * tension;
    let damping_force = -*velocity * damping;
    let accel = force + damping_force;
    *velocity += accel * dt;
    current + *velocity * dt
}

/// Multi-harmonic organic floating drift
/// Generates gentle, cinematic breathing motion so words feel suspended in space
pub fn organic_drift(time: f32, seed_x: f32, seed_y: f32) -> (f32, f32) {
    let t = time * 0.45;
    let dx = (t * 0.73 + seed_x).sin() * 0.015
        + (t * 1.37 + seed_x * 2.0).cos() * 0.008
        + (t * 0.31 + seed_x * 0.5).sin() * 0.005;

    let dy = (t * 0.61 + seed_y).cos() * 0.012
        + (t * 1.19 + seed_y * 1.5).sin() * 0.007
        + (t * 0.27 + seed_y * 0.7).cos() * 0.004;

    (dx, dy)
}

/// Dynamic rotation drift
pub fn organic_rot_drift(time: f32, seed: f32) -> f32 {
    let t = time * 0.35;
    (t * 0.52 + seed).sin() * 0.018 + (t * 0.89 + seed * 1.7).cos() * 0.009
}
