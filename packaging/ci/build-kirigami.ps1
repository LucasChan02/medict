param(
    [string]$Prefix = "",
    [string]$BuildRoot = ""
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

New-Item -ItemType Directory -Force $Downloads, $Prefix | Out-Null

function Get-VerifiedArchive {
    param(
        [string]$Name,
        [string]$Checksum,
        [string]$Url
    )

    $File = Join-Path $Downloads $Name
    if (-not (Test-Path $File)) {
        Invoke-WebRequest -Uri $Url -OutFile $File -UseBasicParsing
    }
    $Actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $File).Hash.ToLowerInvariant()
    if ($Actual -ne $Checksum) {
        throw "Checksum mismatch for $Name."
    }
    return $File
}

$EcmFile = Get-VerifiedArchive `
    -Name $EcmArchive `
    -Checksum $EcmChecksum `
    -Url "https://download.kde.org/stable/frameworks/6.31/$EcmArchive"
$KirigamiFile = Get-VerifiedArchive `
    -Name $KirigamiArchive `
    -Checksum $KirigamiChecksum `
    -Url "https://download.kde.org/stable/frameworks/6.31/$KirigamiArchive"

tar -xJf $EcmFile -C $BuildRoot
tar -xJf $KirigamiFile -C $BuildRoot

$PrefixPath = "$env:QT_ROOT_DIR;$Prefix"
$EcmSource = Join-Path $BuildRoot "extra-cmake-modules-$EcmVersion"
$EcmBuild = Join-Path $BuildRoot "ecm-build"

cmake -S $EcmSource -B $EcmBuild -G "Visual Studio 17 2022" -A x64 `
    -DCMAKE_INSTALL_PREFIX="$Prefix" `
    -DBUILD_TESTING=OFF
if ($LASTEXITCODE -ne 0) {
    throw "ECM configure failed."
}
cmake --build $EcmBuild --config Release --parallel
if ($LASTEXITCODE -ne 0) {
    throw "ECM build failed."
}
cmake --install $EcmBuild --config Release
if ($LASTEXITCODE -ne 0) {
    throw "ECM install failed."
}

$KirigamiSource = Join-Path $BuildRoot "kirigami-$KirigamiVersion"
$KirigamiBuild = Join-Path $BuildRoot "kirigami-build"

cmake -S $KirigamiSource -B $KirigamiBuild -G "Visual Studio 17 2022" -A x64 `
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
cmake --build $KirigamiBuild --config Release --parallel
if ($LASTEXITCODE -ne 0) {
    throw "Kirigami build failed."
}
cmake --install $KirigamiBuild --config Release
if ($LASTEXITCODE -ne 0) {
    throw "Kirigami install failed."
}
