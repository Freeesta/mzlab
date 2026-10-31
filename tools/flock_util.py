"""Blocking exclusive file lock shared by tests and tools: fcntl.flock on Linux/macOS, msvcrt.locking on Windows."""
try:
    import fcntl

    def lock_exclusive(f):
        """Block until `f` (an open file) is exclusively locked; the lock is released when `f` is closed."""
        fcntl.flock(f, fcntl.LOCK_EX)
except ImportError:  # Windows
    import msvcrt, time

    def lock_exclusive(f):
        """Block until `f` is exclusively locked (byte 0); released when `f` is closed."""
        f.seek(0)
        while True:
            try:
                msvcrt.locking(f.fileno(), msvcrt.LK_NBLCK, 1)
                return
            except OSError:
                time.sleep(0.1)
