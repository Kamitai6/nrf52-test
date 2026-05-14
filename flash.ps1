param(
    [Parameter(Mandatory = $true)]
    [string]$UF2Path
)

$vol = Get-Volume | Where-Object { $_.FileSystemLabel -eq "XIAO-SENSE" } | Select-Object -First 1

if ($null -eq $vol) {
    Write-Error "XIAO-SENSE drive not found. Please enter bootloader mode with double-tap."
    exit 1
}

$drive = $vol.DriveLetter + ":\"

if (-not (Test-Path "${drive}INFO_UF2.TXT")) {
    Write-Error "INFO_UF2.TXT not found on drive."
    exit 1
}

$absUF2 = Resolve-Path $UF2Path
cmd /c "copy /b `"$absUF2`" `"$drive`""
if ($LASTEXITCODE -ne 0) {
    Write-Error "Copy failed."
    exit 1
}
Write-Host "Flash done: $drive"