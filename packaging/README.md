# Agent-Radar Packaging Guide

This directory contains packaging manifests for Linux distributions:

- **`aur/PKGBUILD`**: Arch Linux User Repository (AUR) binary package (`agent-radar-bin`).
  - Pre-built static binary for `x86_64` verified with SHA256 checksums.
  - Supports repository overrides via `AGENT_RADAR_REPO="owner/repo" makepkg`.
- **`aur/PKGBUILD.src`**: Arch Linux source package (`agent-radar`).
  - Compiles natively on both `x86_64` and `aarch64` architectures using `cargo build --release --locked`.
- **`deb/control`**: Debian/Ubuntu package metadata for `cargo-deb` / `dpkg-deb`.
  - Architecture: `any` (multi-arch).
- **`rpm/agent-radar.spec`**: Fedora/RHEL/CentOS RPM spec file.
  - Fully builds from source using `%prep`, `%build` (`cargo build --release --locked`), and `%install`.
  - Supports repository overrides: `rpmbuild -ba --define 'agent_radar_repo owner/repo' packaging/rpm/agent-radar.spec`.

## Repository URL Overrides

When building or testing packages against a custom fork or staging repo:

1. **One-line installer (`install.sh`)**:
   ```bash
   AGENT_RADAR_REPO="your-user/your-fork" ./install.sh
   ```
2. **Arch Linux (`PKGBUILD`)**:
   ```bash
   AGENT_RADAR_REPO="your-user/your-fork" makepkg -si
   ```
3. **RPM (`agent-radar.spec`)**:
   ```bash
   rpmbuild -ba --define "agent_radar_repo your-user/your-fork" packaging/rpm/agent-radar.spec
   ```
