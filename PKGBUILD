# Maintainer: zylquinal <your-email@example.com>
pkgname=anima-linux
pkgver=0.1.2
pkgrel=1
pkgdesc="A desktop mascot manager for Linux"
arch=('x86_64')
url="https://github.com/zylquinal/anima-linux"
license=('GPL-3.0-or-later')
depends=(
    'gtk3'
    'gdk-pixbuf2'
    'glib2'
    'cairo'
    'gsettings-desktop-schemas'
    'sqlite'
    'ffmpeg'
)
makedepends=(
    'rust'
    'cargo'
    'pkg-config'
    'git'
)
optdepends=(
    'libappindicator-gtk3: system tray support (minimize to tray instead of closing)'
    'xorg-xwayland: recommended for taskbar hiding on Wayland sessions'
)
source=("$pkgname::git+$url.git#tag=v$pkgver")
sha256sums=('SKIP')

build() {
    cd "$pkgname"
    cargo build --release
}

check() {
    cd "$pkgname"
    cargo test --release
}

package() {
    cd "$pkgname"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/lib/$pkgname/$pkgname"
    install -Dm755 /dev/stdin "$pkgdir/usr/bin/$pkgname" << 'EOF'
#!/bin/sh
# Anima launch wrapper — auto-detects display environment.
_bin="/usr/lib/anima-linux/anima-linux"

if [ "$XDG_SESSION_TYPE" = "wayland" ] && [ -n "$DISPLAY" ]; then
    # Wayland session with XWayland available: force X11 backend
    # so GTK window hints (skip-taskbar, keep-above) are respected.
    exec env GDK_BACKEND=x11 "$_bin" "$@"
else
    # Native X11 or pure Wayland: run normally
    exec "$_bin" "$@"
fi
EOF

    # Install .desktop file (Exec points to the wrapper script)
    install -Dm644 "assets/$pkgname.desktop" \
        "$pkgdir/usr/share/applications/$pkgname.desktop"

    install -Dm644 icon.png \
        "$pkgdir/usr/share/icons/hicolor/256x256/apps/$pkgname.png"

    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md" 2>/dev/null || true
}

post_install() {
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
    update-desktop-database -q /usr/share/applications 2>/dev/null || true
}

post_upgrade() {
    post_install
}

post_remove() {
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
    update-desktop-database -q /usr/share/applications 2>/dev/null || true
}
