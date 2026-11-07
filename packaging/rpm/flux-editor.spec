%global debug_package %{nil}
%global __os_install_post %{nil}
Name:           flux-editor
Version:        @VER@
Release:        1
Summary:        Flux – a small, fast code editor
License:        MIT
URL:            https://pantr1x.github.io/Flux/
ExclusiveArch:  x86_64
AutoReqProv:    no
Requires:       libxkbcommon, mesa-libGL, libX11, libxcb
Recommends:     webkit2gtk4.1

%description
Run code with one click, see websites live while you type, Claude AI built in.

%install
install -Dm755 %{_sourcedir}/flux-native %{buildroot}%{_bindir}/flux
install -Dm644 %{_sourcedir}/flux.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/flux.png
install -Dm644 %{_sourcedir}/flux.desktop %{buildroot}%{_datadir}/applications/flux.desktop

%files
%{_bindir}/flux
%{_datadir}/icons/hicolor/512x512/apps/flux.png
%{_datadir}/applications/flux.desktop
