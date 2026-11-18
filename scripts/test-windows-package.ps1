param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archives = @(Get-ChildItem -LiteralPath $OutputDirectory -Filter '*.zip' -File)
if ($archives.Count -ne 2) { throw 'Expected exactly portable and corresponding-source archives' }
foreach ($archive in $archives) {
    $hash = (Get-FileHash $archive.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ([IO.File]::ReadAllText("$($archive.FullName).sha256") -ne "$hash  $($archive.Name)`n") { throw 'Checksum mismatch' }
    $zip = [IO.Compression.ZipFile]::OpenRead($archive.FullName)
    try {
        $names = @($zip.Entries.FullName)
        if ($archive.Name -match 'corresponding-source') {
            foreach ($required in @('Cargo.lock','.cargo/config.toml','scripts/package-windows.ps1','SOURCE-PROVENANCE.json')) {
                if ($required -notin $names) { throw "Source missing $required" }
            }
            if (-not ($names -match '^vendor/')) { throw 'Missing locked Rust source' }
            if ($names -match '^(dist|target|artifacts|tools|node_modules)/') { throw 'Source contains build/runtime caches' }
        } else {
            foreach ($required in @('Fuzzy Music.exe','LICENSE','THIRD-PARTY.md','setup-windows-runtime.ps1','SOURCE-PROVENANCE.json')) {
                if ($required -notin $names) { throw "Portable missing $required" }
            }
            if ($names -match '(^|/)(source|vendor|target|dist|tools|node_modules)/|\.(pdb|ilk|rlib|rmeta)$') { throw 'Portable contains source/debug/cache/runtime files' }
            if (-not ($names -match '^licenses/npm/')) { throw 'Missing notices' }
        }
        [pscustomobject]@{Archive=$archive.Name;CompressedBytes=$archive.Length;UnpackedBytes=($zip.Entries | Measure-Object Length -Sum).Sum;Entries=$zip.Entries.Count;SHA256=$hash}
    } finally { $zip.Dispose() }
}
