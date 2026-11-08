#!/usr/bin/env bash
# Verci One-Click Launcher

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

# Start YouTube Music Desktop if not already running
if ! pgrep -f "youtube-music-desktop-app" >/dev/null; then
    /opt/ytmdesktop/youtube-music-desktop-app >/dev/null 2>&1 &
fi

# Run Verci in background detached so terminal stays free
nohup "$DIR/target/release/verci" >/dev/null 2>&1 &

echo "✨ Verci chal gaya hai background me bina kisi extra terminal window ke!"
