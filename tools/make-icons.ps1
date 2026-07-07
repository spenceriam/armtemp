# Generates the ARMTEMP icon set (PNG + ICO) from scratch using .NET drawing.
# Produces a 150deg gradient tile (accent blue) with a simple thermometer glyph —
# the same mark used in the Claude design.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$outDir = 'C:\Users\spenc\GitHub\armtemp\src-tauri\icons'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

function New-Bitmap($size) {
    $b = New-Object System.Drawing.Bitmap($size, $size)
    $b.SetResolution(96, 96)
    return $b
}

function Draw-Icon($size) {
    $b = New-Bitmap $size
    $g = [System.Drawing.Graphics]::FromImage($b)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    # Rounded square tile with 150deg gradient (#0078D4 -> darker).
    $pad = [int]($size * 0.06)
    $tileSize = $size - (2 * $pad)
    $rect = New-Object System.Drawing.Rectangle($pad, $pad, $tileSize, $tileSize)
    $radius = [int]($size * 0.18)
    $gp = New-Object System.Drawing.Drawing2D.GraphicsPath
    $gp.AddArc($rect.X, $rect.Y, $radius, $radius, 180, 90)
    $gp.AddArc($rect.Right - $radius, $rect.Y, $radius, $radius, 270, 90)
    $gp.AddArc($rect.Right - $radius, $rect.Bottom - $radius, $radius, $radius, 0, 90)
    $gp.AddArc($rect.X, $rect.Bottom - $radius, $radius, $radius, 90, 90)
    $gp.CloseFigure()

    $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
      (New-Object System.Drawing.Point($rect.X, $rect.Y)),
      (New-Object System.Drawing.Point($rect.Right, $rect.Bottom)),
      [System.Drawing.ColorTranslator]::FromHtml('#0078D4'),
      [System.Drawing.ColorTranslator]::FromHtml('#00548F'))
    $g.FillPath($grad, $gp)

    # Thermometer: vertical stem + bulb, white.
    $cx = $size / 2
    $stemW = [int]($size * 0.10)
    $stemH = [int]($size * 0.40)
    $stemX = $cx - $stemW / 2
    $stemY = [int]($size * 0.22)
    $bulbR = [int]($size * 0.16)
    $white = [System.Drawing.Color]::White

    $stemRect = New-Object System.Drawing.Rectangle($stemX, $stemY, $stemW, $stemH)
    $g.FillRectangle((New-Object System.Drawing.SolidBrush($white)), $stemRect)
    $bulbRect = New-Object System.Drawing.RectangleF(($cx - $bulbR), ($stemY + $stemH - $bulbR * 0.4), ($bulbR*2), ($bulbR*2))
    $g.FillEllipse((New-Object System.Drawing.SolidBrush($white)), $bulbRect)
    # Round the stem top.
    $g.FillEllipse((New-Object System.Drawing.SolidBrush($white)), (New-Object System.Drawing.RectangleF(($stemX - $stemW*0.3), ($stemY - $stemW*0.3), ($stemW*1.6), ($stemW*1.6))))

    $g.Dispose()
    return $b
}

# PNG outputs.
foreach ($s in @(32, 128, 256)) {
    $b = Draw-Icon $s
    $b.Save("$outDir\$($s)x$($s).png", [System.Drawing.Imaging.ImageFormat]::Png)
    if ($s -eq 128) { $b.Save("$outDir\128x128@2x.png", [System.Drawing.Imaging.ImageFormat]::Png) }
    if ($s -eq 256) { $b.Save("$outDir\icon.png", [System.Drawing.Imaging.ImageFormat]::Png) }
    $b.Dispose()
}

# ICO (multi-size).
$ico = "$outDir\icon.ico"
$ms = New-Object System.IO.FileStream($ico, [System.IO.FileMode]::Create)
$writer = New-Object System.IO.BinaryWriter($ms)
$sizes = @(16, 32, 48, 64, 128, 256)
# ICONDIR header
$writer.Write([UInt16]0)      # reserved
$writer.Write([UInt16]1)      # type: icon
$writer.Write([UInt16]$sizes.Count)
$entries = @()
$imgs = @()
foreach ($sz in $sizes) {
    $b = Draw-Icon $sz
    $ms2 = New-Object System.IO.MemoryStream
    $b.Save($ms2, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms2.ToArray()
    $ms2.Dispose()
    $imgs += ,$bytes
    $entries += ,@{ w=if($sz -eq 256){0}else{$sz}; h=if($sz -eq 256){0}else{$sz}; size=$bytes.Length; offset=0 }
}
# DIRENTRYs (6 * 16 bytes), then images.
$dataOffset = 6 + 16 * $sizes.Count
for ($i=0; $i -lt $entries.Count; $i++) {
    $e = $entries[$i]
    $writer.Write([byte]$e.w)
    $writer.Write([byte]$e.h)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]32)
    $writer.Write([UInt32]$e.size)
    $writer.Write([UInt32]($dataOffset))
    $dataOffset += $e.size
}
foreach ($bytes in $imgs) { $writer.Write($bytes) }
$writer.Dispose()
$ms.Dispose()

# Placeholder .icns (Tauri wants it listed; macOS only, not built here).
Copy-Item "$outDir\icon.png" "$outDir\icon.icns" -Force

Write-Host "Icons generated in $outDir"
Get-ChildItem $outDir | Select-Object Name, Length | Format-Table -AutoSize
