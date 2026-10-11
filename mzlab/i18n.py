"""Messages for the interface without a language: the server never speaks Italian or English.

Python returns KEYS and parameters; the page translates them with ``I18N.t(key, params)`` (catalogs ``mzlab/web/lang/it.js`` and
``en.js``). An error caused by the user is a :class:`UserError`; ``api.dispatch`` answers
``{"error": <developer text in English>, "error_key": <key>, "params": {...}}``.
"""
from __future__ import annotations


class UserError(ValueError):
    """An error the user can fix (wrong file, nothing selected...). ``key`` names the message in the catalogs, ``params`` fills it."""

    def __init__(self, key: str, params: dict | None = None, text: str | None = None):
        self.key = key
        self.params = dict(params or {})
        super().__init__(text or key)

    def to_json(self) -> dict:
        return {"error": str(self), "error_key": self.key, "params": self.params}


def message(key: str, **params) -> dict:
    """A message for the interface (not an error): ``{"key": ..., "params": {...}}``."""
    return {"key": key, "params": params}
