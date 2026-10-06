# Run from a developer shell with MSVC/Windows SDK available.
# Synthetic timing/process memory is written alongside the offline WAV. Live map QA is
# still required; see docs/audio/STAGE6_TESTING.md.
param([string]$RadioUrl = "")
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
Push-Location $workspace
try {
    & cargo check -p audio --all-targets --locked
    if ($LASTEXITCODE) { throw 'audio check failed' }
    & cargo check -p simulation --lib --locked
    if ($LASTEXITCODE) { throw 'sim check failed' }
    if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) {
        & cargo nextest run -p audio --locked
    } else {
        & cargo test -p audio --locked
    }
    if ($LASTEXITCODE) { throw 'audio tests failed' }
    & cargo check --workspace --all-targets --locked
    if ($LASTEXITCODE) { throw 'workspace check failed' }
    & cargo build -p core --release --locked
    if ($LASTEXITCODE) { throw 'release application build failed' }
    & cargo build -p audio --release --examples --locked
    if ($LASTEXITCODE) { throw 'release audio examples failed' }
    $resultsDir = Join-Path $workspace 'out/audio-stage6'
    New-Item -ItemType Directory -Path $resultsDir -Force | Out-Null
    $wave = Join-Path $resultsDir 'offline.wav'
    & cargo run -p audio --release --example offline_render --locked -- crates/audio/tests/fixtures/soundcfg/trigger.cfg 2 $wave
    if ($LASTEXITCODE) { throw 'offline render failed' }
    $executable = Join-Path $workspace 'target/release/examples/audio_load.exe'
    $metrics = foreach ($voiceCount in 32, 100, 200, 500) {
        $stdout = Join-Path $resultsDir "load-$voiceCount.csv"
        $stderr = Join-Path $resultsDir "load-$voiceCount.stderr.txt"
        $process = Start-Process -FilePath $executable -ArgumentList "$voiceCount" -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        $peak = 0L
        while (-not $process.HasExited) {
            $process.Refresh()
            $peak = [Math]::Max($peak, $process.PeakWorkingSet64)
            Start-Sleep -Milliseconds 50
        }
        $process.WaitForExit()
        if ($process.ExitCode) { throw "load probe $voiceCount failed" }
        $cpuMs = $null
        try { $cpuMs = $process.TotalProcessorTime.TotalMilliseconds } catch { }
        [PSCustomObject]@{ voices=$voiceCount; peak_working_set_bytes=$peak; process_cpu_ms=$cpuMs; csv=$stdout }
    }
    $metrics | Export-Csv -NoTypeInformation -Path (Join-Path $resultsDir 'process-memory.csv')
    if ($RadioUrl) {
        & cargo run -p audio --release --example radio_probe --locked -- $RadioUrl
        if ($LASTEXITCODE) { throw 'radio probe failed' }
    }
} finally { Pop-Location }
