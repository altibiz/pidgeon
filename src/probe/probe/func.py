import functools
from typing import Callable, Any


def compose(*fs: Callable[..., Any]) -> Callable[..., Any]:
  return functools.reduce(lambda f, g: lambda *a, **kw: g(f(*a, **kw)), fs)
