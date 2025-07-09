import os
from pathlib import Path
from typing import Callable, Optional
from shenzi.discover.search import cffi_find_library, ctypes_cdll_find_library
from shenzi.discover.types import LoadParams, LocalLoad


def cffi_dlopen_callback(name: str, loads: dict[LocalLoad, LoadParams]) -> None:
    _std_load_callback(name, loads, _cffi_dlopen, "cffi.FFI.dlopen", True)


def ctypes_cdll_callback(name: str, loads: dict[LocalLoad, LoadParams]) -> None:
    _std_load_callback(name, loads, _ctypes_cdll_dlopen, "ctypes.CDLL", True)


def _std_load_callback(
    name: str,
    loads: dict[LocalLoad, LoadParams],
    params_getter: Callable[
        [str, dict[LocalLoad, LoadParams]], Optional[tuple[LocalLoad, LoadParams]]
    ],
    patched_id: str,
    strict: bool,
) -> None:
    maybe_result = params_getter(name, loads)
    if not maybe_result:
        if not strict:
            return
        else:
            raise Exception(
                f"shenzi: failed in finding dynamic library, name={name}, dlopen_called_from={patched_id}"
            )
    local_load, params = maybe_result
    loads[local_load] = params


def _cffi_dlopen(
    name: str, loads: dict[LocalLoad, LoadParams]
) -> Optional[tuple[LocalLoad, LoadParams]]:
    return _std_dlopen(name, loads, cffi_find_library)


def _ctypes_cdll_dlopen(
    name: str, loads: dict[LocalLoad, LoadParams]
) -> Optional[tuple[LocalLoad, LoadParams]]:
    return _std_dlopen(name, loads, ctypes_cdll_find_library)


def _std_dlopen(
    name: str,
    loads: dict[LocalLoad, LoadParams],
    fd_lib: Callable[[str], Optional[Path]],
) -> Optional[tuple[LocalLoad, LoadParams]]:
    lib = fd_lib(name)
    if not lib:
        print("could not find lib for", name)
        return None
    if not lib.exists():
        raise Exception(
            f"got a path from find_library which does not exist, search_term={name} path={lib} finder={fd_lib.__qualname__}"
        )

    lib = os.path.realpath(lib)
    symlinks_to_add = get_symlinks(name, lib)

    local_load = LocalLoad(path=lib, kind="dlopen")
    if local_load in loads:
        loaded_params = loads[local_load]
        symlinks_to_add = symlinks_to_add | loaded_params.symlinks

    return local_load, LoadParams(symlinks=symlinks_to_add)


def get_symlinks(name: str, full_path: str) -> set[str]:
    _name = Path(name)
    _full_path = Path(full_path)
    if _name.name == _full_path.name:
        return set()
    else:
        return set([_name.name])
