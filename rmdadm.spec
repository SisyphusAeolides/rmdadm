Name:           rmdadm
Version:        0.1.0
Release:        6%{?dist}
Summary:        A modern Rust rewrite of mdadm
License:        MIT
URL:            https://github.com/SisyphusCode/rmdadm
Source0:        %{name}-%{version}.tar.gz

%global debug_package %{nil}

BuildRequires:  make
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  systemd-rpm-macros
BuildRequires:  openssl-devel

%description
A modern Rust rewrite of mdadm

%prep
%setup -q

%build
make build

%install
make install DESTDIR=%{buildroot} PREFIX=/usr BINDIR=/usr/sbin SYSTEMDDIR=%{_unitdir} UDEVDIR=/usr/lib/udev/rules.d

%files
/usr/sbin/rmdadm
%{_unitdir}/rmdadm.service
%{_unitdir}/rmdadm-scrub.service
%{_unitdir}/rmdadm-scrub.timer
/usr/lib/udev/rules.d/64-rmdadm.rules

%changelog
* Tue Jun 30 2026 Sisyphus <sisyphus@example.com> - 0.1.0-6
- Fix device number extraction in RAID array creation
- Correct MduDiskInfo field initialization
- Skip SET_ARRAY_INFO for v1.x metadata

* Tue Jun 30 2026 Sisyphus <sisyphus@example.com> - 0.1.0-1
- Initial package
