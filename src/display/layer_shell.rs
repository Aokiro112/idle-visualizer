use gtk4::Window;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tracing::info;

pub struct LayerShellHelper;

impl LayerShellHelper {
    pub fn is_supported() -> bool {
        gtk4_layer_shell::is_supported()
    }

    pub fn setup_overlay(window: &Window, layer: Layer) {
        if !Self::is_supported() {
            info!("Wayland layer-shell protocol not supported by current compositor; using standard windowing.");
            return;
        }

        window.init_layer_shell();
        window.set_layer(layer);

        // Anchor to all 4 screen edges for borderless coverage
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        // Exclusive zone -1 means does not reserve desktop edge space
        window.set_exclusive_zone(-1);
        window.set_namespace(Some("verci-lyrics"));

        // None keyboard mode ensures keystrokes pass directly to active apps
        window.set_keyboard_mode(KeyboardMode::None);
        info!("Wayland layer-shell configured successfully (Overlay mode).");
    }
}
