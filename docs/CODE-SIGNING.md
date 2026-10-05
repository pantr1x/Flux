# Code signing policy

Flux (the Electron app and Flux Native) is free and open-source software under the [MIT License](../LICENSE).

Windows builds of Flux Native are built on GitHub Actions from the public source in this repository
(`.github/workflows/native.yml`) and signed with a certificate provided by the
[SignPath Foundation](https://signpath.org/foundation) through [SignPath.io](https://signpath.io).

## Roles

- **Author, reviewer and approver:** [pantr1x](https://github.com/pantr1x) – the only person who can approve a signing request.
- Only builds made by the GitHub Actions workflow from this repository (`main` and `claude/**` branches) are submitted for signing.

## What is signed

Only `Flux-Native.exe`, built from the source in this repository. No third-party binaries are signed with this certificate.

## Privacy

Flux does not collect or send personal data. The only network traffic it makes on its own is the check for a newer
version (GitHub) and the features you turn on yourself (GitHub sign-in, Claude AI, plugin store).
Your code and settings stay on your computer.

## Reporting a wrongly flagged file

If antivirus software flags a signed build, open an [issue](https://github.com/pantr1x/Flux/issues) with the threat name and the version.
