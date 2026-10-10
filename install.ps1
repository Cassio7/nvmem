# Installs (or updates) nvmem on Windows.
#
#   irm https://raw.githubusercontent.com/Cassio7/nvmem/main/install.ps1 | iex
#
# Environment variables:
#   NVMEM_VERSION      release tag to install, e.g. v0.1.2 (default: latest)
#   NVMEM_INSTALL_DIR  where to put nvmem.exe (default: %LOCALAPPDATA%\Programs\nvmem)

# Wrapped in a function so a partially downloaded script never runs, and so
# variables don't leak into the caller's session when piped into `iex`.
function Install-Nvmem {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'  # Invoke-WebRequest is very slow with the progress bar
    $repo = 'Cassio7/nvmem'

    # Windows PowerShell 5.1 may default to TLS 1.0, which GitHub rejects.
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    # Only an x64 build exists; Windows on ARM runs it through emulation.
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -ne 'AMD64' -and $arch -ne 'ARM64') {
        throw "nvmem install: unsupported CPU architecture: $arch"
    }
    $asset = 'nvmem-x86_64-pc-windows-msvc.exe'

    $version = if ($env:NVMEM_VERSION) { $env:NVMEM_VERSION } else { 'latest' }
    $url = if ($version -eq 'latest') {
        "https://github.com/$repo/releases/latest/download/$asset"
    } else {
        "https://github.com/$repo/releases/download/$version/$asset"
    }
    $installDir = if ($env:NVMEM_INSTALL_DIR) {
        $env:NVMEM_INSTALL_DIR
    } else {
        Join-Path $env:LOCALAPPDATA 'Programs\nvmem'
    }
    $exe = Join-Path $installDir 'nvmem.exe'

    Write-Host "Downloading $asset ($version)..."
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    $tmp = "$exe.download"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $tmp
    } catch {
        Remove-Item -Force -ErrorAction SilentlyContinue $tmp
        throw "nvmem install: download failed: $url ($($_.Exception.Message))"
    }

    # Verify against the release's "<hex>  <name>" checksum file. A mismatch is
    # an error; a missing checksum (releases up to v0.1.3) is only a warning.
    $sumUrl = "$url.sha256"
    $sumFile = "$exe.sha256.download"
    try {
        $haveSum = $true
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $sumUrl -OutFile $sumFile
        } catch {
            $status = $null
            if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
            if ($status -ne 404) {
                Remove-Item -Force -ErrorAction SilentlyContinue $tmp
                throw "nvmem install: download failed: $sumUrl ($($_.Exception.Message))"
            }
            $haveSum = $false
            Write-Warning 'nvmem install: checksum not found for this release, skipping verification'
        }
        if ($haveSum) {
            if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
                $sumText = "$(Get-Content -Raw -LiteralPath $sumFile)"  # "" if the file is empty
                $expected = ($sumText.Trim() -split '\s+')[0]
                $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $tmp).Hash
                if ($actual -ine $expected) {
                    Remove-Item -Force -ErrorAction SilentlyContinue $tmp
                    throw "nvmem install: checksum mismatch for ${asset}: expected $expected, got $actual"
                }
                Write-Host "Checksum verified (SHA256 $($actual.ToLowerInvariant()))."
            } else {
                Write-Warning 'nvmem install: Get-FileHash not available, skipping checksum verification'
            }
        }
    } finally {
        Remove-Item -Force -ErrorAction SilentlyContinue $sumFile
    }

    Move-Item -Force $tmp $exe

    $installed = & $exe --version
    if ($LASTEXITCODE -ne 0) { throw "nvmem install: installed binary failed to run: $exe" }
    Write-Host "Installed $installed to $exe"

    # Add to the user PATH (persistent) and to this session.
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ })
    if ($entries -notcontains $installDir) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $installDir) -join ';'), 'User')
        Write-Host "Added $installDir to your user PATH. Open a new terminal to use 'nvmem' everywhere."
    }
    if (($env:Path -split ';') -notcontains $installDir) {
        $env:Path = "$env:Path;$installDir"
    }
    Write-Host "Run 'nvmem' to get started."
}

Install-Nvmem
