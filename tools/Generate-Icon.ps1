Add-Type -AssemblyName System.Drawing

$iconPath = Join-Path $PSScriptRoot '..\assets\orbit.ico'
$assetDir = Split-Path -Parent $iconPath
New-Item -ItemType Directory -Force -Path $assetDir | Out-Null

$bitmap = [System.Drawing.Bitmap]::new(256, 256, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([System.Drawing.Color]::Transparent)

$background = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 22, 29, 47))
$graphics.FillEllipse($background, 8, 8, 240, 240)
$orbitPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 235, 242, 255), 25)
$orbitPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$orbitPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$graphics.DrawArc($orbitPen, 52, 52, 152, 152, 33, 280)
$dotBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 85, 223, 204))
$graphics.FillEllipse($dotBrush, 170, 47, 49, 49)

$pngStream = [System.IO.MemoryStream]::new()
$bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
$pngBytes = $pngStream.ToArray()
[System.IO.File]::WriteAllBytes((Join-Path $assetDir 'orbit.png'), $pngBytes)
$file = [System.IO.File]::Create([System.IO.Path]::GetFullPath($iconPath))
$writer = [System.IO.BinaryWriter]::new($file)
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]1)
$writer.Write([byte]0)
$writer.Write([byte]0)
$writer.Write([byte]0)
$writer.Write([byte]0)
$writer.Write([uint16]1)
$writer.Write([uint16]32)
$writer.Write([uint32]$pngBytes.Length)
$writer.Write([uint32]22)
$writer.Write($pngBytes)
$writer.Dispose()
$file.Dispose()
$pngStream.Dispose()
$dotBrush.Dispose()
$orbitPen.Dispose()
$background.Dispose()
$graphics.Dispose()
$bitmap.Dispose()
