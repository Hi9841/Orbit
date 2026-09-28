use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use orbit::update::Manifest;
use rand_core::OsRng;
use std::{fs, io::Write, path::Path};

fn main() {
    if let Err(error) = run() {
        println!("error: {error}\nhelp: run orbit-release --help");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 1 && ["--version", "-v", "-V"].contains(&args[0].as_str()) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args == ["--help"] {
        println!(
            "Orbit release signing\nUsage:\n  orbit-release keygen SECRET_FILE\n  orbit-release sign INPUT_MANIFEST OUTPUT_MANIFEST\n  orbit-release verify MANIFEST INSTALLER\nThe sign command reads ORBIT_UPDATE_SIGNING_SEED as a base64 32-byte seed. Verification uses the embedded public key."
        );
        return Ok(());
    }
    match args.as_slice() {
        [command, path] if command == "keygen" => keygen(Path::new(path)),
        [command, input, output] if command == "sign" => sign(Path::new(input), Path::new(output)),
        [command, manifest, installer] if command == "verify" => {
            verify(Path::new(manifest), Path::new(installer))
        }
        _ => Err("expected keygen SECRET_FILE, sign INPUT_MANIFEST OUTPUT_MANIFEST, or verify MANIFEST INSTALLER".into()),
    }
}

fn keygen(path: &Path) -> Result<(), String> {
    let key = SigningKey::generate(&mut OsRng);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("cannot create secret file: {e}"))?;
    file.write_all(STANDARD.encode(key.to_bytes()).as_bytes())
        .map_err(|e| format!("cannot write secret: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("cannot flush secret: {e}"))?;
    println!(
        "status: key_created\npublic_key: {}",
        STANDARD.encode(key.verifying_key().to_bytes())
    );
    Ok(())
}

fn sign(input: &Path, output: &Path) -> Result<(), String> {
    let seed = std::env::var("ORBIT_UPDATE_SIGNING_SEED")
        .map_err(|_| "ORBIT_UPDATE_SIGNING_SEED is not set")?;
    let bytes = STANDARD
        .decode(seed.trim())
        .map_err(|_| "signing seed is not valid base64")?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "signing seed must be 32 bytes")?;
    let key = SigningKey::from_bytes(&seed);
    let expected = orbit::update::configured()
        .ok_or("release verification key is not configured")?
        .1;
    if STANDARD.encode(key.verifying_key().to_bytes()) != expected {
        return Err("signing seed does not match the public key embedded in Orbit".into());
    }
    let data = fs::read(input).map_err(|e| format!("cannot read manifest: {e}"))?;
    let mut manifest: Manifest =
        serde_json::from_slice(&data).map_err(|e| format!("invalid manifest: {e}"))?;
    manifest.signature = STANDARD.encode(key.sign(&manifest.signed_payload()).to_bytes());
    manifest.verify(
        &STANDARD.encode(key.verifying_key().to_bytes()),
        env!("CARGO_PKG_VERSION"),
    )?;
    let serialized = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("cannot stage signed manifest: {e}"))?;
    staged
        .write_all(&serialized)
        .map_err(|e| format!("cannot write signed manifest: {e}"))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|e| format!("cannot flush signed manifest: {e}"))?;
    staged
        .persist_noclobber(output)
        .map_err(|e| format!("cannot create signed manifest without replacing a file: {e}"))?;
    println!(
        "status: signed\nversion: {}\noutput: {}",
        manifest.version,
        output.display()
    );
    Ok(())
}

fn verify(manifest_path: &Path, installer: &Path) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let data = fs::read(manifest_path).map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    let (_, key) =
        orbit::update::configured().ok_or("release verification key is not configured")?;
    manifest.verify_channel(key, "0.0.0", true)?;
    let mut file = fs::File::open(installer).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if hex::encode(digest.finalize()) != manifest.sha256.to_ascii_lowercase() {
        return Err("installer does not match signed metadata".into());
    }
    println!("status: verified\nversion: {}", manifest.version);
    Ok(())
}
