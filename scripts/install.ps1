$ErrorActionPreference = "Stop"

function Get-TargetTriple {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        default { throw "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
}

function Add-BinDirToUserPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathToAdd
    )

    $currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @()
    if ($currentUserPath) {
        $entries = $currentUserPath -split ';' | Where-Object { $_ }
    }

    if ($entries -contains $PathToAdd) {
        return $false
    }

    $updatedEntries = @($entries + $PathToAdd)
    [Environment]::SetEnvironmentVariable("Path", ($updatedEntries -join ';'), "User")
    return $true
}

$repo = "howeg/wat"
$binDir = Join-Path $HOME ".local\bin"
$target = Get-TargetTriple
$archive = "wat-$target.zip"
$url = "https://github.com/$repo/releases/latest/download/$archive"
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("wat-install-" + [System.Guid]::NewGuid().ToString("N"))

New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    $archivePath = Join-Path $tempDir $archive

    Write-Host "installing wat from $url"
    Invoke-WebRequest -Uri $url -OutFile $archivePath
    Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force
    Copy-Item -Path (Join-Path $tempDir "wat.exe") -Destination (Join-Path $binDir "wat.exe") -Force

    Write-Host "wat installed to $(Join-Path $binDir 'wat.exe')"

    $sessionPathEntries = $env:PATH -split ';'
    if (-not ($sessionPathEntries -contains $binDir)) {
        $env:PATH = "$binDir;$env:PATH"
    }

    if (Add-BinDirToUserPath -PathToAdd $binDir) {
        Write-Host "Added $binDir to your user PATH."
        Write-Host "Open a new terminal window to use wat everywhere."
    }
    elseif (-not ($sessionPathEntries -contains $binDir)) {
        Write-Host "Updated PATH for this session."
    }
}
finally {
    Remove-Item -Recurse -Force -Path $tempDir -ErrorAction SilentlyContinue
}
