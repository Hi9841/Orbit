use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_INSTALLER_BYTES: u64 = 250 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub installer_url: String,
    pub sha256: String,
    pub notes: String,
    pub signature: String,
}

impl Manifest {
    pub fn signed_payload(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct Payload<'a> {
            format: &'static str,
            version: &'a str,
            installer_url: &'a str,
            sha256: &'a str,
            notes: &'a str,
        }
        serde_json::to_vec(&Payload {
            format: "orbit-update-v1",
            version: &self.version,
            installer_url: &self.installer_url,
            sha256: &self.sha256,
            notes: &self.notes,
        })
        .expect("update payload serialization must succeed")
    }

    pub fn verify(&self, public_key_base64: &str, current_version: &str) -> Result<bool, String> {
        self.verify_channel(public_key_base64, current_version, false)
    }

    pub fn verify_channel(
        &self,
        public_key_base64: &str,
        current_version: &str,
        include_development: bool,
    ) -> Result<bool, String> {
        let key_bytes = STANDARD
            .decode(public_key_base64)
            .map_err(|_| "invalid update public key")?;
        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| "invalid update public key length")?;
        let key = VerifyingKey::from_bytes(&key_array).map_err(|_| "invalid update public key")?;
        let signature_bytes = STANDARD
            .decode(&self.signature)
            .map_err(|_| "invalid update signature")?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| "invalid update signature length")?;
        key.verify_strict(&self.signed_payload(), &signature)
            .map_err(|_| "update manifest signature does not match")?;
        validate_url(&self.installer_url)?;
        if self.sha256.len() != 64 || hex::decode(&self.sha256).is_err() {
            return Err("invalid installer SHA-256".into());
        }
        let offered = Version::parse(&self.version).map_err(|_| "invalid update version")?;
        let current = Version::parse(current_version).map_err(|_| "invalid current version")?;
        if !include_development && !offered.pre.is_empty() && current.pre.is_empty() {
            return Ok(false);
        }
        Ok(offered > current)
    }
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

pub fn check() -> Result<Option<Manifest>, String> {
    let include_development = crate::settings::Settings::load()?.include_development_versions;
    check_channel(include_development)
}

pub fn check_channel(include_development: bool) -> Result<Option<Manifest>, String> {
    let Some((url, key)) = configured() else {
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
            .filter(|asset| asset.name == "update.json")
            .take(5)
        {
            if let Some(manifest) = fetch_manifest(&asset.browser_download_url, key, true)?
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
    fetch_manifest(url, key, include_development)
}

fn fetch_manifest(
    url: &str,
    key: &str,
    include_development: bool,
) -> Result<Option<Manifest>, String> {
    validate_url(url)?;
    let response = match client().get(url).timeout(Duration::from_secs(30)).call() {
        Ok(response) => response,
        // GitHub has no latest-release asset until the first stable release exists.
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
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid update manifest: {e}"))?;
    if manifest.verify_channel(key, env!("CARGO_PKG_VERSION"), include_development)? {
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
        key,
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
    let mut temp = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| format!("cannot stage update: {e}"))?;
    let mut response = client()
        .get(&manifest.installer_url)
        .call()
        .map_err(|e| format!("cannot download update: {e}"))?
        .into_reader();
    copy_verified(
        &mut response,
        &mut temp,
        &manifest.sha256,
        MAX_INSTALLER_BYTES,
    )?;
    temp.as_file()
        .sync_all()
        .map_err(|e| format!("cannot flush update: {e}"))?;
    let destination = directory.join(format!("OrbitSetup-{}-x64.exe", manifest.version));
    temp.persist(&destination)
        .map_err(|e| format!("cannot store update: {e}"))?;
    Ok(destination)
}

fn copy_verified(
    source: &mut impl Read,
    target: &mut impl Write,
    expected: &str,
    limit: u64,
) -> Result<(), String> {
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
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
        digest.update(&buffer[..size]);
        target
            .write_all(&buffer[..size])
            .map_err(|e| format!("cannot stage update: {e}"))?;
    }
    if total == 0 {
        return Err("update installer is empty".into());
    }
    if hex::encode(digest.finalize()) != expected.to_ascii_lowercase() {
        return Err("update installer checksum does not match".into());
    }
    Ok(())
}

/// Recheck the exact staged file immediately before handing it to Windows.
pub fn verify_download(path: &Path, manifest: &Manifest) -> Result<(), String> {
    let (_, key) = configured().ok_or("updates are not configured")?;
    if !manifest.verify_channel(
        key,
        env!("CARGO_PKG_VERSION"),
        crate::settings::Settings::load()?.include_development_versions,
    )? {
        return Err("update is not newer".into());
    }
    let mut source =
        fs::File::open(path).map_err(|error| format!("cannot open staged update: {error}"))?;
    copy_verified(
        &mut source,
        &mut std::io::sink(),
        &manifest.sha256,
        MAX_INSTALLER_BYTES,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_manifest() -> (Manifest, String) {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut manifest = Manifest {
            version: "9.0.0".into(),
            installer_url: "https://example.com/OrbitSetup.exe".into(),
            sha256: "a".repeat(64),
            notes: "Changes".into(),
            signature: String::new(),
        };
        manifest.signature = STANDARD.encode(key.sign(&manifest.signed_payload()).to_bytes());
        (manifest, STANDARD.encode(key.verifying_key().to_bytes()))
    }

    #[test]
    fn verifies_newer_signed_release() {
        let (manifest, key) = signed_manifest();
        assert!(manifest.verify(&key, "0.1.0").unwrap());
        assert!(!manifest.verify(&key, "9.0.0").unwrap());
    }

    #[test]
    fn rejects_tampered_download_location() {
        let (mut manifest, key) = signed_manifest();
        manifest.installer_url = "https://attacker.example/installer.exe".into();
        assert!(manifest.verify(&key, "0.1.0").is_err());
    }

    #[test]
    fn rejects_unsigned_or_tampered_metadata_and_wrong_key() {
        let (original, key) = signed_manifest();
        for field in 0..4 {
            let mut manifest = original.clone();
            match field {
                0 => manifest.version = "10.0.0".into(),
                1 => manifest.sha256 = "b".repeat(64),
                2 => manifest.notes.push_str(" changed"),
                _ => manifest.signature.clear(),
            }
            assert!(manifest.verify(&key, "0.1.0").is_err());
        }
        assert!(
            original
                .verify(&STANDARD.encode([4u8; 32]), "0.1.0")
                .is_err()
        );
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
    fn installer_stream_enforces_hash_size_and_nonempty() {
        let payload = b"disposable installer bytes";
        let expected = hex::encode(Sha256::digest(payload));
        let mut destination = Vec::new();
        copy_verified(&mut &payload[..], &mut destination, &expected, 1024).unwrap();
        assert_eq!(destination, payload);
        assert!(copy_verified(&mut &payload[..], &mut Vec::new(), &expected, 3).is_err());
        assert!(copy_verified(&mut &payload[..], &mut Vec::new(), &"0".repeat(64), 1024).is_err());
        assert!(
            copy_verified(
                &mut &b""[..],
                &mut Vec::new(),
                &hex::encode(Sha256::digest(b"")),
                1024
            )
            .is_err()
        );
    }

    #[test]
    fn stable_build_does_not_offer_prerelease() {
        let (mut manifest, public_key) = signed_manifest();
        manifest.version = "10.0.0-beta.1".into();
        manifest.signature = STANDARD.encode(
            SigningKey::from_bytes(&[7u8; 32])
                .sign(&manifest.signed_payload())
                .to_bytes(),
        );
        assert!(!manifest.verify(&public_key, "0.1.0").unwrap());
        assert!(manifest.verify_channel(&public_key, "0.1.0", true).unwrap());
        assert!(manifest.verify(&public_key, "0.2.0-beta.1").unwrap());
    }
}
