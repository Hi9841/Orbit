Add-Type -AssemblyName System.Drawing

$assetDir = Join-Path $PSScriptRoot '..\assets'
$sourcePath = Join-Path $assetDir 'orbit-mark.png'
$pngPath = Join-Path $assetDir 'orbit.png'
$iconPath = Join-Path $assetDir 'orbit.ico'

if (-not (Test-Path $sourcePath)) {
    throw "Missing $sourcePath"
}

$source = [System.Drawing.Bitmap]::FromFile((Resolve-Path $sourcePath))
$minX = $source.Width
$minY = $source.Height
$maxX = 0
$maxY = 0
for ($y = 0; $y -lt $source.Height; $y++) {
    for ($x = 0; $x -lt $source.Width; $x++) {
        if ($source.GetPixel($x, $y).A -gt 16) {
            if ($x -lt $minX) { $minX = $x }
            if ($y -lt $minY) { $minY = $y }
            if ($x -gt $maxX) { $maxX = $x }
            if ($y -gt $maxY) { $maxY = $y }
        }
    }
}
$cropW = [Math]::Max(1, $maxX - $minX + 1)
$cropH = [Math]::Max(1, $maxY - $minY + 1)
$side = [Math]::Max($cropW, $cropH)
$cropped = New-Object System.Drawing.Bitmap $side, $side, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$cropGraphics = [System.Drawing.Graphics]::FromImage($cropped)
$cropGraphics.Clear([System.Drawing.Color]::Transparent)
$cropGraphics.DrawImage(
    $source,
    (New-Object System.Drawing.Rectangle ([int](($side - $cropW) / 2), [int](($side - $cropH) / 2), $cropW, $cropH)),
    (New-Object System.Drawing.Rectangle $minX, $minY, $cropW, $cropH),
    [System.Drawing.GraphicsUnit]::Pixel
)
$cropGraphics.Dispose()
$source.Dispose()

function New-SquareIcon([System.Drawing.Image]$image, [int]$size) {
    $bitmap = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.DrawImage($image, 0, 0, $size, $size)
    $graphics.Dispose()
    return $bitmap
}

function Get-IconImageBytes([System.Drawing.Bitmap]$bitmap) {
    $width = $bitmap.Width
    $height = $bitmap.Height
    $xor = New-Object byte[] ($width * $height * 4)
    for ($y = 0; $y -lt $height; $y++) {
        $sourceY = $height - 1 - $y
        for ($x = 0; $x -lt $width; $x++) {
            $color = $bitmap.GetPixel($x, $sourceY)
            $index = ($y * $width + $x) * 4
            $xor[$index] = $color.B
            $xor[$index + 1] = $color.G
            $xor[$index + 2] = $color.R
            $xor[$index + 3] = $color.A
        }
    }
    $maskStride = [int]([Math]::Ceiling($width / 32.0) * 4)
    $mask = New-Object byte[] ($maskStride * $height)
    $stream = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter $stream
    $writer.Write([uint32]40)
    $writer.Write([int32]$width)
    $writer.Write([int32]($height * 2))
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]0)
    $writer.Write([uint32]($xor.Length + $mask.Length))
    $writer.Write([int32]0)
    $writer.Write([int32]0)
    $writer.Write([uint32]0)
    $writer.Write([uint32]0)
    $writer.Write($xor)
    $writer.Write($mask)
    $writer.Flush()
    $bytes = $stream.ToArray()
    $writer.Dispose()
    $stream.Dispose()
    return $bytes
}

$sizes = @(16, 24, 32, 48, 64, 256)
$images = @()
$payloads = @()
foreach ($size in $sizes) {
    $bitmap = New-SquareIcon $cropped $size
    if ($size -eq 256) {
        $bitmap.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    $images += $bitmap
    $payloads += ,(Get-IconImageBytes $bitmap)
}

$file = [System.IO.File]::Create([System.IO.Path]::GetFullPath($iconPath))
$writer = New-Object System.IO.BinaryWriter $file
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]$sizes.Count)
$offset = 6 + (16 * $sizes.Count)
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $size = $sizes[$i]
    $dimension = if ($size -ge 256) { [byte]0 } else { [byte]$size }
    $writer.Write($dimension)
    $writer.Write($dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$payloads[$i].Length)
    $writer.Write([uint32]$offset)
    $offset += $payloads[$i].Length
}
$writer.Flush()
foreach ($payload in $payloads) {
    $raw = [byte[]]$payload
    $file.Write($raw, 0, $raw.Length)
}
$file.Flush()
$writer.Dispose()
$file.Dispose()
foreach ($bitmap in $images) { $bitmap.Dispose() }
$cropped.Dispose()
Write-Output "wrote $iconPath and $pngPath"
