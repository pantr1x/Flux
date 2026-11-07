# Publishing to the AUR (needs your AUR account)

1. Register at https://aur.archlinux.org and add your SSH public key (My Account → SSH Public Key).
2. Publish:
   ```bash
   git clone ssh://aur@aur.archlinux.org/flux-editor-git.git
   cp packaging/aur/PKGBUILD flux-editor-git/ && cd flux-editor-git
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO && git commit -m "Initial import: flux-editor-git" && git push
   ```
3. Then anyone can run `yay -S flux-editor-git` (or `paru -S flux-editor-git`).

Test first on Arch/CachyOS: `cd packaging/aur && makepkg -si` (builds in ~5 minutes).
Official repositories (extra, cachyos) can only be asked for after the package has users; see `docs/PACKAGING.md`.
