$root = Get-Location
$src = Join-Path $root 'src'
if (-not (Test-Path $src)) { Write-Output (ConvertTo-Json @{ error = 'src directory not found'; root = $root }); exit 1 }

$patterns = @('\.bodies\.push\(', '\.bodies\.insert\(', '\.bodies\.remove\(', '\.bodies\.clear\(', '\.bodies\.pop\(', '\.bodies\.swap_remove\(')

$issues = @()

Get-ChildItem -Path $src -Recurse -Filter *.rs | ForEach-Object {
    $path = $_.FullName
    try {
        $lines = [System.IO.File]::ReadAllLines($path)
    } catch {
        Write-Host ("Failed to read {0}: {1}" -f $path, $_)
        return
    }

    $text = $lines -join "`n"
    # find impl Simulation ranges
    $impl_ranges = @()
    $index = 0
    for ($i = 0; $i -lt $lines.Length; $i++) {
        if ($lines[$i] -match '\bimpl\s+Simulation\s*{') {
            $start = $i
            $brace = 0
            for ($j = $i; $j -lt $lines.Length; $j++) {
                $brace += ([regex]::Matches($lines[$j], '{').Count)
                $brace -= ([regex]::Matches($lines[$j], '}').Count)
                if ($j -eq $i) {
                    # we've seen the opening '{' on same line - ensure count decremented correctly
                    # continue until brace-balanced to 0
                }
                if ($brace -le 0 -and $j -gt $i) {
                    $end = $j
                    break
                }
                # if loop ends without break, end is last line
                $end = $lines.Length - 1
            }
            $impl_ranges += @{ start = $start; end = $end }
        }
    }

    for ($i = 0; $i -lt $lines.Length; $i++) {
        $raw = $lines[$i]
        $line = $raw.Trim()
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line.StartsWith('//') -or $line.StartsWith('/*')) { continue }

        $is_mutation = $false
        foreach ($p in $patterns) {
            if ($line -match $p) { $is_mutation = $true; break }
        }

        if (-not $is_mutation) {
            if ($line -match 'self\.bodies\[') {
                $idx = $line.IndexOf('self.bodies[')
                $rb = $line.IndexOf(']', $idx)
                if ($rb -ge 0) {
                    $after = $line.Substring($rb + 1)
                    if ($after.TrimStart().StartsWith('=')) { $is_mutation = $true }
                }
            }
            if (-not $is_mutation -and $line -match 'simulation.bodies') {
                $pos = $line.IndexOf('simulation.bodies')
                $after = $line.Substring($pos + 'simulation.bodies'.Length)
                if ($after.TrimStart().StartsWith('=')) { $is_mutation = $true }
            }
        }

        if (-not $is_mutation) {
            if (($line -match '\.mass') -or ($line -match '\.segment_type')) {
                if ($line -match 'self\.bodies' -or $line -match 'simulation.bodies' -or $line -match '\.bodies\[' -or $line -match '\.bodies\.') {
                    if ($line -match '\.mass') {
                        $pos = $line.IndexOf('.mass')
                        if ($line.IndexOf('=', $pos) -ge 0) { $is_mutation = $true }
                    }
                    if (-not $is_mutation -and $line -match '\.segment_type') {
                        $pos = $line.IndexOf('.segment_type')
                        if ($line.IndexOf('=', $pos) -ge 0) { $is_mutation = $true }
                    }
                }
            } elseif ($line -match 'body\.mass' -or $line -match 'body\.segment_type') {
                $bind_lookback = 12
                $start = [Math]::Max(0, $i - $bind_lookback)
                $found_bind = $false
                for ($k = $start; $k -lt $i; $k++) {
                    $bl = $lines[$k].Trim()
                    if ($bl -match 'for\s+' -and ($bl -match 'in\s+&mut\s+self\.bodies' -or $bl -match 'in\s+&mut\s+simulation.bodies' -or $bl -match 'in\s+&mut\s+sim.bodies')) {
                        $found_bind = $true; break
                    }
                    if ($bl -match 'let\s+' -and $bl -match '=\s*&mut' -and ($bl -match 'self\.bodies\[' -or $bl -match 'simulation.bodies\[')) {
                        $found_bind = $true; break
                    }
                }
                if ($found_bind -and $line.Contains('=')) { $is_mutation = $true }
            }
        }

        if (-not $is_mutation) { continue }

        $lookback = 40
        $start_check = [Math]::Max(0, $i - $lookback)
        $found_instr = $false
        for ($j = $start_check; $j -lt $i; $j++) {
            $ll = $lines[$j].Trim()
            if ($ll -match 'record_gpdm_event' -or $ll -match 'record_body_state_change') { $found_instr = $true; break }
        }
        if (-not $found_instr) {
            $in_sim_impl = $false
            foreach ($r in $impl_ranges) {
                if ($r.start -le $i -and $i -le $r.end) { $in_sim_impl = $true; break }
            }
            $issues += [PSCustomObject]@{ file = $path; line_no = $i + 1; line = $line; in_impl_simulation = $in_sim_impl }
        }
    }
}

$report = [PSCustomObject]@{ root = (Get-Location).Path; issues = $issues }
$report | ConvertTo-Json -Depth 5 | Write-Output
