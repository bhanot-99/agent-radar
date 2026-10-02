%global repo %{?agent_radar_repo}%{!?agent_radar_repo:agent-radar/agent-radar}

Name:           agent-radar
Version:        0.1.0
Release:        1%{?dist}
Summary:        Ultra-lightweight terminal HUD for AI coding agents
License:        MIT
URL:            https://github.com/%{repo}
Source0:        https://github.com/%{repo}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.75

%description
Agent-Radar is an ultra-lightweight, zero-privilege workspace activity HUD
designed to monitor, categorize, and visually stream AI CLI agent behavior
(Claude Code, Aider, Antigravity, Cursor) directly inside a Linux terminal.

%prep
%autosetup

%build
cargo build --release --locked

%install
mkdir -p %{buildroot}%{_bindir}
install -m 755 target/release/agent-radar %{buildroot}%{_bindir}/agent-radar

%files
%license LICENSE
%doc README.md
%{_bindir}/agent-radar
