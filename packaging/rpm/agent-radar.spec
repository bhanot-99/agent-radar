Name:           agent-radar
Version:        0.1.0
Release:        1%{?dist}
Summary:        Ultra-lightweight terminal HUD for AI coding agents
License:        MIT
URL:            https://github.com/agent-radar/agent-radar
BuildArch:      x86_64 aarch64

%description
Agent-Radar is an ultra-lightweight, zero-privilege workspace activity HUD
designed to monitor, categorize, and visually stream AI CLI agent behavior
(Claude Code, Aider, Antigravity, Cursor) directly inside a Linux terminal.

%install
mkdir -p %{buildroot}%{_bindir}
install -m 755 target/release/agent-radar %{buildroot}%{_bindir}/agent-radar

%files
%{_bindir}/agent-radar
