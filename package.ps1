# Packages the already-built release exe into a portable zip.
# Run AFTER `pnpm tauri build --no-bundle`.
#
# Produces:
#   deploy/
#     DaVinci Resolve Uninstaller/          (exe + README.txt)
#     DaVinci Resolve Uninstaller.zip       (that folder, zipped)

$ErrorActionPreference = "Stop"

$name  = "DaVinci Resolve Uninstaller"
$root  = $PSScriptRoot
$exe   = Join-Path $root "src-tauri\target\release\davinci-uninstaller.exe"
$stage = Join-Path $root "deploy\$name"
$zip   = Join-Path $root "deploy\$name.zip"

if (-not (Test-Path $exe)) {
    throw "Build not found: $exe`nRun 'pnpm tauri build --no-bundle' first."
}

# Fresh output each run.
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
if (Test-Path $zip)   { Remove-Item $zip -Force }
New-Item -ItemType Directory $stage -Force | Out-Null

Copy-Item $exe (Join-Path $stage "$name.exe")

@'
DaVinci Resolve Uninstaller (portable)

USAGE:
  Right-click "DaVinci Resolve Uninstaller.exe" -> Run as administrator.
  (Administrator rights are required to delete system files and registry keys.)

Requires WebView2, preinstalled on Windows 10/11.
No installation needed. To remove this app, just delete the .exe.
'@ | Out-File (Join-Path $stage "README.txt") -Encoding utf8

Compress-Archive -Path $stage -DestinationPath $zip -CompressionLevel Optimal

"Packaged -> $zip ({0:N1} MB)" -f ((Get-Item $zip).Length / 1MB)
