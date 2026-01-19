"""Exception classes for ailoop-py."""


class AiloopError(Exception):
    """Base exception for ailoop-py errors."""


class ConnectionError(AiloopError):
    """Raised when connection to ailoop server fails."""


class ValidationError(AiloopError):
    """Raised when message validation fails."""


class TimeoutError(AiloopError):
    """Raised when operations timeout."""


class AuthenticationError(AiloopError):
    """Raised when authentication fails."""
