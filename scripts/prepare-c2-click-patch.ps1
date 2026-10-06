param(
    [Parameter(Mandatory = $true)][string]$OmsiRoot,
    [Parameter(Mandatory = $true)][string]$OutputRoot
)

# Optional content adjustment for the user's MB_C2_EN_BVG recordings, not engine DSP.
# Write an overlay root; never overwrite the original installation.
$ErrorActionPreference = 'Stop'
$originalRoot = [IO.Path]::GetFullPath($OmsiRoot).TrimEnd('\', '/')
$patchRoot = [IO.Path]::GetFullPath($OutputRoot).TrimEnd('\', '/')
if ($patchRoot.Equals($originalRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $patchRoot.StartsWith($originalRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'OutputRoot must be outside the original OMSI installation.'
}
$ffmpegPath = (Get-Command ffmpeg -ErrorAction Stop).Source
$relativeFolder = 'Vehicles\MB_C2_EN_BVG\Sound-10er\Cockpit'
$sourceFolder = Join-Path $originalRoot $relativeFolder
$destinationFolder = Join-Path $patchRoot $relativeFolder
foreach ($clipName in 'D_blinker_on.wav', 'D_blinker_off.wav') {
    if (-not (Test-Path -LiteralPath (Join-Path $sourceFolder $clipName) -PathType Leaf)) {
        throw "Missing C2 recording: $clipName"
    }
}
New-Item -ItemType Directory -Path $destinationFolder -Force | Out-Null
foreach ($clipName in 'D_blinker_on.wav', 'D_blinker_off.wav') {
    $sourceClip = Join-Path $sourceFolder $clipName
    $destinationClip = Join-Path $destinationFolder $clipName
    # Suppress the two ringing regions; keep the impact and smoothly remove the
    # noisy recorded tail after 60 ms. Preserve rate, channels and file duration.
    & $ffmpegPath -v error -i $sourceClip -af `
        'equalizer=f=1500:t=h:w=400:g=-18,equalizer=f=3020:t=h:w=400:g=-18,afade=t=out:st=0.06:d=0.06' `
        -acodec pcm_s16le -y $destinationClip
    if ($LASTEXITCODE -ne 0) { throw "Failed to process $clipName" }
    [PSCustomObject]@{
        Source = $sourceClip
        SourceSHA256 = (Get-FileHash -LiteralPath $sourceClip -Algorithm SHA256).Hash
        Output = $destinationClip
        OutputSHA256 = (Get-FileHash -LiteralPath $destinationClip -Algorithm SHA256).Hash
    }
}
