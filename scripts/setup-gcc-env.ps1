param(
    [string]$ToolBin = "D:\Battery\tools\w64devkit\w64devkit\bin"
)

if (-not (Test-Path (Join-Path $ToolBin "gcc.exe"))) {
    Write-Error "gcc.exe not found in $ToolBin"
    exit 1
}

$env:Path = "$ToolBin;$env:Path"
$env:CC = Join-Path $ToolBin "gcc.exe"
$env:CGO_ENABLED = "1"

Write-Output "Configured GCC toolchain for current shell:"
Write-Output "  PATH includes: $ToolBin"
Write-Output "  CC=$env:CC"
Write-Output "  CGO_ENABLED=$env:CGO_ENABLED"
