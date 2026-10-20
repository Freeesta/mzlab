"""The one error type of the Libro converter: always carries file and line, so an unsupported construct stops the build instead of being guessed."""


class LibroError(Exception):
    def __init__(self, file: str, line: int, msg: str):
        super().__init__(f"{file}:{line}: {msg}")
        self.file, self.line, self.msg = file, line, msg
