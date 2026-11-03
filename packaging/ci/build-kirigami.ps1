param(
    [string]$Prefix = "",
    [string]$BuildRoot = "",
    [ValidateSet("All", "Download", "ECM", "Kirigami")]
    [string]$Stage = "All"
)

$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $Prefix) {
    $Prefix = Join-Path $Root "build\kf6-prefix"
}
if (-not $BuildRoot) {
    $BuildRoot = Join-Path $Root "build"
}
if (-not $env:QT_ROOT_DIR) {
    throw "QT_ROOT_DIR is not set."
}

$Downloads = Join-Path $BuildRoot "downloads"
$EcmVersion = "6.31.0"
$KirigamiVersion = "6.31.0"
$EcmArchive = "extra-cmake-modules-$EcmVersion.tar.xz"
$KirigamiArchive = "kirigami-$KirigamiVersion.tar.xz"
$EcmChecksum = "8f74af329e4c918358bfe54316f8cf4d530c28444595a3e2fb15075f72877f17"
$KirigamiChecksum = "811a8a150e636216269c0346e62470997fa282113a39e32b2d332bd5969926f0"
$Marker = Join-Path $Prefix ".medict-kirigami-$KirigamiVersion"

function Write-Phase {
    param([string]$Message)
    Write-Host ("[{0}] {1}" -f (Get-Date).ToUniversalTime().ToString("HH:mm:ss"), $Message)
}

New-Item -ItemType Directory -Force $Downloads, $Prefix | Out-Null

if (Test-Path -LiteralPath $Marker) {
    Write-Phase "Reusing installed Kirigami $KirigamiVersion from $Prefix."
    return
}

function Get-VerifiedArchive {
    param(
        [string]$Name,
        [string]$Checksum,
        [string]$Url
    )

    $File = Join-Path $Downloads $Name
    if (-not (Test-Path $File)) {
        $Partial = "$File.part"
        Remove-Item -LiteralPath $Partial -Force -ErrorAction SilentlyContinue
        Write-Phase "Downloading $Url"
        & curl.exe -fL --retry 3 --retry-delay 5 --connect-timeout 20 --max-time 300 `
            --output $Partial $Url
        if ($LASTEXITCODE -ne 0) {
            throw "Download failed for $Name."
        }
        Move-Item -LiteralPath $Partial -Destination $File -Force
    }
    Write-Phase "Verifying $Name"
    $Actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $File).Hash.ToLowerInvariant()
    if ($Actual -ne $Checksum) {
        throw "Checksum mismatch for $Name."
    }
    return $File
}

$PrefixPath = "$env:QT_ROOT_DIR;$Prefix"
$EcmSource = Join-Path $BuildRoot "extra-cmake-modules-$EcmVersion"
$EcmBuild = Join-Path $BuildRoot "ecm-build"
$KirigamiSource = Join-Path $BuildRoot "kirigami-$KirigamiVersion"
$KirigamiBuild = Join-Path $BuildRoot "kirigami-build"

if ($Stage -in @("All", "Download")) {
    $EcmFile = Get-VerifiedArchive `
        -Name $EcmArchive `
        -Checksum $EcmChecksum `
        -Url "https://download.kde.org/stable/frameworks/6.31/$EcmArchive"
    $KirigamiFile = Get-VerifiedArchive `
        -Name $KirigamiArchive `
        -Checksum $KirigamiChecksum `
        -Url "https://download.kde.org/stable/frameworks/6.31/$KirigamiArchive"

    if (-not (Test-Path -LiteralPath $EcmSource)) {
        Write-Phase "Extracting ECM"
        tar -xJf $EcmFile -C $BuildRoot
        if ($LASTEXITCODE -ne 0) {
            throw "ECM extraction failed."
        }
    }
    if (-not (Test-Path -LiteralPath $KirigamiSource)) {
        Write-Phase "Extracting Kirigami"
        tar -xJf $KirigamiFile -C $BuildRoot
        if ($LASTEXITCODE -ne 0) {
            throw "Kirigami extraction failed."
        }
    }
}

if ($Stage -eq "Download") {
    Write-Phase "KDE source archives are ready."
    return
}

if ($Stage -in @("All", "ECM")) {
    Write-Phase "Configuring ECM"
    cmake -S $EcmSource -B $EcmBuild -G Ninja `
        -DCMAKE_BUILD_TYPE=Release `
        -DCMAKE_INSTALL_PREFIX="$Prefix" `
        -DBUILD_TESTING=OFF
    if ($LASTEXITCODE -ne 0) {
        throw "ECM configure failed."
    }
    Write-Phase "Building ECM"
    cmake --build $EcmBuild --parallel
    if ($LASTEXITCODE -ne 0) {
        throw "ECM build failed."
    }
    Write-Phase "Installing ECM"
    cmake --install $EcmBuild
    if ($LASTEXITCODE -ne 0) {
        throw "ECM install failed."
    }
}

if ($Stage -eq "ECM") {
    Write-Phase "ECM is ready."
    return
}

if ($Stage -in @("All", "Kirigami")) {
    Write-Phase "Configuring Kirigami"
    cmake -S $KirigamiSource -B $KirigamiBuild -G Ninja `
        -DCMAKE_BUILD_TYPE=Release `
        -DCMAKE_INSTALL_PREFIX="$Prefix" `
        -DCMAKE_PREFIX_PATH="$PrefixPath" `
        -DBUILD_TESTING=OFF `
        -DBUILD_EXAMPLES=OFF `
        -DUSE_DBUS=OFF `
        -DKDE_INSTALL_BINDIR=. `
        -DKDE_INSTALL_LIBDIR=lib `
        -DKDE_INSTALL_QMLDIR=qml `
        -DKDE_INSTALL_INCLUDEDIR=include
    if ($LASTEXITCODE -ne 0) {
        throw "Kirigami configure failed."
    }
    Write-Phase "Building Kirigami"
    cmake --build $KirigamiBuild --parallel
    if ($LASTEXITCODE -ne 0) {
        throw "Kirigami build failed."
    }
    Write-Phase "Installing Kirigami"
    cmake --install $KirigamiBuild
    if ($LASTEXITCODE -ne 0) {
        throw "Kirigami install failed."
    }

    New-Item -ItemType File -Path $Marker -Force | Out-Null
    Write-Phase "Kirigami $KirigamiVersion is ready."
}
