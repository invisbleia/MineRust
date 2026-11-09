# Spike dev server: correct .wasm MIME + COOP/COEP headers (for wasm threads later).
# No Python needed — plain .NET HttpListener.
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add('http://127.0.0.1:8765/')
$listener.Start()
Write-Host "serving $root on http://127.0.0.1:8765"
while ($listener.IsListening) {
    $ctx = $listener.GetContext()
    $path = $ctx.Request.Url.AbsolutePath
    if ($path -eq '/') { $path = '/index.html' }
    $file = Join-Path $root ($path.TrimStart('/', '\') -replace '/', '\')
    if (Test-Path $file -PathType Leaf) {
        $bytes = [IO.File]::ReadAllBytes($file)
        $ext = [IO.Path]::GetExtension($file).ToLower()
        $mime = @{ '.html' = 'text/html'; '.js' = 'text/javascript'; '.wasm' = 'application/wasm'; '.d.ts' = 'text/plain'; '.ts' = 'text/plain' }[$ext]
        if (-not $mime) { $mime = 'application/octet-stream' }
        $ctx.Response.ContentType = $mime
        $ctx.Response.Headers['Cross-Origin-Opener-Policy'] = 'same-origin'
        $ctx.Response.Headers['Cross-Origin-Embedder-Policy'] = 'require-corp'
        $ctx.Response.ContentLength64 = $bytes.Length
        $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
    } else {
        $ctx.Response.StatusCode = 404
    }
    $ctx.Response.Close()
}
