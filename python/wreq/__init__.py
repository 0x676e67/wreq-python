# wreq/__init__.py

from .wreq import *

from .cookie import *
from .exceptions import *
from .header import *
from .emulation import *
from .http1 import *
from .http2 import *
from .tls import *
from .dns import *
from .redirect import *
from .proxy import *
from .runtime import *

import sys as _sys

if _sys.implementation.name == "pypy":
    from ._compat import _install

    # Creating and closing an unpolled coroutine does not start a request or runtime.
    _coroutine = get("")
    try:
        _install(type(_coroutine))
    finally:
        _coroutine.close()
    del _coroutine, _install

del _sys

__all__ = (
    header.__all__
    + cookie.__all__
    + emulation.__all__
    + exceptions.__all__
    + http1.__all__
    + http2.__all__
    + tls.__all__
    + dns.__all__
    + redirect.__all__
    + proxy.__all__
    + runtime.__all__
)  # type: ignore
