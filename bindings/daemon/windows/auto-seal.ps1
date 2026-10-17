$bin = "C:\Users\rise\.rise\bin\rise-core.exe"
$key = "C:\Users\rise\.rise\keys\auto.key"
$log = "C:\Users\rise\.rise\daemon.log"
$known = @{}

function Seal($file) {
    $ext = [System.IO.Path]::GetExtension($file).ToLower()
    $out = if ($ext -in ".mp4",".mkv",".mov",".avi") { "$file.rvid" } else { "$file.rise" }
    if (Test-Path $out) { return }
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Add-Content $log "$ts SEAL $file"
    if ($ext -in ".mp4",".mkv",".mov",".avi") {
        # Fail-closed: folder drops are not camera captures.
        & $bin video $file --device auto --key $key --capture-source screen --out $out 2>>$log
    } elseif ($ext -in ".jpg",".jpeg",".png") {
        $tmp = "C:\Users\rise\.rise\tmp-manifest.json"
        & $bin sign $file --device auto --key $key --capture-source screen --out $tmp 2>>$log
        & $bin pack $file $tmp --out $out 2>>$log
        Remove-Item $tmp -ErrorAction SilentlyContinue
    }
    if (Test-Path $out) { Add-Content $log "$ts DONE $out" }
}

$dirs = @()
Get-ChildItem "C:\Users" -Directory -ErrorAction SilentlyContinue | ForEach-Object {
    $dirs += Join-Path $_.FullName "Desktop"
    $dirs += Join-Path $_.FullName "Videos"
    $dirs += Join-Path $_.FullName "Pictures\Screenshots"
}
$dirs = $dirs | Where-Object { Test-Path $_ }

Add-Content $log "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') START v2.2 watching $($dirs.Count) folders"
while ($true) {
    foreach ($dir in $dirs) {
        $files = Get-ChildItem $dir -File -ErrorAction SilentlyContinue | Where-Object {
            $_.Extension.ToLower() -in ".mp4",".mkv",".mov",".avi",".jpg",".jpeg",".png" -and
            -not $_.Name.EndsWith(".rise") -and
            -not $_.Name.EndsWith(".rvid") -and
            -not $_.Name.EndsWith(".manifest.json") -and
            -not $_.Name.StartsWith("rise-tmp")
        }
        foreach ($f in $files) {
            $p = $f.FullName
            if (-not $known.ContainsKey($p)) {
                $known[$p] = @{Size=$f.Length;Count=0}
            } else {
                if ($known[$p].Size -eq $f.Length) {
                    $known[$p].Count++
                    if ($known[$p].Count -ge 2) { Seal $p; $known.Remove($p) }
                } else {
                    $known[$p] = @{Size=$f.Length;Count=0}
                }
            }
        }
    }
    Start-Sleep -Seconds 10
}
