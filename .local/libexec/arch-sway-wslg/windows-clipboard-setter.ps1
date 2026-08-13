$ErrorActionPreference = "Stop"

function Write-ProtocolLine([string] $Line) {
    # The Linux reader accepts CRLF defensively, but the wire format is LF.
    [Console]::Out.Write($Line + "`n")
    [Console]::Out.Flush()
}

while (($message = [Console]::In.ReadLine()) -ne $null) {
    if (-not $message.StartsWith("B:")) { continue }
    $separator = $message.IndexOf(":", 2)
    if ($separator -ne 66) { continue }
    $hash = $message.Substring(2, 64)
    $payload = $message.Substring($separator + 1)

    try {
        $bytes = [Convert]::FromBase64String($payload)
        $text = [System.Text.Encoding]::UTF8.GetString($bytes)
    } catch {
        [Console]::Error.WriteLine("clipboard decode failed: " + $_.Exception.Message)
        Write-ProtocolLine ("ERR:" + $hash)
        continue
    }

    $written = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        try {
            Set-Clipboard -Value $text -ErrorAction Stop
            $written = $true
            break
        } catch {
            Start-Sleep -Milliseconds 25
        }
    }

    if ($written) {
        Write-ProtocolLine ("OK:" + $hash)
    } else {
        [Console]::Error.WriteLine("clipboard write failed after retries")
        Write-ProtocolLine ("ERR:" + $hash)
    }
}
