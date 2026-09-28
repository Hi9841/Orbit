# Build an Orbit release

## Build the files

1. Update `package.version` in `Cargo.toml` using the intended release version.
2. Run `cargo test --locked --all-targets` and `cargo clippy --locked --all-targets -- -D warnings`. Ignored desktop tests remain opt-in.
3. Run `./tools/Build-Release.ps1 -VendorDependencies`.
4. Review `dist/OrbitSetup-VERSION-x64.exe`, `dist/Orbit-VERSION-source.zip`, `dist/SHA256SUMS.txt`, and `dist/THIRD-PARTY-NOTICES.txt`.
5. Record outstanding desktop checks in the release notes. Use a prerelease while those checks remain incomplete.

The archive generator copies explicit source directories. It excludes `.tools`, `.git`, build output, and private signing files. Vendored dependencies include their original license files. Keep the source archive beside every published installer.

## Sign update metadata

`packaging/update-public-key.txt` contains the public update verification key. The private seed must stay outside the repository. Keep an offline backup. Losing it prevents future releases from being accepted by installed versions using that public key.

The release helper reads the base64 seed from `ORBIT_UPDATE_SIGNING_SEED`. Set it only in the publishing process. Do not print it or put it in command arguments, source files, logs, or build artifacts.

```powershell
$env:ORBIT_UPDATE_SIGNING_SEED = [System.IO.File]::ReadAllText($privateSeedPath).Trim()
try {
    ./target/release/orbit-release.exe sign dist/update-unsigned.json dist/update.json
    if ($LASTEXITCODE -ne 0) { throw 'Manifest signing failed' }
} finally {
    Remove-Item Env:ORBIT_UPDATE_SIGNING_SEED -ErrorAction SilentlyContinue
}
```

Edit the unsigned manifest's release notes before signing. The signer refuses to replace an existing output file. Regenerate metadata whenever the installer changes.

Verify the actual installer before publishing:

```powershell
./target/release/orbit-release.exe verify dist/update.json dist/OrbitSetup-VERSION-x64.exe
```

This checks the signature against Orbit's embedded public key and hashes the complete installer. Signing also refuses a seed that does not match that public key.

The signed payload is compact UTF-8 JSON with fields in this exact order: `format`, `version`, `installer_url`, `sha256`, `notes`. `format` is `orbit-update-v1`. The signature is base64 Ed25519 over those bytes.

## Publish

Create a release tagged `vVERSION` in [Hi9841/Orbit](https://github.com/Hi9841/Orbit). Attach the installer, source ZIP, dependency notices, checksum file, and signed `update.json`.

By default, the application checks `https://github.com/Hi9841/Orbit/releases/latest/download/update.json`. GitHub excludes prereleases from this endpoint. Stable-only checks also reject prerelease versions in update manifests. HTTP 404 means no stable update is available yet; other network errors are reported for manual checks and do not prevent window management.

With "Include development versions" enabled, Orbit reads the repository's release listing and verifies signed `update.json` assets before choosing the newest offered version. The release listing itself cannot authorize an installer. Every accepted manifest must pass the same embedded-key signature check.

The updater checks in the background, asks before downloading and installing, verifies the manifest and installer, and launches the normal installer. The installer closes the previous resident instance and restarts Orbit for `/UPDATE=1` upgrades.

## Forks

Generate a new key using `orbit-release keygen PRIVATE_FILE`. Store only its printed public key in the fork. Set `ORBIT_UPDATE_MANIFEST_URL`, `ORBIT_DEVELOPMENT_RELEASES_URL`, and `ORBIT_UPDATE_PUBLIC_KEY` at build time to use another feed and key. Update installer links and release scripts to point to the fork.
