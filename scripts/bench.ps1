param(
    [string]$BeforeCommit = "HEAD~9",
    [string]$AfterCommit = "HEAD",
    [string]$MapPath = "assets/maps/temp_playtest.vrtx",
    [int]$Frames = 500
)

$ErrorActionPreference = "Stop"
$repo = (Get-Location).Path

function Run-Bench {
    param([string]$label, [string]$commit)
    Write-Host "=== BENCHMARK: $label (commit $commit) ===" -ForegroundColor Cyan
    git checkout $commit 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to checkout $commit" }
    Write-Host "  Building release with bench feature..."
    cargo build --release --features bench --bin RaveEngineServer 2>&1 | Select-String "Compiling|Warning|Error" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $commit" }
    Write-Host "  Running benchmark ($Frames frames)..."
    $result = & .\target\release\RaveEngineServer.exe --benchmark --bench-frames $Frames --map $MapPath 2>&1
    $jsonLine = $result | Select-String '^\{"total_frames"'
    if (-not $jsonLine) { throw "No JSON output from benchmark for $commit" }
    $outFile = "bench_$($label -replace '[^a-zA-Z0-9]','_').json"
    $jsonLine.Line | Out-File -Encoding utf8 $outFile
    Write-Host "  -> $outFile" -ForegroundColor Green
    Write-Host $jsonLine.Line
}

Push-Location $repo
try {
    $beforeFile = Run-Bench -label "before" -commit $BeforeCommit
    $afterFile = Run-Bench -label "after" -commit $AfterCommit

    Write-Host ""
    Write-Host "=== COMPARISON ===" -ForegroundColor Yellow

    $before = Get-Content $beforeFile | ConvertFrom-Json
    $after = Get-Content $afterFile | ConvertFrom-Json

    function Pct { param($old, $new)
        if ($old -eq 0) { return "n/a" }
        $pct = [math]::Round(($new - $old) / $old * 100, 1)
        if ($pct -lt 0) { return "$pct% (faster)" }
        return "+$pct% (slower)"
    }

    Write-Host ("{'Metric',-28} {'Before',-16} {'After',-16} {'Delta'}" -f "", "", "", "")
    Write-Host ("{0,-28} {1,-16} {2,-16} {3}" -f "---", "---", "---", "---")
    Write-Host ("{0,-28} {1,16} {2,16}" -f "total_frames", $before.total_frames, $after.total_frames)
    Write-Host ("{0,-28} {1,16:N0}ns {2,16:N0}ns {3}" -f "avg_frame_time", $before.avg_frame_ns, $after.avg_frame_ns, (Pct $before.avg_frame_ns $after.avg_frame_ns))
    Write-Host ("{0,-28} {1,16} {2,16} {3}" -f "find_service_calls", $before.find_service_calls, $after.find_service_calls, "")
    Write-Host ("{0,-28} {1,16} {2,16} {3}" -f "find_service_scan_calls", $before.find_service_scan_calls, $after.find_service_scan_calls, (Pct $before.find_service_scan_calls $after.find_service_scan_calls))
    Write-Host ("{0,-28} {1,16:N0}ns {2,16:N0}ns {3}" -f "find_service_total", $before.find_service_ns, $after.find_service_ns, (Pct $before.find_service_ns $after.find_service_ns))
    Write-Host ("{0,-28} {1,16:N0}ns {2,16:N0}ns {3}" -f "event_dispatch", $before.event_dispatch_ns, $after.event_dispatch_ns, (Pct $before.event_dispatch_ns $after.event_dispatch_ns))
    Write-Host ("{0,-28} {1,16:N0}ns {2,16:N0}ns {3}" -f "scheduler", $before.scheduler_ns, $after.scheduler_ns, (Pct $before.scheduler_ns $after.scheduler_ns))
    Write-Host ("{0,-28} {1,16:N0}ns {2,16:N0}ns {3}" -f "collision", $before.collision_ns, $after.collision_ns, (Pct $before.collision_ns $after.collision_ns))
} finally {
    Write-Host ""
    Write-Host "=== Restoring original commit ($AfterCommit) ===" -ForegroundColor Cyan
    git checkout $AfterCommit 2>&1 | Out-Null
    Pop-Location
}
