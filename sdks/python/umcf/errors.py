"""errors.py — Structured Python exceptions matching RFC 9457 Problem Details."""

from typing import Any, Dict, Optional


class UmcfError(Exception):
    """Base exception for all UMCF SDK operations."""

    def __init__(
        self,
        status_code: int,
        message: str,
        problem_details: Optional[Dict[str, Any]] = None,
    ):
        super().__init__(message)
        self.status_code = status_code
        self.message = message
        self.problem_details = problem_details or {}


class UnauthorizedError(UmcfError):
    """Raised when Bearer capability token is missing, invalid, or expired."""
    pass


class ForbiddenError(UmcfError):
    """Raised when Bearer capability token lacks required capability scope."""
    pass


class NotFoundError(UmcfError):
    """Raised when target cluster resource or storage object is not found."""
    pass


class RateLimitError(UmcfError):
    """Raised when rate limit bucket is exhausted."""

    def __init__(self, message: str, retry_after_ms: int = 1000):
        super().__init__(429, message)
        self.retry_after_ms = retry_after_ms
