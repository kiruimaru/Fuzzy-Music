param([string]$Destination = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Destination -PathType Container)) { throw 'Destination directory does not exist.' }
$tools = Join-Path $Destination 'tools'
New-Item -ItemType Directory -Force $tools | Out-Null
$dependencies = @(
    @{ name = 'mpv'; version = '20261111-bcde6b93cc'; file = 'mpv.7z'; url = 'https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261111/mpv-x86_64-20261111-git-bcde6b93cc.7z'; sha256 = 'f259b5fd4b5c395617c948a4fadcdf55bba1c5282fb7c13e88626b3f71072baf' },
    @{ name = 'yt-dlp'; version = '2026.10.26'; file = 'yt-dlp.exe'; url = 'https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe'; sha256 = 'f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d' }
)
foreach ($dependency in $dependencies) {
    $path = Join-Path $tools $dependency.file
    if (-not (Test-Path -LiteralPath $path)) {
        $download = "$path.download"
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            try {
                Invoke-WebRequest -UseBasicParsing $dependency.url -OutFile $download -TimeoutSec 180
                if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ne $dependency.sha256) { throw "SHA256 mismatch for $($dependency.name)" }
                Move-Item -LiteralPath $download -Destination $path -Force
                break
            } catch { if ($attempt -eq 3) { throw }; Start-Sleep -Seconds (2 * $attempt) }
        }
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $dependency.sha256) { throw "SHA256 mismatch for $($dependency.name); remove the damaged file and retry." }
    if ($dependency.name -eq 'mpv') {
        & tar -xf $path -C $tools
        if ($LASTEXITCODE) { throw 'Cannot extract mpv. Windows tar with 7z support is required.' }
    }
}
# Keep the upstream executable notices with the locally acquired tools.
foreach ($name in @('LICENSE', 'THIRD_PARTY_LICENSES.txt')) {
    Invoke-WebRequest -UseBasicParsing "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/$name" -OutFile (Join-Path $tools "yt-dlp-$name") -TimeoutSec 120
}
$dependencies | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $tools 'runtime-provenance.json') -Encoding UTF8
'Runtime installed locally. Start Fuzzy Music.exe. Keep the complete tools directory.'
