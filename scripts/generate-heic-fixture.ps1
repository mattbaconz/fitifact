param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($PSVersionTable.PSEdition -ne "Desktop") {
    throw "Run this generator with Windows PowerShell 5.1 (powershell.exe), not PowerShell 7."
}

Add-Type -AssemblyName System.Runtime.WindowsRuntime
$bitmapEncoderType = [Windows.Graphics.Imaging.BitmapEncoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$bitmapPixelFormatType = [Windows.Graphics.Imaging.BitmapPixelFormat, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$bitmapAlphaModeType = [Windows.Graphics.Imaging.BitmapAlphaMode, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$memoryStreamType = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]

$asTaskGeneric = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq "AsTask" -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
} | Select-Object -First 1
$asTaskAction = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq "AsTask" -and -not $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
} | Select-Object -First 1

function Await-Result($Operation, [Type]$ResultType) {
    $task = $asTaskGeneric.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
    return $task.GetAwaiter().GetResult()
}

function Await-Action($Operation) {
    $task = $asTaskAction.Invoke($null, @($Operation))
    $task.GetAwaiter().GetResult()
}

$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root "fixtures\image\synthetic-single.heic"
if ((Test-Path -LiteralPath $output) -and -not $Force) {
    Write-Host "Keeping existing checksum-pinned HEIC fixture. Pass -Force to regenerate it with the installed Windows codec."
    return
}
$width = 16
$height = 12
$pixels = New-Object byte[] ($width * $height * 4)
for ($y = 0; $y -lt $height; $y += 1) {
    for ($x = 0; $x -lt $width; $x += 1) {
        $offset = ($y * $width + $x) * 4
        $pixels[$offset] = [byte](32 + $x * 10)
        $pixels[$offset + 1] = [byte](48 + $y * 12)
        $pixels[$offset + 2] = [byte](180 - $x * 5)
        $pixels[$offset + 3] = 255
    }
}

$memory = New-Object $memoryStreamType
try {
    $encoder = Await-Result ($bitmapEncoderType::CreateAsync($bitmapEncoderType::HeifEncoderId, $memory)) $bitmapEncoderType
    $encoder.SetPixelData(
        $bitmapPixelFormatType::Rgba8,
        $bitmapAlphaModeType::Ignore,
        $width,
        $height,
        96,
        96,
        $pixels
    )
    Await-Action ($encoder.FlushAsync())
    $memory.Seek(0)
    $source = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($memory)
    $destination = [System.IO.File]::Create($output)
    try {
        $source.CopyTo($destination)
    }
    finally {
        $destination.Dispose()
        $source.Dispose()
    }
}
finally {
    $memory.Dispose()
}

Write-Host "Generated $output from the owned 16 x 12 RGBA pixel pattern with the installed Windows HEIF encoder."
