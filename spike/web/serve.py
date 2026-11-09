"""Spike dev server: correct .wasm MIME, COOP/COEP headers (for wasm threads later)."""
import http.server

HOST, PORT = "127.0.0.1", 8765


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
    }

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()


if __name__ == "__main__":
    with http.server.ThreadingHTTPServer((HOST, PORT), Handler) as httpd:
        print(f"serving on http://{HOST}:{PORT}")
        httpd.serve_forever()
