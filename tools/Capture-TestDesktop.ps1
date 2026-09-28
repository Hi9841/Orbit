param([Parameter(Mandatory)][string]$Output, [int]$Left, [int]$Top, [int]$Width, [int]$Height)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Desktop capture is restricted to the disposable hosted test runner.' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
if ($Width -gt 0 -and $Height -gt 0) {
    $requested = New-Object System.Drawing.Rectangle($Left, $Top, $Width, $Height)
    $bounds = [System.Drawing.Rectangle]::Intersect($bounds, $requested)
}
$bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save($Output, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
