param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$version = (Get-Content -LiteralPath (Join-Path $PSScriptRoot '../src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$provenanceJson = $null
$archives = @(Get-ChildItem -LiteralPath $OutputDirectory -Filter '*.zip' -File)
if ($archives.Count -ne 2) { throw 'Expected exactly portable and corresponding-source archives' }
foreach ($archive in $archives) {
    if ($archive.Name -notin @("Fuzzy-Music-$version-windows-x64.zip", "Fuzzy-Music-$version-corresponding-source.zip")) { throw 'Archive version/name mismatch' }
    $hash = (Get-FileHash $archive.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ([IO.File]::ReadAllText("$($archive.FullName).sha256") -ne "$hash  $($archive.Name)`n") { throw 'Checksum mismatch' }
    $zip = [IO.Compression.ZipFile]::OpenRead($archive.FullName)
    try {
        $names = @($zip.Entries.FullName)
        $reader = [IO.StreamReader]::new($zip.GetEntry('SOURCE-PROVENANCE.json').Open())
        try { $json = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $provenance = $json | ConvertFrom-Json
        if ($provenance.version -ne $version) { throw 'Provenance version mismatch' }
        if ($null -ne $provenanceJson -and $json -ne $provenanceJson) { throw 'Source/executable provenance differs' }
        $provenanceJson = $json
        if ($env:GITHUB_ACTIONS -eq 'true' -and ($provenance.dirty -or $provenance.commit -ne $env:GITHUB_SHA)) { throw 'CI package does not match the clean workflow commit' }
        if ($archive.Name -match 'corresponding-source') {
            foreach ($required in @('Cargo.lock','.cargo/config.toml','scripts/package-windows.ps1','scripts/check-release.mjs','frontend-dependencies/lucide-react/package.json','SOURCE-PROVENANCE.json')) {
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
            $stream = $zip.GetEntry('Fuzzy Music.exe').Open()
            $sha = [Security.Cryptography.SHA256]::Create()
            try { $executableHash = ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
            finally { $stream.Dispose(); $sha.Dispose() }
            if ($executableHash -ne $provenance.executableSha256) { throw 'Executable provenance checksum mismatch' }
        }
        [pscustomobject]@{Archive=$archive.Name;CompressedBytes=$archive.Length;UnpackedBytes=($zip.Entries | Measure-Object Length -Sum).Sum;Entries=$zip.Entries.Count;SHA256=$hash}
    } finally { $zip.Dispose() }
}
