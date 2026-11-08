pub struct VisualSymbolParticle {
    pub symbol: String,
    pub word_index: usize,
    pub spawn_time: f64,
    pub inactive_time: Option<f64>,
    pub scale: f32,
    pub opacity: f32,
    pub y_offset: f32,
}

impl VisualSymbolParticle {
    pub fn new(symbol: String, word_index: usize, now: f64) -> Self {
        Self {
            symbol,
            word_index,
            spawn_time: now,
            inactive_time: None,
            scale: 0.2,
            opacity: 0.0,
            y_offset: 0.0,
        }
    }

    pub fn update(&mut self, now: f64, is_active: bool) {
        let age = (now - self.spawn_time).max(0.0) as f32;

        if is_active {
            self.inactive_time = None;
            // Spring animation: expand quickly, overshoot, settle
            let t = (age * 4.0).min(1.0);
            let overshoot = 1.0 + (1.0 - t).powi(2) * 0.3 * (t * std::f32::consts::PI).sin();
            self.scale = overshoot;
            self.opacity = (age * 5.0).min(1.0);
            self.y_offset = -(age * 0.08).min(0.12);
        } else {
            let inact_t = *self.inactive_time.get_or_insert(now);
            let decay = ((now - inact_t) * 3.5) as f32;
            self.opacity = (1.0 - decay).max(0.0);
            self.y_offset -= (now - inact_t) as f32 * 0.05;
        }
    }

    pub fn is_alive(&self) -> bool {
        self.opacity > 0.01
    }
}

pub struct VisualRenderer {
    pub active_particle: Option<VisualSymbolParticle>,
}

impl VisualRenderer {
    pub fn new() -> Self {
        Self {
            active_particle: None,
        }
    }

    pub fn trigger_symbol(&mut self, symbol: &str, word_index: usize, now: f64) {
        let should_spawn = match &self.active_particle {
            Some(p) => p.word_index != word_index || p.symbol != symbol,
            None => true,
        };

        if should_spawn {
            self.active_particle = Some(VisualSymbolParticle::new(
                symbol.to_string(),
                word_index,
                now,
            ));
        }
    }

    pub fn update(&mut self, now: f64, active_word_idx: Option<usize>) {
        if let Some(ref mut particle) = self.active_particle {
            let is_active = Some(particle.word_index) == active_word_idx;
            particle.update(now, is_active);
            if !particle.is_alive() {
                self.active_particle = None;
            }
        }
    }
}

impl Default for VisualRenderer {
    fn default() -> Self {
        Self::new()
    }
}
