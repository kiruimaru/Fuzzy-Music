param([switch]$SkipBuild, [string]$OutputDirectory = 'artifacts/windows-alpha')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath $root)) { throw 'Repository missing' }
if (-not $SkipBuild) {
    & npm.cmd run tauri -- build --no-bundle
    if ($LASTEXITCODE) { throw 'Tauri release build failed' }
}
$release = Join-Path $root $OutputDirectory
New-Item -ItemType Directory -Force $release | Out-Null
$stage = Join-Path $release 'Fuzzy Music 0.1.0-alpha.1 Windows x64'
if (Test-Path -LiteralPath $stage) { throw 'Staging directory exists; preserve/rename it before packaging again.' }
New-Item -ItemType Directory $stage | Out-Null
Copy-Item (Join-Path $root 'scripts/setup-windows-runtime.ps1') $stage
Copy-Item (Join-Path $root 'target/release/fuzzy-shell.exe') (Join-Path $stage 'Fuzzy Music.exe')
Copy-Item (Join-Path $root 'LICENSE') $stage
Copy-Item (Join-Path $root 'docs/windows-alpha.md') (Join-Path $stage 'README.md')
Copy-Item (Join-Path $root 'docs/third-party-windows.md') (Join-Path $stage 'THIRD-PARTY.md')
$notices = Join-Path $stage 'licenses/npm'
New-Item -ItemType Directory -Force $notices | Out-Null
foreach ($file in (Get-ChildItem -LiteralPath (Join-Path $root 'node_modules') -Recurse -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)(\..*)?$' })) {
    $relative = $file.FullName.Substring((Join-Path $root 'node_modules').Length + 1)
    $destination = Join-Path $notices $relative
    New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $file.FullName -Destination $destination
}
# Accompany the application with the exact working source and locked Rust sources.
# No external runtime executable is redistributed in this archive.
$source = Join-Path $stage 'source'
New-Item -ItemType Directory $source | Out-Null
foreach ($name in @('Cargo.toml','Cargo.lock','package.json','package-lock.json','index.html','tsconfig.json','vite.config.ts','LICENSE','README.md','AGENTS.md')) {
    Copy-Item (Join-Path $root $name) $source
}
foreach ($name in @('crates','src','src-tauri','scripts','docs')) {
    Copy-Item (Join-Path $root $name) $source -Recurse
}
$frontendSource = Join-Path $source 'frontend-dependencies'
New-Item -ItemType Directory $frontendSource | Out-Null
foreach ($name in @('react','react-dom','scheduler','@tauri-apps/api')) {
    $destination = Join-Path $frontendSource $name
    New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
    Copy-Item (Join-Path $root "node_modules/$name") $destination -Recurse
}
# Exclude Tauri generated build schema/cache (if present) from the source copy.
$generated = Join-Path $source 'src-tauri/gen'
if (Test-Path -LiteralPath $generated) { Remove-Item -LiteralPath $generated -Recurse -Force }
& cargo vendor --locked --manifest-path (Join-Path $root 'Cargo.toml') (Join-Path $source 'vendor')
if ($LASTEXITCODE) { throw 'Cannot provision locked Rust corresponding source' }
$config = Join-Path $source '.cargo'
New-Item -ItemType Directory $config | Out-Null
[IO.File]::WriteAllText((Join-Path $config 'config.toml'), "[source.crates-io]`nreplace-with = 'vendored-sources'`n[source.vendored-sources]`ndirectory = 'vendor'`n")
# Stable entry order/timestamps: same input bytes produce the same ZIP on .NET.
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zipPath = "$stage.zip"
$zip = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($file in (Get-ChildItem -LiteralPath $stage -File -Recurse | Sort-Object FullName)) {
        $name = $file.FullName.Substring($stage.Length + 1).Replace('\','/')
        $entry = $zip.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
        $entry.LastWriteTime = [DateTimeOffset]::new(2026,11,11,0,0,0,[TimeSpan]::Zero)
        $input = $file.OpenRead(); $output = $entry.Open()
        try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
    }
} finally { $zip.Dispose() }
$sha = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$zipPath.sha256", "$sha  $([IO.Path]::GetFileName($zipPath))`n", [Text.Encoding]::ASCII)
Get-Item -LiteralPath $zipPath | Select-Object FullName,Length
$sha
