"""A module demonstrating the Python scanner path."""

MAX_RETRIES = 3


def retry(operation, attempts=MAX_RETRIES):
    """Runs an operation, retrying on transient failure.

    Why this exists: the client fails transiently under load, and one retry policy
    here is better than scattering try/except across every call site.
    """
    for attempt in range(attempts):
        try:
            return operation()
        except TransientError as error:
            if attempt == attempts - 1:
                raise TooManyRetries(attempts) from error

    raise TooManyRetries(attempts)
