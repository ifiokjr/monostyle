import functools


@functools.lru_cache(maxsize=None)
def compute(radius):
    return radius * 2


def register(route="/index", methods=("GET",)):
    def wrap(fn):
        return fn

    return wrap
