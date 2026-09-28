param([string]$Executable = "$PSScriptRoot\..\target\release\orbit.exe")
$ErrorActionPreference = 'Stop'
$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Executable))
function Read-U16([int]$Offset) { [BitConverter]::ToUInt16($bytes, $Offset) }
function Read-U32([int]$Offset) { [BitConverter]::ToUInt32($bytes, $Offset) }
if ($bytes.Length -lt 64 -or (Read-U16 0) -ne 0x5A4D) { throw 'Not a Windows PE executable' }
$pe = [int](Read-U32 0x3C)
if ((Read-U32 $pe) -ne 0x00004550) { throw 'PE signature missing' }
$machine = Read-U16 ($pe + 4)
$sections = Read-U16 ($pe + 6)
$optionalLength = Read-U16 ($pe + 20)
$optional = $pe + 24
if ($machine -ne 0x8664 -or (Read-U16 $optional) -ne 0x20B) { throw 'Expected x64 PE32+' }
$subsystem = Read-U16 ($optional + 68)
if ($subsystem -ne 2) { throw "Expected Windows GUI subsystem, got $subsystem" }
$sectionTable = $optional + $optionalLength
function Resolve-Rva([uint32]$Rva) {
    for ($index = 0; $index -lt $sections; $index++) {
        $section = $sectionTable + $index * 40
        $virtualSize = Read-U32 ($section + 8)
        $virtualAddress = Read-U32 ($section + 12)
        $rawSize = Read-U32 ($section + 16)
        $rawPointer = Read-U32 ($section + 20)
        if ($Rva -ge $virtualAddress -and $Rva -lt $virtualAddress + [Math]::Max($virtualSize, $rawSize)) {
            return [int]($rawPointer + $Rva - $virtualAddress)
        }
    }
    throw "RVA not found: $Rva"
}
$importsRva = Read-U32 ($optional + 120)
$imports = @()
if ($importsRva -ne 0) {
    $descriptor = Resolve-Rva $importsRva
    while ((Read-U32 ($descriptor + 12)) -ne 0) {
        $nameOffset = Resolve-Rva (Read-U32 ($descriptor + 12))
        $end = $nameOffset
        while ($end -lt $bytes.Length -and $bytes[$end] -ne 0) { $end++ }
        $imports += [System.Text.Encoding]::ASCII.GetString($bytes, $nameOffset, $end - $nameOffset)
        $descriptor += 20
    }
}
if ($imports | Where-Object { $_ -match '^(VCRUNTIME|MSVCP|concrt)' }) {
    throw 'Executable depends on the Visual C++ redistributable despite static CRT configuration'
}
'status: passed'
'architecture: x64'
'subsystem: windows_gui'
"imported_dlls: $($imports -join ', ')"
"size_bytes: $($bytes.Length)"
