# Anima-Linux

Anima-Linux is a powerful, highly-customizable desktop mascot application for Linux. It allows you to spawn, manage, and interact with animated mascots right on your desktop, with full support for Wayland and X11 transparent windows.

![Anima-Linux Screenshot](assets/app_image.png)
<img width="2560" height="1438" alt="image" src="https://github.com/user-attachments/assets/e4f89765-b530-4060-9381-9cae33617629" />

## Features

- **Multi-Instance Support**: Spawn multiple mascots at once, each with their own unique configuration.
- **3D Rotations & Transformations**: Customize your mascots in real-time with Pitch, Yaw, and Roll 3D rotations, and flip them vertically or horizontally.
- **Real-Time Live Updates**: Fine-tune your mascot's scale, opacity, contrast, brightness, saturation, hue, and color temperature via the control panel with instant visual feedback.
- **Automated Spawning**: Set your favorite mascots to auto-spawn when the application starts.
- **Persistent Positions**: Mascot positions are automatically saved, so they stay exactly where you leave them.
- **System Tray**: When `libappindicator-gtk3` is installed, closing the window minimizes Anima to the system tray instead of quitting. Right-click the tray icon to show/hide the window or quit the application.

## Installation & Running

Ensure you have Rust and Cargo installed, as well as the GTK3 development libraries (`libgtk-3-dev` on Debian/Ubuntu, `gtk3` on Arch, etc.).

```bash
# Clone the repository
git clone https://github.com/zylquinal/anima-linux.git
cd anima-linux

# Run the application (system tray enabled by default)
cargo run --release

# Run without system tray support
cargo run --release --no-default-features
```

## System Tray

The tray feature is enabled by default and requires `libappindicator-gtk3` (Arch: `libappindicator-gtk3`, Debian/Ubuntu: `libappindicator3-dev`) at both build and runtime.

| Behaviour | How to build |
|-----------|-------------|
| Tray enabled — X hides to tray, quit from menu | `cargo build --release` *(default)* |
| Tray disabled — X closes normally | `cargo build --release --no-default-features` |

> **GNOME note**: Standard GNOME Shell does not display AppIndicator tray icons by default. Install the [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/) extension to enable them. KDE Plasma, XFCE, and most other desktop environments work out of the box.

## Configuration

By default, the application stores its database and copied GIF assets in your user's local data directory (e.g., `~/.local/share/anima-linux/`).

You can override this location by setting the `ANIMA_CONFIG` environment variable. This is especially useful for testing or portable setups:

```bash
ANIMA_CONFIG=/path/to/custom/dir cargo run
```

## Usage

1. **Library Tab**: Import new Anima GIFs. Files larger than 10MB will prompt a warning to prevent performance issues.
2. **Instances Tab**: Spawn, despawn, locate, or delete specific instances.
3. **Settings Tab**: Adjust the Live Update delay, toggle real-time rendering, or clear all application data.

## Contributing

Contributions, bug reports, and feature requests are welcome! Feel free to open an issue or submit a pull request.
