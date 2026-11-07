# Flux on Linux: packages and official channels

Our own repositories are built by CI (`native.yml`, job `linux`) into the branch `ci-native-linux`:
pacman (`[flux]`), apt, dnf, Flatpak. See the README for the commands. They are unsigned for now.

## Official channels (need the owner's account – cannot be done from CI alone)

| Channel | What is needed |
|---|---|
| **Flathub** (`dev.flux.Flux`) | Pull request to `flathub/flathub` with a manifest that builds from source (Flathub does not take prebuilt Rust binaries): a tagged release of the source, `cargo-sources.json` made with `flatpak-cargo-generator`, and `dev.flux.Flux.metainfo.xml` from `packaging/flatpak/`. Needs a verified id (`dev.flux` domain or `io.github.pantr1x.Flux`; Flathub prefers the second one without an own domain). |
| **AUR** | An AUR account + SSH key; push `packaging/arch/PKGBUILD` as `flux-editor-bin` (or a source `flux-editor` PKGBUILD with a git tag). Then `yay -S flux-editor-bin` works. |
| **Ubuntu PPA / Debian** | A Launchpad account, a GPG key, and a source package (`debian/` folder); a PPA builds from source. |
| **Fedora COPR** | A Fedora account; COPR builds the spec `packaging/rpm/flux-editor.spec` from source (needs a source tarball). |
| **Signed repositories** | A GPG key stored as a repo secret; then `repo-add --sign`, a signed apt `Release`, and `gpgcheck=1` can replace `Optional TrustAll` / `trusted=yes`. |

Most of these want a **tagged source release**; Flux Native is deliberately not published as a GitHub release (see `CLAUDE.md`), so decide on tags first.
