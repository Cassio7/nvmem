# Installs (or updates) nvmtree on Windows.
#
#   irm https://raw.githubusercontent.com/Cassio7/nvmtree/main/install.ps1 | iex
#
# Environment variables:
#   NVMTREE_VERSION      release tag to install, e.g. v0.1.2 (default: latest)
#   NVMTREE_INSTALL_DIR  where to put nvmtree.exe (default: %LOCALAPPDATA%\Programs\nvmtree)

# Wrapped in a function so a partially downloaded script never runs, and so
# variables don't leak into the caller's session when piped into `iex`.
function Install-Nvmtree {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'  # Invoke-WebRequest is very slow with the progress bar
    $repo = 'Cassio7/nvmtree'

    # Windows PowerShell 5.1 may default to TLS 1.0, which GitHub rejects.
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    # Only an x64 build exists; Windows on ARM runs it through emulation.
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -ne 'AMD64' -and $arch -ne 'ARM64') {
        throw "nvmtree install: unsupported CPU architecture: $arch"
    }
    $asset = 'nvmtree-x86_64-pc-windows-msvc.exe'

    $version = if ($env:NVMTREE_VERSION) { $env:NVMTREE_VERSION } else { 'latest' }
    $url = if ($version -eq 'latest') {
        "https://github.com/$repo/releases/latest/download/$asset"
    } else {
        "https://github.com/$repo/releases/download/$version/$asset"
    }
    $installDir = if ($env:NVMTREE_INSTALL_DIR) {
        $env:NVMTREE_INSTALL_DIR
    } else {
        Join-Path $env:LOCALAPPDATA 'Programs\nvmtree'
    }
    $exe = Join-Path $installDir 'nvmtree.exe'

    Write-Host "Downloading $asset ($version)..."
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    $tmp = "$exe.download"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $tmp
    } catch {
        Remove-Item -Force -ErrorAction SilentlyContinue $tmp
        throw "nvmtree install: download failed: $url ($($_.Exception.Message))"
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
                throw "nvmtree install: download failed: $sumUrl ($($_.Exception.Message))"
            }
            $haveSum = $false
            Write-Warning 'nvmtree install: checksum not found for this release, skipping verification'
        }
        if ($haveSum) {
            if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
                $sumText = "$(Get-Content -Raw -LiteralPath $sumFile)"  # "" if the file is empty
                $expected = ($sumText.Trim() -split '\s+')[0]
                $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $tmp).Hash
                if ($actual -ine $expected) {
                    Remove-Item -Force -ErrorAction SilentlyContinue $tmp
                    throw "nvmtree install: checksum mismatch for ${asset}: expected $expected, got $actual"
                }
                Write-Host "Checksum verified (SHA256 $($actual.ToLowerInvariant()))."
            } else {
                Write-Warning 'nvmtree install: Get-FileHash not available, skipping checksum verification'
            }
        }
    } finally {
        Remove-Item -Force -ErrorAction SilentlyContinue $sumFile
    }

    Move-Item -Force $tmp $exe

    $installed = & $exe --version
    if ($LASTEXITCODE -ne 0) { throw "nvmtree install: installed binary failed to run: $exe" }
    Write-Host "Installed $installed to $exe"

    # Add to the user PATH (persistent) and to this session.
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ })
    if ($entries -notcontains $installDir) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $installDir) -join ';'), 'User')
        Write-Host "Added $installDir to your user PATH. Open a new terminal to use 'nvmtree' everywhere."
    }
    if (($env:Path -split ';') -notcontains $installDir) {
        $env:Path = "$env:Path;$installDir"
    }
    Write-Host "Run 'nvmtree' to get started."
}

Install-Nvmtree
