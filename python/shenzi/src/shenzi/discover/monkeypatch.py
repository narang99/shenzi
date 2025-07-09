
import importlib
from typing import Any
import logging


logger = logging.getLogger(__name__)

def _get_element(mod, attrs):
    el = mod
    for attr in attrs:
        el = getattr(el, attr)
    return el


def _set_element(mod, attrs, el):
    parent = _get_element(mod, attrs[:-1])
    setattr(parent, attrs[-1], el)


def _monkey_patch(mod, attrs, add_lib_callback, args_to_path):
    try:
        original_fn = _get_element(mod, attrs)

        def new_fn(*args, **kwargs):
            path = args_to_path(*args, **kwargs)
            result = original_fn(*args, **kwargs)
            if not result or not path:
                return result
            logger.info("adding %s", path)
            add_lib_callback(path)
            return result

        _set_element(mod, attrs, new_fn)
    except AttributeError as ex:
        print("failed in patching", mod, attrs, ex)


def try_monkey_patch(mod, attrs, add_lib_callback, args_to_path):
    try:
        mod_ = importlib.import_module(mod)
        _monkey_patch(mod_, attrs, add_lib_callback, args_to_path)
        print("patched", mod, attrs)
    except ImportError as ex:
        print("failed in importing", mod, ex)


def kwarg_else_arg(var: str, i: int) -> Any:
    """return a function which tries to find `kwargs[var]` or `args[i]` from its arguments

    When we monkey patch a function like `dlopen`, we want to infer the path that `dlopen` was trying to open
    We intercept its args and given *args and **kwargs, try to find the potential path the args point to

    if `var` is in kwargs, that is returned, else `args[i]` is returned

    NOTE: this returns a relative path if the args themselves get a relative path, see what we can do about it
    Maybe also try to infer from return types?
    """
    def _f(*a, **kw):
        return kw[var] if var in kw else a[i]

    return _f