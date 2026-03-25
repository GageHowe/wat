param(
    [string]$Repo = $(if ($env:WAT_INSTALL_REPO) { $env:WAT_INSTALL_REPO } else { "howeg/wat" }),
    [string]$Version = $(if ($env:WAT_INSTALL_VERSION) { $env:WAT_INSTALL_VERSION } else { "latest" }),
    [string]$BinDir = $(if ($env:WAT_INSTALL_BIN) { $env:WAT_INSTALL_BIN } else { Join-Path $HOME ".local\bin" }),
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"

function Get-TargetTriple {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
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

$target = Get-TargetTriple
$archive = "wat-$target.zip"
$url = if ($Version -eq "latest") {
    "https://github.com/$Repo/releases/latest/download/$archive"
} else {
    "https://github.com/$Repo/releases/download/$Version/$archive"
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("wat-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $archivePath = Join-Path $tempDir $archive

    Write-Host "installing wat from $url"
    Invoke-WebRequest -Uri $url -OutFile $archivePath
    Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force
    Copy-Item -Path (Join-Path $tempDir "wat.exe") -Destination (Join-Path $BinDir "wat.exe") -Force

    Write-Host "wat installed to $(Join-Path $BinDir 'wat.exe')"
    $sessionPathEntries = $env:PATH -split ';'
    if (-not ($sessionPathEntries -contains $BinDir)) {
        $env:PATH = "$BinDir;$env:PATH"
    }

    if (-not $NoPathUpdate -and (Add-BinDirToUserPath -PathToAdd $BinDir)) {
        Write-Host "Added $BinDir to your user PATH."
        Write-Host "Open a new terminal window to use wat everywhere."
    }
    elseif (-not ($sessionPathEntries -contains $BinDir)) {
        Write-Host "Updated PATH for this session only."
    }
}
finally {
    Remove-Item -Recurse -Force -Path $tempDir -ErrorAction SilentlyContinue
}
