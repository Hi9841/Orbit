param([Parameter(Mandatory)][string]$Archive)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $Archive))
try {
    $names = @($zip.Entries | ForEach-Object { $_.FullName })
    if ($names.Count -eq 0) { throw 'Source archive is empty' }
    foreach ($name in $names) {
        if ($name.Contains('\') -or $name.StartsWith('/') -or ($name.Split('/') -contains '..')) {
            throw "Nonportable archive path: $name"
        }
    }
    $roots = @($names | ForEach-Object { $_.Split('/')[0] } | Select-Object -Unique)
    if ($roots.Count -ne 1) { throw 'Source archive must have one root directory' }
    $prefix = $roots[0] + '/'
    foreach ($required in @('Cargo.toml','Cargo.lock','LICENSE','NOTICE.md','build.rs','src/main.rs','.cargo/config.toml','packaging/update-public-key.txt','tools/Build-Release.ps1')) {
        if ($names -notcontains ($prefix + $required)) { throw "Source archive lacks $required" }
    }
    foreach ($name in $names) {
        $relative = $name.Substring($prefix.Length)
        if ($relative -match '^([.]git/|[.]tools/|target/|dist/)' -or $relative -match '(^|/)(update-signing[.]key|task_plan[.]md|progress[.]md|findings[.]md)$') {
            throw "Private or generated workspace data in source archive: $relative"
        }
    }
    if (-not ($names | Where-Object { $_ -like ($prefix + 'vendor/*/.cargo-checksum.json') })) {
        throw 'Release source archive lacks vendored dependencies'
    }
    $reader = New-Object System.IO.StreamReader($zip.GetEntry($prefix + '.cargo/config.toml').Open())
    try { $config = $reader.ReadToEnd() } finally { $reader.Dispose() }
    if ($config -notmatch 'directory\s*=\s*"vendor"') { throw 'Vendored source configuration is not portable' }
    "status: passed`nentries: $($names.Count)`narchive: $Archive"
} finally { $zip.Dispose() }
