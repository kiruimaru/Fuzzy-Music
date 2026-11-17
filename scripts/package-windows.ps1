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
# Offer exact working source and locked Rust sources as a separate download.
# No external runtime executable is redistributed in this archive.
$source = Join-Path $release 'Fuzzy Music 0.1.0-alpha.1 corresponding-source'
if (Test-Path -LiteralPath $source) { throw 'Source staging exists; choose a fresh output directory.' }
New-Item -ItemType Directory $source | Out-Null
foreach ($name in @('Cargo.toml','Cargo.lock','package.json','package-lock.json','index.html','tsconfig.json','vite.config.ts','LICENSE','README.md','AGENTS.md')) {
    Copy-Item (Join-Path $root $name) $source
}
# Copy only tracked inputs: never recurse into dist, caches or runtime downloads.
$tracked = & git -C $root ls-files
if ($LASTEXITCODE) { throw 'Cannot enumerate source tree' }
foreach ($name in $tracked) {
    if ($name -match '^(crates|src|src-tauri|scripts|docs|tests|tasks|\.github)/' -or $name -in @('playwright.config.ts','.gitignore')) {
        $destination = Join-Path $source $name
        New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath (Join-Path $root $name) $destination
    }
}
$commit = & git -C $root rev-parse HEAD
$tree = & git -C $root rev-parse 'HEAD^{tree}'
$dirty = @(& git -C $root status --porcelain --untracked-files=no)
$provenance = [ordered]@{ commit=$commit; tree=$tree; dirty=($dirty.Count -gt 0); note='Tracked working files packaged; dirty=true means local edits differ from the recorded Git tree.'; executableSha256=(Get-FileHash (Join-Path $stage 'Fuzzy Music.exe')).Hash.ToLowerInvariant() }
$provenance | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $source 'SOURCE-PROVENANCE.json') -Encoding UTF8
$provenance | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'SOURCE-PROVENANCE.json') -Encoding UTF8
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
foreach ($directory in @($stage,$source)) {
$zipPath = "$directory.zip"
$zip = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($file in (Get-ChildItem -LiteralPath $directory -File -Recurse | Sort-Object FullName)) {
        $name = $file.FullName.Substring($directory.Length + 1).Replace('\','/')
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
}
& (Join-Path $PSScriptRoot 'test-windows-package.ps1') -OutputDirectory $release | Format-List
if (-not $?) { throw 'Package assertions failed' }
