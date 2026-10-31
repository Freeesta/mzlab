"""Static server of the built site for the e2e tests: `python -m http.server`, with a listen queue long enough for the many parallel
connections of a browser (the default of five makes Windows and macOS refuse some of them under load: the scripts of the page fail to load)."""
import sys, functools
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler

class Server(ThreadingHTTPServer):
    request_queue_size = 256
    daemon_threads = True

    def handle_error(self, request, client_address):
        # the browser aborts a download when a page closes or navigates (a large .wasm in flight): not an error of the site
        if isinstance(sys.exc_info()[1], ConnectionError): return
        super().handle_error(request, client_address)

if __name__ == "__main__":
    port, root = int(sys.argv[1]), sys.argv[2]
    Server(("127.0.0.1", port), functools.partial(SimpleHTTPRequestHandler, directory=root)).serve_forever()
