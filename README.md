# idle-visualizer

idle-visualizer is an elegant lyric visualizer for Linux desktops. It monitors playing music across players such as YouTube Music Desktop, Spotify, and browser media sessions, rendering word by word synchronized rolling typography on an immersive pure black canvas.

Inspired by the visual experience of verci.xyz, the application is written entirely in native Rust using GTK4, OpenGL, and Pango Cairo text rasterization.

***

## Transforming Your Idle Screen

When music plays in the background, a display often sits unused or shows static desktop clutter. idle-visualizer converts your monitor into a dynamic visual art display:

1. Pure Pitch Black Atmosphere
The visualizer renders on an absolute zero black canvas. On OLED and modern panels, this creates boundless depth where text appears to emerge organically from the display glass without borders, glows, or distracting visual artifacts.

2. Typographic Scale Hierarchy
Rather than scrolling flat blocks of text, lyrics transition through a structured vertical composition. The central word commands dominant monumental presence, the active sung word receives soft highlight emphasis, and peripheral lyrics float in smaller muted scale.

3. Seamless Rolling Motion
Words glide vertically using smooth cubic easing. The motion feels natural and rhythmic, synchronizing directly with the cadence of the vocalist.

4. Uninterrupted Playback
idle-visualizer automatically requests power assertions from your desktop compositor while songs play. This keeps your screen bright and legible throughout an entire album, gracefully restoring normal sleep behavior once playback stops.

***

## Project Structure

The codebase is organized into modular Rust components:

```
src/
├── main.rs
│   Application entry point and logging configuration
│
├── lib.rs
│   Library root exporting core submodules
│
├── app/
│   ├── application.rs
│   │   GTK4 application lifecycle, background threads, and event handling
│   ├── settings.rs
│   │   Persistent user preferences stored in JSON format
│   └── state.rs
│       Thread safe shared application state across UI and rendering
│
├── display/
│   ├── fullscreen.rs
│   │   Borderless visualizer window, OpenGL area, and tick callback
│   ├── layer_shell.rs
│   │   Wayland layer shell support for ambient desktop surfaces
│   └── sleep_inhibitor.rs
│       Screen saver inhibitor keeping monitors awake during playback
│
├── media/
│   ├── mpris.rs
│   │   D Bus MPRIS discovery for YouTube Music Desktop, Spotify, and browsers
│   └── player.rs
│       Playback state tracking with high resolution monotonic position estimation
│
├── lyrics/
│   ├── lrclib.rs
│   │   LRCLIB API client with intelligent title and artist sanitization
│   ├── synchronizer.rs
│   │   Real time player alignment matching positions to active lyric words
│   ├── model.rs
│   │   Data representations for words, lines, and timestamped lyrics
│   ├── parser.rs
│   │   Parser for standard and enhanced lyric timestamp formats
│   ├── cache.rs
│   │   Persistent local storage caching retrieved lyrics on disk
│   └── mock.rs
│       Offline fallback provider with precalculated timestamps
│
├── visualizer/
│   ├── rolling.rs
│   │   Continuous slot coordinate engine and cubic easing calculation
│   ├── engine.rs
│   │   Dynamic layout coordinator and lyric lifecycle management
│   ├── layout.rs
│   │   Spatial typography distribution algorithms
│   ├── animation.rs
│   │   Spring dynamics and organic motion smoothing
│   └── palette.rs
│       Color interpolation and ambient theme transitions
│
├── renderer/
│   ├── mod.rs
│   │   OpenGL drawing pipeline coordinating highlight badges and typography
│   ├── text_atlas.rs
│   │   Subpixel font rasterizer generating GPU textures via Pango and Cairo
│   ├── shaders.rs
│   │   Vertex and fragment shader sources for perspective rendering
│   ├── gl_context.rs
│   │   OpenGL buffer, vertex array, and texture abstractions
│   ├── ship.rs
│   │   Three dimensional camera projection and spatial depth math
│   ├── fisheye.rs
│   │   Optical lens barrel distortion shaders
│   └── visual.rs
│       Contextual particle animations and physics
│
└── ui/
    ├── tray.rs
    │   StatusNotifierItem system tray menu integration
    └── settings_window.rs
        Libadwaita graphical interface for adjusting settings
```

***

## Customization Guide

You can customize every visual and operational aspect of idle-visualizer. Settings persist automatically inside your configuration file:

```
~/.config/verci/settings.json
```

1. Typography and Font Sizing
You can alter font properties to match your desktop aesthetic. The font family setting allows any installed system font such as Inter, Roboto, Helvetica, or custom sans serif typefaces. The font size setting adjusts base typography scale to suit your screen dimensions and viewing distance.

2. Color Palette
Colors are configured using red, green, blue, and alpha values between 0.0 and 1.0:
* lyric_color: Color of general rolling words. Defaults to warm terracotta beige.
* highlight_color: Color of the active singing word text. Defaults to creamy ivory.
* secondary_color: Color of the rounded highlight box behind the active word. Defaults to dusty rose.
* background_color: Canvas background fill. Defaults to pure pitch black.

3. Media Player Selection
The preferred player setting lets you prioritize your favorite music client. Set this value to ytmdesktop for YouTube Music Desktop App, spotify for Spotify, or leave null to automatically follow any currently playing audio player.

4. Synchronization Timing Offset
The sync offset setting allows millisecond adjustments to compensate for audio hardware latency or Bluetooth wireless delay. Positive values shift lyrics earlier, while negative values introduce delay.

***

## How to Use on Your System

### Prerequisites

Install the required graphic libraries for your Linux distribution.

On Arch Linux:
```bash
sudo pacman -S gtk4 libadwaita mesa pkgconf
```

On Fedora:
```bash
sudo dnf install gtk4-devel libadwaita-devel mesa-libGL-devel
```

On Ubuntu or Debian:
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libgl1-mesa-dev pkg-config
```

### Installation

1. Clone and navigate to the project directory:
```bash
git clone https://github.com/Aokiro112/idle-visualizer.git
cd idle-visualizer
```

2. Compile the release binary:
```bash
cargo build --release
```

3. Install the executable to your user binary path:
```bash
cp target/release/verci ~/.local/bin/idle-visualizer
chmod +x ~/.local/bin/idle-visualizer
```

### Launching the Application

You have multiple convenient methods to launch idle-visualizer:

1. Quick Launch Script
Run the included launcher to automatically ensure YouTube Music Desktop is open and start idle-visualizer in the background:
```bash
./run.sh
```

2. Direct Terminal Launch
Launch directly from your shell:
```bash
idle-visualizer &
```

3. Application Launcher
Open your desktop application menu or press Super key, search for idle-visualizer, and press Enter. The application opens as a standalone window without keeping any terminal open.

### Controls and Shortcuts

1. Play or Pause
Click the rounded pill button in the bottom left corner or press Spacebar. This toggles playback directly inside YouTube Music or your active media player.

2. Toggle Fullscreen
Double click anywhere on the window or press F11.

3. Exit
Press Escape to close the visualizer.
# idle-visualizer
