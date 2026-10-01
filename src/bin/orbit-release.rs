use orbit::update::{self, manifest_from_latest_json};
use std::{fs, path::Path};

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
            "Orbit release signing\nUsage:\n  orbit-release pack VERSION SIGNATURE_FILE OUTPUT PUB_DATE\n  orbit-release verify LATEST_JSON INSTALLER\npack writes latest.json in the Prism and HyperType shape. verify checks the installer bytes with the embedded minisign public key. Sign the installer with the Tauri signer. Do not print the private key."
        );
        return Ok(());
    }
    match args.as_slice() {
        [command, version, signature, output, pub_date] if command == "pack" => {
            pack(version, Path::new(signature), Path::new(output), pub_date)
        }
        [command, manifest, installer] if command == "verify" => {
            verify(Path::new(manifest), Path::new(installer))
        }
        _ => Err(
            "expected pack VERSION SIGNATURE_FILE OUTPUT PUB_DATE, or verify LATEST_JSON INSTALLER"
                .into(),
        ),
    }
}

fn pack(version: &str, signature_file: &Path, output: &Path, pub_date: &str) -> Result<(), String> {
    let signature = fs::read_to_string(signature_file)
        .map_err(|error| format!("cannot read signature: {error}"))?;
    let bytes = update::pack_latest_json(version, signature.trim(), pub_date)?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create output directory: {error}"))?;
    }
    fs::write(output, bytes).map_err(|error| format!("cannot write latest.json: {error}"))?;
    println!(
        "status: packed\nversion: {version}\noutput: {}",
        output.display()
    );
    Ok(())
}

fn verify(manifest_path: &Path, installer: &Path) -> Result<(), String> {
    let data = fs::read(manifest_path).map_err(|error| error.to_string())?;
    let manifest =
        manifest_from_latest_json(&data)?.ok_or("latest.json has no signed Windows installer")?;
    let (_, key) = update::configured().ok_or("release verification key is not configured")?;
    let bytes = fs::read(installer).map_err(|error| format!("cannot read installer: {error}"))?;
    if bytes.is_empty() {
        return Err("installer is empty".into());
    }
    update::verify_installer(&bytes, &manifest.signature, key, &manifest.version)?;
    println!("status: verified\nversion: {}", manifest.version);
    Ok(())
}
