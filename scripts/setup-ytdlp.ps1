$ErrorActionPreference = 'Stop'
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
    throw 'This setup script supports Windows x64. See docs/soundcloud-development.md for other platforms.'
}
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$toolsDirectory = Join-Path $repositoryRoot 'tools'
$executable = Join-Path $toolsDirectory 'yt-dlp.exe'
$expectedHash = 'f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d'
if (Test-Path -LiteralPath $executable) {
    $currentHash = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($currentHash -eq $expectedHash) { return }
    throw 'Existing tools/yt-dlp.exe does not match the pinned release. Rename it before running setup again.'
}
New-Item -ItemType Directory -Force -Path $toolsDirectory | Out-Null
$download = Join-Path $toolsDirectory 'yt-dlp.exe.download'
Write-Host 'Downloading official yt-dlp 2026.10.26 (Windows x64)...'
Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe' -OutFile $download -TimeoutSec 120
$downloadHash = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant()
if ($downloadHash -ne $expectedHash) { throw 'Downloaded yt-dlp failed SHA-256 verification.' }
Move-Item -LiteralPath $download -Destination $executable
Write-Host 'yt-dlp ready; SHA-256 verified.'
