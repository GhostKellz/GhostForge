# Maintainer: GhostKellz <ckelley@ghostkellz.sh>
pkgname=ghostforge
pkgver=0.1.0
pkgrel=1
pkgdesc="Next-generation Linux gaming platform - Better than Lutris with containers and AI"
arch=('x86_64')
url="https://ghostforge.dev"
license=('GPL-3.0-or-later')
depends=(
    'glibc'
    'gcc-libs'
    'wine'
    'winetricks'
    'dxvk'
    'vulkan-icd-loader'
    'vulkan-tools'
    'lib32-vulkan-icd-loader'
    'sqlite'
    'openssl'
)
makedepends=(
    'rust'
    'cargo'
    'git'
    'pkgconf'
    'cmake'
    'clang'
)
optdepends=(
    'gamemode: For gaming performance optimizations'
    'mangohud: For gaming overlay and performance monitoring'
    'steam: For Steam integration'
    'lutris: For compatibility with existing Lutris installations'
    'docker: For Docker container runtime support'
    'podman: For Podman container runtime support'
    'nvidia-utils: For NVIDIA GPU support'
    'lib32-nvidia-utils: For 32-bit NVIDIA support'
    'mesa: For AMD/Intel GPU support'
    'lib32-mesa: For 32-bit AMD/Intel support'
    'xorg-xrandr: For X11 display management'
    'wayland: For Wayland display protocol support'
    'libdrm: For direct rendering manager support'
    'zenpower3: For AMD Zen 3D V-Cache monitoring'
)
provides=('ghostforge')
conflicts=('ghostforge-git')
source=("git+https://github.com/ghostforge/ghostforge.git#tag=v${pkgver}")
sha256sums=('SKIP')

# Zen 3D V-Cache optimizations
export RUSTFLAGS="-C target-cpu=znver3 -C opt-level=3 -C target-feature=+avx2,+fma"
export CARGO_TARGET_DIR="target"

prepare() {
    cd "$pkgname"

    # Update Cargo dependencies
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"

    # Apply Zen 3D optimizations if detected
    if grep -q "AMD.*3D" /proc/cpuinfo; then
        echo "Detected AMD Zen 3D V-Cache CPU - applying optimizations"
        export RUSTFLAGS="$RUSTFLAGS -C target-feature=+3dnow"
    fi
}

build() {
    cd "$pkgname"

    # Build with gaming performance features enabled
    cargo build \
        --frozen \
        --release \
        --features "gaming-performance,display-management" \
        --bin forge
}

check() {
    cd "$pkgname"

    # Run tests (excluding container tests that require runtime)
    cargo test --frozen --release --lib
}

package() {
    cd "$pkgname"

    # Install binary
    install -Dm755 "target/release/forge" "$pkgdir/usr/bin/forge"

    # Create symlink for legacy compatibility
    ln -s forge "$pkgdir/usr/bin/ghostforge"

    # Install desktop entry
    install -Dm644 <(cat <<EOF
[Desktop Entry]
Type=Application
Name=GhostForge
GenericName=Gaming Platform
Comment=Next-generation Linux gaming platform
Exec=forge gui
Icon=ghostforge
Terminal=false
Categories=Game;
Keywords=gaming;wine;proton;steam;lutris;
StartupNotify=true
EOF
) "$pkgdir/usr/share/applications/ghostforge.desktop"

    # Install systemd service for gaming optimizations
    install -Dm644 <(cat <<EOF
[Unit]
Description=GhostForge Gaming Optimizations
After=graphical-session.target

[Service]
Type=oneshot
ExecStart=/usr/bin/forge optimize --system
RemainAfterExit=yes
User=%i

[Install]
WantedBy=default.target
EOF
) "$pkgdir/usr/lib/systemd/user/ghostforge-optimize.service"

    # Install configuration directory
    install -dm755 "$pkgdir/etc/ghostforge"

    # Install default configuration
    install -Dm644 <(cat <<EOF
# GhostForge Configuration
# This file is sourced by GhostForge on startup

# Gaming optimizations
GHOSTFORGE_ENABLE_GAMEMODE=true
GHOSTFORGE_ENABLE_MANGOHUD=true

# Container runtime preference
GHOSTFORGE_CONTAINER_RUNTIME=bolt

# Display settings
GHOSTFORGE_ENABLE_VRR=true
GHOSTFORGE_ENABLE_HDR=auto

# Performance settings for Zen 3D V-Cache
GHOSTFORGE_ZEN3D_OPTIMIZATIONS=auto
EOF
) "$pkgdir/etc/ghostforge/config"

    # Install documentation
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"

    # Install license
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Install shell completions
    install -dm755 "$pkgdir/usr/share/bash-completion/completions"
    install -dm755 "$pkgdir/usr/share/zsh/site-functions"
    install -dm755 "$pkgdir/usr/share/fish/vendor_completions.d"

    # Generate completions
    "$pkgdir/usr/bin/forge" completions bash > "$pkgdir/usr/share/bash-completion/completions/forge"
    "$pkgdir/usr/bin/forge" completions zsh > "$pkgdir/usr/share/zsh/site-functions/_forge"
    "$pkgdir/usr/bin/forge" completions fish > "$pkgdir/usr/share/fish/vendor_completions.d/forge.fish"

    # Install man page (if it exists)
    if [ -f "docs/forge.1" ]; then
        install -Dm644 "docs/forge.1" "$pkgdir/usr/share/man/man1/forge.1"
    fi
}

# Post-install message
post_install() {
    echo "GhostForge has been installed!"
    echo ""
    echo "To get started:"
    echo "  1. Run 'forge --help' to see available commands"
    echo "  2. Run 'forge gui' to launch the graphical interface"
    echo "  3. Run 'forge setup' to configure your gaming environment"
    echo ""
    echo "For AMD Zen 3D V-Cache users:"
    echo "  - Gaming optimizations are automatically enabled"
    echo "  - Consider installing 'zenpower3' for detailed CPU monitoring"
    echo ""
    echo "Optional dependencies:"
    echo "  - Install 'gamemode' and 'mangohud' for enhanced gaming performance"
    echo "  - Install 'docker' or 'podman' for container runtime support"
    echo ""
    echo "Enable user service for gaming optimizations:"
    echo "  systemctl --user enable ghostforge-optimize.service"
}

post_upgrade() {
    post_install
}