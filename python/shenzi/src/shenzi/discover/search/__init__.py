from typing import Optional
from pathlib import Path
from shenzi.discover.plat import get_platform


def search_lib(name: str) -> Optional[Path]:
    from shenzi.discover.search.linux import search_linux
    from shenzi.discover.search.mac import search_mac

    plat = get_platform()
    if plat == "linux":
        return search_linux(name)
    elif plat == "mac":
        result = search_mac(name)
        return Path(result) if result else None
    else:
        # unknown
        raise Exception(f"platform {plat} is unknown")


def cffi_find_library(name: str) -> Optional[Path]:
    return search_lib(name)


def ctypes_cdll_find_library(name: str) -> Optional[Path]:
    return search_lib(name)
