def get_dyld_finder():
    import ctypes
    import ctypes.util

    _libdyld = ctypes.CDLL(ctypes.util.find_library("libdyld.dylib"))

    _libdyld._dyld_image_count.restype = ctypes.c_uint
    _libdyld._dyld_get_image_name.restype = ctypes.c_char_p

    def wrapped():
        count = _libdyld._dyld_image_count()
        res = []
        for i in range(count):
            image_name = _libdyld._dyld_get_image_name(i)
            res.append(image_name.decode())
        return res

    return wrapped


def get_search_terms_for_dylib(name: str) -> list[str]:
    return _get_search_terms_using_by_ctypes(name) + _adhoc_search_term_heuristics(name)


def _adhoc_search_term_heuristics(name: str) -> list[str]:
    res = _get_search_terms_with_suffixes(name)
    if not name.startswith("lib"):
        res.extend(_get_search_terms_with_suffixes(f"lib{name}"))
    return res


def _get_search_terms_with_suffixes(name: str) -> list[str]:
    # TODO: find all the dyld conventions for suffixes (which files are same kinda suffixes)
    res = [
        f"{name}.dylib" if not name.endswith(".dylib") else None,
        f"{name}.0.dylib" if not name.endswith(".0.dylib") else None,
        f"{name}-0.dylib" if not name.endswith("-0.dylib") else None,
        f"{name}.so" if not name.endswith(".so") else None,
        f"{name}.so.0" if not name.endswith(".so.0") else None,
    ]
    return [term for term in res if term is not None]


def _get_search_terms_using_by_ctypes(name: str) -> list[str]:
    import os

    return [os.path.basename(p) for p in get_ctypes_search_paths(name)]


def get_ctypes_search_paths(name: str) -> list[str]:
    # copied from ctypes.macholib.dyld
    from itertools import chain
    import os
    import sys
    from ctypes.macholib.dyld import (
        dyld_image_suffix_search,
        dyld_override_search,
        dyld_executable_path_search,
        dyld_default_search,
    )

    possible = [
        "@executable_path/../lib/lib%s.dylib" % name,
        "lib%s.dylib" % name,
        "%s.dylib" % name,
        "%s.framework/%s" % (name, name),
    ]

    res = []
    for name in possible:
        iterator = dyld_image_suffix_search(
            chain(
                dyld_override_search(name, os.environ),
                dyld_executable_path_search(name, sys.executable),
                dyld_default_search(name, os.environ),
            ),
            os.environ,
        )
        res.extend(iterator)

    return res