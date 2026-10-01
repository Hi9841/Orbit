use base64::{Engine, engine::general_purpose::STANDARD};
use semver::Version;
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_INSTALLER_BYTES: u64 = 250 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Manifest {
    pub version: String,
    pub installer_url: String,
    pub notes: String,
    pub signature: String,
}

impl Manifest {
    pub fn verify(&self, current_version: &str) -> Result<bool, String> {
        self.verify_channel(current_version, false)
    }

    pub fn verify_channel(
        &self,
        current_version: &str,
        include_development: bool,
    ) -> Result<bool, String> {
        validate_url(&self.installer_url)?;
        if self.signature.trim().is_empty() {
            return Err("update signature is missing".into());
        }
        let offered = Version::parse(self.version.trim().trim_start_matches('v'))
            .map_err(|_| "invalid update version")?;
        let current = Version::parse(current_version.trim().trim_start_matches('v'))
            .map_err(|_| "invalid current version")?;
        if !include_development && !offered.pre.is_empty() && current.pre.is_empty() {
            return Ok(false);
        }
        Ok(offered > current)
    }
}

/// Verify installer bytes the way Tauri does: base64 public key, base64 `.sig`
/// body, then minisign over those bytes. A trusted `version:` comment must
/// match the announced version.
pub fn verify_installer(
    bytes: &[u8],
    signature_base64: &str,
    public_key_base64: &str,
    announced_version: &str,
) -> Result<(), String> {
    let public_text = decode_base64_text(public_key_base64, "invalid update public key")?;
    let public_key = minisign_verify::PublicKey::decode(&public_text)
        .map_err(|_| "invalid update public key")?;
    let signature_text = decode_base64_text(signature_base64, "invalid update signature")?;
    let signature = minisign_verify::Signature::decode(&signature_text)
        .map_err(|_| "invalid update signature")?;
    public_key
        .verify(bytes, &signature, true)
        .map_err(|_| "update signature does not match the installer")?;
    if let Some(signed) = signed_version(signature.trusted_comment()) {
        let matches = match (
            Version::parse(signed.trim().trim_start_matches('v')),
            Version::parse(announced_version.trim().trim_start_matches('v')),
        ) {
            (Ok(signed), Ok(announced)) => signed == announced,
            _ => signed == announced_version,
        };
        if !matches {
            return Err("signed installer version does not match the update".into());
        }
    }
    Ok(())
}

fn signed_version(trusted_comment: &str) -> Option<&str> {
    trusted_comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
}

fn decode_base64_text(value: &str, label: &str) -> Result<String, String> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| label.to_string())?;
    String::from_utf8(bytes).map_err(|_| label.to_string())
}

#[derive(Deserialize)]
struct LatestFile {
    version: String,
    #[serde(default)]
    notes: String,
    platforms: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct PlatformAsset {
    signature: String,
    url: String,
}

/// Prism and HyperType publish the NSIS installer on both Windows keys.
/// Prefer the NSIS entry, the same choice as HyperType's updater.
pub fn manifest_from_latest_json(bytes: &[u8]) -> Result<Option<Manifest>, String> {
    let latest: LatestFile = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid update manifest: {error}"))?;
    let asset = latest
        .platforms
        .get("windows-x86_64-nsis")
        .or_else(|| latest.platforms.get("windows-x86_64"));
    let Some(asset) = asset else {
        return Ok(None);
    };
    let asset: PlatformAsset =
        serde_json::from_value(asset.clone()).map_err(|_| "invalid Windows update entry")?;
    if asset.signature.trim().is_empty() || asset.url.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Manifest {
        version: latest.version,
        installer_url: asset.url,
        notes: latest.notes,
        signature: asset.signature,
    }))
}

pub fn pack_latest_json(version: &str, signature: &str, pub_date: &str) -> Result<Vec<u8>, String> {
    let version = version.trim().trim_start_matches('v');
    if Version::parse(version).is_err() {
        return Err("invalid update version".into());
    }
    if signature.trim().is_empty() {
        return Err("update signature is missing".into());
    }
    if pub_date.len() != 20
        || !pub_date.ends_with('Z')
        || pub_date.as_bytes().get(10) != Some(&b'T')
    {
        return Err("pub_date must look like 2026-10-01T00:00:00Z".into());
    }
    let url = format!(
        "https://github.com/Hi9841/Orbit/releases/download/v{version}/OrbitSetup-{version}-x64.exe"
    );
    validate_url(&url)?;
    let platform = serde_json::json!({
        "signature": signature.trim(),
        "url": url,
    });
    let body = serde_json::json!({
        "version": version,
        "notes": format!("Orbit {version}"),
        "pub_date": pub_date,
        "platforms": {
            "windows-x86_64": platform,
            "windows-x86_64-nsis": platform,
        }
    });
    serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())
}

pub fn configured() -> Option<(&'static str, &'static str)> {
    let url = option_env!("ORBIT_UPDATE_MANIFEST_URL")?;
    let key = option_env!("ORBIT_UPDATE_PUBLIC_KEY")?;
    if validate_url(url).is_err() || key.is_empty() {
        return None;
    }
    Some((url, key))
}

fn validate_url(value: &str) -> Result<(), String> {
    let parsed = url::Url::parse(value).map_err(|_| "invalid update URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err("update URLs must use HTTPS without credentials or fragments".into());
    }
    Ok(())
}

fn client() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .https_only(true)
        .redirects(5)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .timeout(Duration::from_secs(15 * 60))
        .user_agent(concat!("Orbit/", env!("CARGO_PKG_VERSION")))
        .build()
}

pub fn version_is_newer(offered: &str, current: &str) -> Result<bool, String> {
    let offered = offered.trim().trim_start_matches('v');
    let current = current.trim().trim_start_matches('v');
    let offered = Version::parse(offered).map_err(|_| "invalid update version".to_string())?;
    let current = Version::parse(current).map_err(|_| "invalid current version".to_string())?;
    Ok(offered > current)
}

#[derive(Clone, Debug)]
pub enum UpdateReport {
    UpToDate,
    Available {
        version: String,
        manifest: Option<Manifest>,
    },
    Failed(String),
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

static LAST_LINE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn remember_status(text: &str) {
    if let Ok(mut slot) = LAST_LINE.lock() {
        *slot = text.to_string();
    }
}

pub fn remembered_status() -> String {
    LAST_LINE
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default()
}

pub fn report() -> UpdateReport {
    let current = current_version();
    let latest = match latest_stable_version() {
        Ok(version) => version,
        Err(error) => return UpdateReport::Failed(error),
    };
    let newer = match version_is_newer(&latest, current) {
        Ok(newer) => newer,
        Err(error) => return UpdateReport::Failed(error),
    };
    if !newer {
        return UpdateReport::UpToDate;
    }
    let manifest = match configured() {
        Some(_) => {
            let asset =
                format!("https://github.com/Hi9841/Orbit/releases/download/v{latest}/latest.json");
            match fetch_manifest(&asset, true) {
                Ok(manifest) => manifest,
                Err(error) => return UpdateReport::Failed(error),
            }
        }
        None => None,
    };
    UpdateReport::Available {
        version: latest,
        manifest,
    }
}

fn latest_stable_version() -> Result<String, String> {
    let url = latest_release_api();
    validate_url(&url)?;
    #[derive(Deserialize)]
    struct Latest {
        tag_name: String,
        draft: bool,
        prerelease: bool,
    }
    let response = client()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|error| format!("update check failed: {error}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(256 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read release: {error}"))?;
    let latest: Latest =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid release: {error}"))?;
    if latest.draft || latest.prerelease {
        return Err("latest GitHub release is not a stable version".into());
    }
    let version = latest.tag_name.trim().trim_start_matches('v').to_string();
    Version::parse(&version).map_err(|_| "invalid update version".to_string())?;
    Ok(version)
}

fn latest_release_api() -> String {
    let feed = option_env!("ORBIT_DEVELOPMENT_RELEASES_URL").unwrap_or("");
    if let Some(base) = feed.split('?').next()
        && let Some(prefix) = base.strip_suffix("/releases")
    {
        return format!("{prefix}/releases/latest");
    }
    "https://api.github.com/repos/Hi9841/Orbit/releases/latest".into()
}

pub fn check() -> Result<Option<Manifest>, String> {
    let include_development = crate::settings::Settings::load()?.include_development_versions;
    check_channel(include_development)
}

pub fn check_channel(include_development: bool) -> Result<Option<Manifest>, String> {
    let Some((url, _)) = configured() else {
        return Ok(None);
    };
    if include_development && let Some(feed) = option_env!("ORBIT_DEVELOPMENT_RELEASES_URL") {
        #[derive(Deserialize)]
        struct Release {
            draft: bool,
            assets: Vec<Asset>,
        }
        #[derive(Deserialize)]
        struct Asset {
            name: String,
            browser_download_url: String,
        }
        validate_url(feed)?;
        let response = client()
            .get(feed)
            .timeout(Duration::from_secs(30))
            .call()
            .map_err(|e| format!("development update check failed: {e}"))?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("release listing is too large".into());
        }
        let releases: Vec<Release> =
            serde_json::from_slice(&bytes).map_err(|e| format!("invalid release listing: {e}"))?;
        let mut best: Option<Manifest> = None;
        for asset in releases
            .into_iter()
            .filter(|release| !release.draft)
            .flat_map(|release| release.assets)
            .filter(|asset| asset.name == "latest.json")
            .take(5)
        {
            if let Some(manifest) = fetch_manifest(&asset.browser_download_url, true)?
                && best.as_ref().is_none_or(|old| {
                    Version::parse(&manifest.version).unwrap()
                        > Version::parse(&old.version).unwrap()
                })
            {
                best = Some(manifest);
            }
        }
        return Ok(best);
    }
    fetch_manifest(url, include_development)
}

fn fetch_manifest(url: &str, include_development: bool) -> Result<Option<Manifest>, String> {
    validate_url(url)?;
    let response = match client().get(url).timeout(Duration::from_secs(30)).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(error) => return Err(format!("update check failed: {error}")),
    };
    let mut response = response.into_reader();
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read update manifest: {e}"))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("update manifest is too large".into());
    }
    let Some(manifest) = manifest_from_latest_json(&bytes)? else {
        return Ok(None);
    };
    if manifest.verify_channel(env!("CARGO_PKG_VERSION"), include_development)? {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

pub fn download(manifest: &Manifest) -> Result<PathBuf, String> {
    let Some((_, key)) = configured() else {
        return Err("updates are not configured".into());
    };
    if !manifest.verify_channel(
        env!("CARGO_PKG_VERSION"),
        crate::settings::Settings::load()?.include_development_versions,
    )? {
        return Err("update is not newer".into());
    }
    let directory = crate::settings::Settings::path()?
        .parent()
        .ok_or("settings path has no parent")?
        .join("Updates");
    fs::create_dir_all(&directory).map_err(|e| format!("cannot create update directory: {e}"))?;
    let mut response = client()
        .get(&manifest.installer_url)
        .call()
        .map_err(|e| format!("cannot download update: {e}"))?
        .into_reader();
    let bytes = read_capped(&mut response, MAX_INSTALLER_BYTES)?;
    verify_installer(&bytes, &manifest.signature, key, &manifest.version)?;
    let mut temp = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| format!("cannot stage update: {e}"))?;
    temp.write_all(&bytes)
        .map_err(|e| format!("cannot stage update: {e}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("cannot flush update: {e}"))?;
    let destination = directory.join(format!("OrbitSetup-{}-x64.exe", manifest.version));
    temp.persist(&destination)
        .map_err(|e| format!("cannot store update: {e}"))?;
    Ok(destination)
}

fn read_capped(source: &mut impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut out = Vec::new();
    loop {
        let size = source
            .read(&mut buffer)
            .map_err(|e| format!("update download failed: {e}"))?;
        if size == 0 {
            break;
        }
        total += size as u64;
        if total > limit {
            return Err("update installer is too large".into());
        }
        out.extend_from_slice(&buffer[..size]);
    }
    if total == 0 {
        return Err("update installer is empty".into());
    }
    Ok(out)
}

/// Inno arguments for an in-place update.
///
/// This matches Prism's Windows updater: a progress window and no wizard
/// (`/SILENT`, the Inno equivalent of NSIS `/P`), no extra prompts, no reboot,
/// and `/UPDATE=1` so setup restarts the resident and keeps settings.
/// The running app must exit after launching setup so Windows can replace it.
pub fn passive_installer_args() -> &'static str {
    "/SILENT /SUPPRESSMSGBOXES /NORESTART /SP- /UPDATE=1"
}

/// Recheck the exact staged file immediately before handing it to Windows.
pub fn verify_download(path: &Path, manifest: &Manifest) -> Result<(), String> {
    let (_, key) = configured().ok_or("updates are not configured")?;
    if !manifest.verify_channel(
        env!("CARGO_PKG_VERSION"),
        crate::settings::Settings::load()?.include_development_versions,
    )? {
        return Err("update is not newer".into());
    }
    let mut source =
        fs::File::open(path).map_err(|error| format!("cannot open staged update: {error}"))?;
    let bytes = read_capped(&mut source, MAX_INSTALLER_BYTES)?;
    verify_installer(&bytes, &manifest.signature, key, &manifest.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_key_and_signature() -> (String, String) {
        let public = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3\n";
        let signature = "untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==\n";
        (STANDARD.encode(public), STANDARD.encode(signature))
    }

    fn offer(version: &str, signature: &str) -> Manifest {
        Manifest {
            version: version.into(),
            installer_url:
                "https://github.com/Hi9841/Orbit/releases/download/v9.0.0/OrbitSetup-9.0.0-x64.exe"
                    .into(),
            notes: "Changes".into(),
            signature: signature.into(),
        }
    }

    #[test]
    fn passive_update_uses_progress_only_and_restarts() {
        let args = passive_installer_args();
        assert!(args.contains("/SILENT"));
        assert!(args.contains("/SUPPRESSMSGBOXES"));
        assert!(args.contains("/NORESTART"));
        assert!(args.contains("/SP-"));
        assert!(args.contains("/UPDATE=1"));
        assert!(!args.contains("/VERYSILENT"));
    }

    #[test]
    fn version_comparison_ignores_a_leading_v_and_rejects_junk() {
        assert!(version_is_newer("0.2.5", "0.2.4").unwrap());
        assert!(!version_is_newer("v0.2.4", "0.2.4").unwrap());
        assert!(version_is_newer("v1.0.0", "v0.9.9").unwrap());
        assert!(version_is_newer("nope", "0.2.4").is_err());
    }

    #[test]
    fn minisign_accepts_the_installer_bytes_and_rejects_a_change() {
        let (key, signature) = fixture_key_and_signature();
        verify_installer(b"test", &signature, &key, "9.0.0").unwrap();
        assert!(verify_installer(b"Test", &signature, &key, "9.0.0").is_err());
        assert!(verify_installer(b"test", &signature, &STANDARD.encode("nope"), "9.0.0").is_err());
    }

    #[test]
    fn signed_version_comment_must_match_the_announcement() {
        assert_eq!(
            signed_version("timestamp:1\tfile:setup.exe\tversion:1.2.3"),
            Some("1.2.3")
        );
        assert_eq!(signed_version("timestamp:1\tfile:setup.exe"), None);
    }

    #[test]
    fn latest_json_prefers_the_nsis_signature_and_url() {
        let raw = r#"{
            "version": "9.0.0",
            "notes": "Orbit 9.0.0",
            "pub_date": "2026-10-01T00:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "generic",
                    "url": "https://github.com/Hi9841/Orbit/releases/download/v9.0.0/generic.exe"
                },
                "windows-x86_64-nsis": {
                    "signature": "nsis-sig",
                    "url": "https://github.com/Hi9841/Orbit/releases/download/v9.0.0/OrbitSetup-9.0.0-x64.exe"
                }
            }
        }"#;
        let manifest = manifest_from_latest_json(raw.as_bytes()).unwrap().unwrap();
        assert_eq!(manifest.signature, "nsis-sig");
        assert!(manifest.installer_url.ends_with("OrbitSetup-9.0.0-x64.exe"));
        assert!(manifest.verify("0.2.10").unwrap());
        assert!(!manifest.verify("9.0.0").unwrap());
    }

    #[test]
    fn latest_json_without_a_windows_signature_is_not_installable() {
        let raw = r#"{"version":"9.0.0","notes":"","platforms":{"darwin-aarch64":{"signature":"x","url":"https://example.com/a"}}}"#;
        assert!(manifest_from_latest_json(raw.as_bytes()).unwrap().is_none());
        let empty = r#"{"version":"9.0.0","notes":"","platforms":{"windows-x86_64-nsis":{"signature":"","url":""}}}"#;
        assert!(
            manifest_from_latest_json(empty.as_bytes())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn packed_latest_json_matches_the_prism_shape() {
        let (key, signature) = fixture_key_and_signature();
        let bytes = pack_latest_json("0.2.11", &signature, "2026-10-01T00:00:00Z").unwrap();
        let manifest = manifest_from_latest_json(&bytes).unwrap().unwrap();
        assert_eq!(manifest.version, "0.2.11");
        assert_eq!(manifest.signature, signature);
        assert_eq!(
            manifest.installer_url,
            "https://github.com/Hi9841/Orbit/releases/download/v0.2.11/OrbitSetup-0.2.11-x64.exe"
        );
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["pub_date"], "2026-10-01T00:00:00Z");
        assert_eq!(
            parsed["platforms"]["windows-x86_64"]["signature"],
            signature
        );
        assert_eq!(
            parsed["platforms"]["windows-x86_64-nsis"]["url"],
            manifest.installer_url
        );
        verify_installer(b"test", &manifest.signature, &key, &manifest.version).unwrap();
    }

    #[test]
    fn stable_build_does_not_offer_prerelease() {
        let manifest = offer("10.0.0-beta.1", "c2ln");
        assert!(!manifest.verify("0.1.0").unwrap());
        assert!(manifest.verify_channel("0.1.0", true).unwrap());
        assert!(manifest.verify("0.2.0-beta.1").unwrap());
    }

    #[test]
    fn rejects_an_unsafe_installer_url_and_a_missing_signature() {
        let mut manifest = offer("9.0.0", "c2ln");
        manifest.installer_url = "http://example.com/a".into();
        assert!(manifest.verify("0.1.0").is_err());
        manifest.installer_url =
            "https://github.com/Hi9841/Orbit/releases/download/v9.0.0/OrbitSetup.exe".into();
        manifest.signature.clear();
        assert!(manifest.verify("0.1.0").is_err());
    }

    #[test]
    fn update_transport_rejects_unsafe_urls() {
        for value in [
            "http://example.com/a",
            "file:///C:/a.exe",
            "https://name:secret@example.com/a",
            "https://example.com/a#fragment",
            "https://",
        ] {
            assert!(validate_url(value).is_err(), "{value}");
        }
        assert!(
            validate_url("https://github.com/owner/repo/releases/download/v1/setup.exe").is_ok()
        );
    }

    #[test]
    fn installer_stream_rejects_empty_and_oversized_payloads() {
        let payload = b"disposable installer bytes";
        assert_eq!(read_capped(&mut &payload[..], 1024).unwrap(), payload);
        assert!(read_capped(&mut &payload[..], 3).is_err());
        assert!(read_capped(&mut &b""[..], 1024).is_err());
    }
}
