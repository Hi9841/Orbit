# Build an Orbit release

## Build the files

1. Update `package.version` in `Cargo.toml` using the intended release version.
2. Run `cargo test --locked --all-targets` and `cargo clippy --locked --all-targets -- -D warnings`. Ignored desktop tests remain opt-in.
3. Run `./tools/Build-Release.ps1 -VendorDependencies`.
4. Review `dist/OrbitSetup-VERSION-x64.exe`, `dist/Orbit-VERSION-source.zip`, `dist/SHA256SUMS.txt`, and `dist/THIRD-PARTY-NOTICES.txt`.
5. Record outstanding desktop checks in the release notes. Use a prerelease while those checks remain incomplete.

The archive generator copies explicit source directories. It excludes `.tools`, `.git`, build output, and private signing files. Vendored dependencies include their original license files. Keep the source archive beside every published installer.

## Sign the installer

`packaging/update-public-key.txt` contains the minisign public key, base64-encoded the same way Prism stores `plugins.updater.pubkey`. The private key and its password stay in the user signing directory, outside the repository. Keep an offline backup. Losing them means installed copies of this key cannot verify later releases.

`tools/Build-Release.ps1` signs `dist/OrbitSetup-VERSION-x64.exe` with the Tauri signer. That writes `OrbitSetup-VERSION-x64.exe.sig`. The signature file body is the base64 minisign signature. `orbit-release pack` then writes `dist/latest.json` with `version`, `notes`, `pub_date`, and both `windows-x86_64` and `windows-x86_64-nsis`. Those two platform entries use the same signature and the same installer URL. `orbit-release verify` checks the signature against the installer bytes and the embedded public key.

Do not print the private key or password. Regenerate `latest.json` and the `.sig` file whenever the installer changes.

## Publish

Create a release tagged `vVERSION` in [Hi9841/Orbit](https://github.com/Hi9841/Orbit). Attach the installer, `OrbitSetup-VERSION-x64.exe.sig`, `latest.json`, the source ZIP, dependency notices, and the checksum file.

By default, the application checks `https://github.com/Hi9841/Orbit/releases/latest/download/latest.json`. GitHub excludes prereleases from this endpoint. Stable-only checks also reject prerelease versions in that manifest. HTTP 404 means no stable update is available yet; other network errors are reported for manual checks and do not prevent window management.

With "Include development versions" enabled, Orbit reads the repository's release listing and uses signed `latest.json` assets before choosing the newest offered version. The release listing itself cannot authorize an installer. The downloaded installer must pass the embedded minisign check.

The updater checks in the background, asks before downloading and installing, and verifies the manifest and installer. It then opens the installer with a progress window only (`/SILENT /SUPPRESSMSGBOXES /NORESTART /SP- /UPDATE=1`) and exits so the executable can be replaced. The installer restarts the resident and leaves settings in place. A manual install still uses the full setup wizard.

## Forks

Generate a new minisign key with the Tauri signer. Store only the public key, base64-encoded, in the fork's `packaging/update-public-key.txt`. Set `ORBIT_UPDATE_MANIFEST_URL`, `ORBIT_DEVELOPMENT_RELEASES_URL`, and `ORBIT_UPDATE_PUBLIC_KEY` at build time to use another feed and key. Update installer links and release scripts to point to the fork.
