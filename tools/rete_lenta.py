"""Slow link for a local test server: one downlink shared by every connection (bytes / rate, in chunks) and a fixed delay before each answer.
Wraps any http.server handler class; used by tests_e2e/misura_lazy_lento.py to model a slow first visit (CDP network emulation does not reach
the workers and is not applied reliably to the service worker)."""
from __future__ import annotations

import threading, time


class Link:
    """One downlink for all connections: each chunk waits for its turn on the line."""

    def __init__(self, rate_bytes: float):
        self.rate, self.lock, self.free = rate_bytes, threading.Lock(), 0.0

    def take(self, n: int) -> None:
        with self.lock:
            now = time.monotonic(); self.free = end = max(now, self.free) + n / self.rate
        time.sleep(end - now)


class _SlowFile:
    def __init__(self, raw, link: Link, chunk: int):
        self.raw, self.link, self.chunk = raw, link, chunk

    def write(self, b) -> int:
        b = bytes(b)
        for i in range(0, len(b), self.chunk):
            self.link.take(len(b[i:i + self.chunk])); self.raw.write(b[i:i + self.chunk])
        return len(b)

    def __getattr__(self, name):      # flush, close, closed: those of the socket file
        return getattr(self.raw, name)


def lento(handler: type, rate_bits: float, rtt_s: float, chunk: int = 4096) -> type:
    """A subclass of `handler` whose answers start `rtt_s` late and share one link of `rate_bits` bit/s."""
    link = Link(rate_bits / 8)

    class Slow(handler):
        def setup(self):
            super().setup(); self.wfile = _SlowFile(self.wfile, link, chunk)

        def do_GET(self):
            time.sleep(rtt_s); super().do_GET()

    return Slow
