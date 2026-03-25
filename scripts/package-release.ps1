param(
    [Parameter(Mandatory = $true)]
    [string]$Target
)

$ErrorActionPreference = "Stop"

$version = Select-String -Path "Cargo.toml" -Pattern '^version = "(.*)"$' | ForEach-Object { $_.Matches[0].Groups[1].Value } | Select-Object -First 1
$outDir = "dist"
$stageDir = Join-Path $outDir "wat-$version-$Target"
$archivePath = Join-Path $outDir "wat-$Target.zip"

if (Test-Path $stageDir) {
    Remove-Item -Recurse -Force $stageDir
}

New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

cargo build --release --target $Target
Copy-Item "target\$Target\release\wat.exe" "$stageDir\wat.exe" -Force
Copy-Item "README.md" "$stageDir\README.md" -Force

if (Test-Path $archivePath) {
    Remove-Item -Force $archivePath
}

Compress-Archive -Path "$stageDir\*" -DestinationPath $archivePath
Write-Host "wrote $archivePath"
