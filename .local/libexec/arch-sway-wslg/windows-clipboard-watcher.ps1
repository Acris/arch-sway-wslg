$ErrorActionPreference = "Stop"

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ClipboardNative {
    [DllImport("user32.dll")]
    public static extern uint GetClipboardSequenceNumber();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsClipboardFormatAvailable(uint format);
}
'@

function Write-ProtocolLine([string] $Line) {
    [Console]::Out.Write($Line + "`n")
    [Console]::Out.Flush()
}

function Try-EmitClipboardText {
    # CF_HDROP is a file list. N means the sequence was handled without a
    # supported text payload, allowing the Linux side to finish initialization.
    if ([ClipboardNative]::IsClipboardFormatAvailable(15)) {
        Write-ProtocolLine "N:"
        return $true
    }
    if (-not [ClipboardNative]::IsClipboardFormatAvailable(13) -and
        -not [ClipboardNative]::IsClipboardFormatAvailable(1)) {
        Write-ProtocolLine "N:"
        return $true
    }

    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        try {
            $text = Get-Clipboard -Format Text -Raw -ErrorAction Stop
            if ($null -eq $text) {
                Write-ProtocolLine "N:"
                return $true
            }
            $text = ([string] $text).Replace("`r`n", "`n")
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($text)
            Write-ProtocolLine ("B:" + [Convert]::ToBase64String($bytes))
            return $true
        } catch {
            Start-Sleep -Milliseconds 25
        }
    }
    return $false
}

$pollMilliseconds = 75
if ($env:ARCH_SWAY_WSLG_CLIPBOARD_POLL_MS -match '^[0-9]+$') {
    $pollMilliseconds = [int] $env:ARCH_SWAY_WSLG_CLIPBOARD_POLL_MS
}

$haveLast = $false
$last = [uint32] 0
while ($true) {
    $current = [ClipboardNative]::GetClipboardSequenceNumber()
    if (-not $haveLast -or $current -ne $last) {
        if (Try-EmitClipboardText) {
            $last = $current
            $haveLast = $true
        }
    }
    Start-Sleep -Milliseconds $pollMilliseconds
}
