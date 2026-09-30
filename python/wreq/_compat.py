"""Compatibility for PyPy's legacy exception delegation to PyO3 coroutines."""

from types import TracebackType


def _install(coroutine_type):
    original = vars(coroutine_type)["throw"]
    if getattr(original, "_wreq_pypy_throw_compat", False):
        return

    # Remove this shim once PyO3 accepts throw(type, value, traceback).
    def throw(self, exc, value=None, traceback=None):
        if traceback is not None and type(traceback) is not TracebackType:
            raise TypeError("throw() third argument must be a traceback object")
        if issubclass(type(exc), BaseException):
            if value is not None:
                raise TypeError("instance exception may not have a separate value")
            instance = exc
            if traceback is not None:
                BaseException.with_traceback(instance, traceback)
        elif issubclass(type(exc), type) and issubclass(exc, BaseException):
            try:
                if issubclass(type(value), BaseException) and issubclass(
                    type(value), exc
                ):
                    instance = value
                elif value is None:
                    instance = exc()
                elif issubclass(type(value), tuple):
                    instance = exc(*value)
                else:
                    instance = exc(value)
            except BaseException as error:
                instance = error
                own_traceback = BaseException.__traceback__.__get__(error).tb_next
                BaseException.with_traceback(instance, own_traceback or traceback)
            else:
                if not issubclass(type(instance), BaseException):
                    instance = TypeError(
                        "exception constructor did not return an exception"
                    )
                BaseException.with_traceback(instance, traceback)
        else:
            raise TypeError("exceptions must derive from BaseException")
        return original(self, instance)

    throw._wreq_pypy_throw_compat = True
    coroutine_type.throw = throw
