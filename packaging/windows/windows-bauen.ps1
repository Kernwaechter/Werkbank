# Baut unter Windows die Werkbank (MSVC) und packt sie als ZIP; mit Inno Setup (iscc) entsteht danach der Installer.
# Aufruf in PowerShell:  .\packaging\windows\windows-bauen.ps1
# Braucht: Rust (rustup, MSVC-Toolchain) und die „Build Tools für Visual Studio“ (C++).
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')
if ((Select-String -Path assets\kauf-url.txt -Pattern '/test_' -Quiet) -and $env:ALLOW_TESTLINK -ne '1') {
    throw 'assets\kauf-url.txt enthält noch den Stripe-Testlink. Live-Link eintragen oder $env:ALLOW_TESTLINK=1 setzen.'
}
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --target x86_64-pc-windows-msvc -p werkbank-app
if ($LASTEXITCODE -ne 0) { throw 'cargo build fehlgeschlagen' }
$v = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$out = 'target\windows'
$ziel = Join-Path $out 'Werkbank'
Remove-Item $ziel -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $ziel | Out-Null
Copy-Item target\x86_64-pc-windows-msvc\release\werkbank-app.exe, assets\handbuch.md, assets\lizenzen.txt, LIZENZVEREINBARUNG.md $ziel
if (Test-Path packaging\windows\woerterbuecher) { Copy-Item packaging\windows\woerterbuecher (Join-Path $ziel 'woerterbuecher') -Recurse }
$zip = Join-Path $out "Werkbank-$v-windows-x64.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $ziel -DestinationPath $zip
Get-Item $zip | Format-Table Name, Length
if (Get-Command iscc -ErrorAction SilentlyContinue) { iscc "/DVersion=$v" packaging\windows\werkbank.iss } else { Write-Host 'Für den Setup-Installer: Inno Setup installieren (winget install JRSoftware.InnoSetup) und iscc packaging\windows\werkbank.iss ausführen.' }