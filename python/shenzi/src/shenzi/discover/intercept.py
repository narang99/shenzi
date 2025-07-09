from functools import partial
import json
import os
import atexit
from typing import Optional
from shenzi.discover.all_libs import get_libs
from shenzi.discover.callbacks import cffi_dlopen_callback, ctypes_cdll_callback
from shenzi.discover.imports import add_import_callback, register_import_watcher
from shenzi.discover.python_props import get_python_props
from shenzi.discover.monkeypatch import kwarg_else_arg, try_monkey_patch
from shenzi.discover.types import *

DUMP_LOC_ENV_VAR = "SHENZI_JSON"
DEFAULT_LOC = "shenzi.json"


LOADS: dict[LocalLoad, LoadParams] = {}


def monkey_patch_dlopen():
    try_monkey_patch(
        "ctypes",
        ["cdll", "LoadLibrary"],
        partial(ctypes_cdll_callback, loads=LOADS),
        kwarg_else_arg("name", 0),
    )
    try_monkey_patch(
        "ctypes",
        ["CDLL", "__init__"],
        partial(ctypes_cdll_callback, loads=LOADS),
        kwarg_else_arg("name", 1),
    )
    try_monkey_patch(
        "cffi",
        ["api", "FFI", "dlopen"],
        partial(cffi_dlopen_callback, loads=LOADS),
        kwarg_else_arg("name", 1),
    )
    try_monkey_patch(
        "cffi",
        ["FFI", "dlopen"],
        partial(cffi_dlopen_callback, loads=LOADS),
        kwarg_else_arg("name", 1),
    )


def _validate_prepared_loads(loads: dict[LocalLoad, LoadParams]):
    for local_load in loads:
        if not os.path.isabs(local_load.path):
            raise Exception(
                f"shenzi critical failure, found a path while searching libraries which is not absolute, path={local_load.path}"
            )


def main_exit_handler(pkgs_to_skip: list[str]):
    import sys
    from pathlib import Path

    site_pkgs = [Path(p) for p in sys.path]
    prefixes_to_skip = [p / skip for p in site_pkgs for skip in pkgs_to_skip]
    prefixes_to_skip = [p for p in prefixes_to_skip if p.exists()]
    prefixes_to_skip = [str(p) for p in prefixes_to_skip]

    exit_handler(prefixes_to_skip)


def exit_handler(prefixes_to_skip: list[str]):
    from copy import deepcopy

    loads: dict[LocalLoad, LoadParams] = deepcopy(LOADS)
    _validate_prepared_loads(loads)

    dump_loc = os.environ.get(DUMP_LOC_ENV_VAR, DEFAULT_LOC)
    payload = shenziDiscovery(
        loads=[
            Load(path=load.path, symlinks=list(param.symlinks), kind=load.kind)
            for load, param in loads.items()
        ],
        libs=[Lib(path=lib) for lib in get_libs()],
        python=get_python_props(),
        skip=Skip(prefixes=prefixes_to_skip, libs=[]),
        env={str(k): str(v) for k, v in os.environ.items()},
    )
    with open(dump_loc, "w") as f:
        json.dump(payload.to_dict(), f)


_ENABLED_DISCOVERY = False


def shenzi_init_discovery(skip: Optional[list[str]] = None):
    if not skip:
        skip = []
    global _ENABLED_DISCOVERY
    if _ENABLED_DISCOVERY:
        return
    _ENABLED_DISCOVERY = True
    monkey_patch_dlopen()
    register_import_watcher(partial(add_import_callback, loads=LOADS))
    atexit.register(lambda: main_exit_handler(skip))
