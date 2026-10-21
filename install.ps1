# Instalador de acm para Windows (no necesita Rust ni cargo).
#
#   irm https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main/install.ps1 | iex
#
# Descarga acm.exe de la última Release de GitHub, lo deja en
# %LOCALAPPDATA%\Programs\acm\bin y añade esa carpeta al PATH del usuario.
# Para una versión concreta:  $env:ACM_VERSION = "v0.1.0"; irm ... | iex

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # Invoke-WebRequest va mucho más rápido sin barra
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo = 'AnthonyDev65/acm-rs-compiler'
$asset = 'acm-windows-x86_64.exe'
if ($env:ACM_VERSION) {
    $url = "https://github.com/$repo/releases/download/$($env:ACM_VERSION)/$asset"
} else {
    $url = "https://github.com/$repo/releases/latest/download/$asset"
}

if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Host 'acm solo tiene binarios para Windows de 64 bits.' -ForegroundColor Red
    return
}

$binDir = Join-Path $env:LOCALAPPDATA 'Programs\acm\bin'
$exe = Join-Path $binDir 'acm.exe'
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

Write-Host "Descargando $url"
$tmp = "$exe.download"
try {
    Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
} catch {
    if (Test-Path $tmp) { Remove-Item $tmp -Force }
    Write-Host "No se pudo descargar acm: $($_.Exception.Message)" -ForegroundColor Red
    return
}

# Si acm.exe está en uso no se puede sobrescribir, pero sí renombrar.
if (Test-Path $exe) {
    $old = "$exe.old"
    if (Test-Path $old) { Remove-Item $old -Force -ErrorAction SilentlyContinue }
    Move-Item $exe $old -Force
}
Move-Item $tmp $exe -Force

# PATH del usuario (persistente) y de la sesión actual.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$parts = @()
if ($userPath) { $parts = $userPath -split ';' | Where-Object { $_ } }
if ($parts -notcontains $binDir) {
    [Environment]::SetEnvironmentVariable('Path', (($parts + $binDir) -join ';'), 'User')
}
if (($env:Path -split ';') -notcontains $binDir) { $env:Path = "$env:Path;$binDir" }

$version = & $exe --version
Write-Host "$version instalado en $exe" -ForegroundColor Green
Write-Host 'Abre una terminal nueva si `acm` no se reconoce en otras ventanas.'
Write-Host 'Opcional: `acm --register` para abrir los .acm con doble clic.'
