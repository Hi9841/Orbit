param(
    [string]$Compiler = "$PSScriptRoot\..\.tools\InnoSetup\ISCC.exe",
    [switch]$SkipSourceArchive,
    [switch]$VendorDependencies
)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path "$PSScriptRoot\..").Path
Push-Location $projectRoot
try {
    if (-not (Test-Path -LiteralPath $Compiler)) {
        $installedCompiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
        if ($installedCompiler) { $Compiler = $installedCompiler.Source }
        else {
            $standardCompiler = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
            if (Test-Path -LiteralPath $standardCompiler) { $Compiler = $standardCompiler }
            else { throw 'Install Inno Setup 6 or pass -Compiler with the path to ISCC.exe.' }
        }
    }
    $metadataText = & cargo metadata --locked --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata = $metadataText | ConvertFrom-Json
    $package = $metadata.packages | Where-Object { $_.name -eq 'orbit' -and $_.manifest_path -eq (Join-Path $projectRoot 'Cargo.toml') }
    if (-not $package) { throw 'Cannot find the Orbit package in cargo metadata' }
    $version = $package.version
    $dist = Join-Path $projectRoot 'dist'
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    $notice = New-Object System.Text.StringBuilder
    [void]$notice.AppendLine('Orbit third-party dependencies')
    [void]$notice.AppendLine('Source and license files for dependencies are available in the corresponding source archive or from the registry URL below.')
    foreach ($dependency in ($metadata.packages | Where-Object { $_.name -ne 'orbit' } | Sort-Object name,version)) {
        [void]$notice.AppendLine("`r`n==== $($dependency.name) $($dependency.version) ====")
        [void]$notice.AppendLine("License: $($dependency.license)")
        [void]$notice.AppendLine("Source: https://crates.io/crates/$($dependency.name)/$($dependency.version)")
        $dependencyRoot = Split-Path -Parent $dependency.manifest_path
        $licenseFiles = @(Get-ChildItem -LiteralPath $dependencyRoot -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)' })
        if ($dependency.license_file) {
            $declared = Join-Path $dependencyRoot $dependency.license_file
            if (Test-Path -LiteralPath $declared) { $licenseFiles += Get-Item -LiteralPath $declared }
        }
        foreach ($licenseFile in ($licenseFiles | Sort-Object FullName -Unique)) {
            [void]$notice.AppendLine("`r`n$($licenseFile.Name):")
            [void]$notice.AppendLine([System.IO.File]::ReadAllText($licenseFile.FullName))
        }
    }
    [System.IO.File]::WriteAllText((Join-Path $dist 'THIRD-PARTY-NOTICES.txt'), $notice.ToString())
    & cargo build --locked --release --bins
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & $Compiler /Q "/DAppVersion=$version" packaging/Orbit.iss
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
    $installer = Join-Path $dist "OrbitSetup-$version-x64.exe"
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText((Join-Path $dist 'update-unsigned.json'), (@{
        version = $version
        installer_url = "https://github.com/Hi9841/Orbit/releases/download/v$version/OrbitSetup-$version-x64.exe"
        sha256 = $hash
        notes = 'See the release notes for changes and known limitations.'
        signature = ''
    } | ConvertTo-Json))
    if (-not $SkipSourceArchive) {
        $stage = Join-Path $projectRoot ('.tools\source-' + [guid]::NewGuid().ToString('N'))
        $archiveRoot = Join-Path $stage "Orbit-$version"
        New-Item -ItemType Directory -Force -Path $archiveRoot | Out-Null
        # Explicit source inputs exclude private signing material, local tools, and build output.
        foreach ($item in @('Cargo.toml','Cargo.lock','build.rs','LICENSE','NOTICE.md','README.md','.gitignore','src','assets','tests','tools','packaging','docs','migration','.github','.cargo')) {
            $source = Join-Path $projectRoot $item
            if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination $archiveRoot -Recurse }
        }
        Copy-Item -LiteralPath (Join-Path $dist 'THIRD-PARTY-NOTICES.txt') -Destination $archiveRoot
        if ($VendorDependencies) {
            $vendorDirectory = Join-Path $archiveRoot 'vendor'
            $vendorConfig = & cargo vendor --locked $vendorDirectory
            if ($LASTEXITCODE -ne 0) { throw 'Dependency vendoring failed' }
            $cargoConfigDir = Join-Path $archiveRoot '.cargo'
            New-Item -ItemType Directory -Force -Path $cargoConfigDir | Out-Null
            # cargo vendor prints an absolute path; the archive needs a portable relative path.
            $portableConfig = @'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
'@
            [System.IO.File]::AppendAllText((Join-Path $cargoConfigDir 'config.toml'), "`r`n" + $portableConfig + "`r`n")
        }
        $sourceZip = Join-Path $dist "Orbit-$version-source.zip"
        if (Test-Path -LiteralPath $sourceZip) { Remove-Item -LiteralPath $sourceZip }
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $archive = [System.IO.Compression.ZipFile]::Open($sourceZip, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($file in Get-ChildItem -LiteralPath $stage -File -Recurse -Force) {
                $entryName = $file.FullName.Substring($stage.Length + 1).Replace('\', '/')
                [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $file.FullName, $entryName, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
            }
        } finally { $archive.Dispose() }
        # The stage contains only copies created by this invocation and is inside .tools.
        $allowedRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot '.tools')) + [System.IO.Path]::DirectorySeparatorChar
        if (-not ([System.IO.Path]::GetFullPath($stage).StartsWith($allowedRoot, [System.StringComparison]::OrdinalIgnoreCase))) { throw 'Unexpected source staging path' }
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    $releaseFiles = @($installer, (Join-Path $dist 'THIRD-PARTY-NOTICES.txt'))
    if (-not $SkipSourceArchive) { $releaseFiles += $sourceZip }
    $checksums = $releaseFiles | ForEach-Object { Get-Item -LiteralPath $_ } | Sort-Object Name | ForEach-Object {
        '{0}  {1}' -f (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant(), $_.Name
    }
    [System.IO.File]::WriteAllLines((Join-Path $dist 'SHA256SUMS.txt'), $checksums)
    "status: built`nversion: $version`ninstaller: $installer`nsha256: $hash"
} finally {
    Pop-Location
}
