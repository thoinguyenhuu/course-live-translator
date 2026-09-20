$exePath = "rust-app\target\release\coursera-rust-translator.exe"
$stdoutPath = "bench_stdout.log"
$stderrPath = "bench_stderr.log"

Remove-Item -Path $stdoutPath -ErrorAction SilentlyContinue
Remove-Item -Path $stderrPath -ErrorAction SilentlyContinue

$proc = Start-Process -FilePath $exePath -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
Start-Sleep -Seconds 2

# Measure idle RAM
$idleMem = (Get-Process -Id $proc.Id).WorkingSet64 / 1MB
Write-Host "Idle RAM: $([math]::Round($idleMem, 2)) MB"

# Run simulation using native Rust simulate client
$simJob = Start-Job -ScriptBlock {
    Set-Location "$using:PWD\rust-app"
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" run --bin simulate
}

$peakMem = $idleMem
$memSamples = @()

while ($simJob.State -eq 'Running') {
    Start-Sleep -Milliseconds 500
    try {
        $curMem = (Get-Process -Id $proc.Id -ErrorAction Stop).WorkingSet64 / 1MB
        $memSamples += $curMem
        if ($curMem -gt $peakMem) { $peakMem = $curMem }
    } catch {
        break
    }
}

# Wait another 8 seconds for last audio segment to finish processing and playing
Start-Sleep -Seconds 8

Receive-Job $simJob | Out-Null
Remove-Job $simJob -Force

Stop-Process -Id $proc.Id -Force

$avgMem = ($memSamples | Measure-Object -Average).Average

Write-Host "=== BENCHMARK RAM RESULT ==="
Write-Host "Idle RAM: $([math]::Round($idleMem, 2)) MB"
Write-Host "Steady/Avg RAM: $([math]::Round($avgMem, 2)) MB"
Write-Host "Peak RAM: $([math]::Round($peakMem, 2)) MB"
