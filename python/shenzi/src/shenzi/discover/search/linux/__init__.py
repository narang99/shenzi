from ctypes.util import find_library
from pathlib import Path
import sys
from typing import Optional
import os

from shenzi.discover.search.linux.ldconfig import ldconfig_find
from shenzi.discover.search.linux.rpath import get_rpaths, parse_rpath


SYS_RPATHS = {}


def search_linux(name: str) -> Optional[Path]:
    ld_library_path, ld_preload = _get_library_paths()
    cwd = Path.cwd()
    rpaths, runpaths = _get_sys_rpaths()
    ctypes_find = find_library(name)
    name = ctypes_find if ctypes_find else name

    return _search(
        name, rpaths, runpaths, ld_preload, ld_library_path, cwd
    )

def _get_sys_rpaths():
    exe = Path(sys.executable)
    rpaths, runpaths = get_rpaths(exe)

    rpaths = [parse_rpath(r, exe) for r in rpaths]
    rpaths = [r for r in rpaths if r]

    runpaths = [parse_rpath(r, exe) for r in runpaths]
    runpaths = [r for r in runpaths if r]

    return rpaths, runpaths

def _get_library_paths():
    ld_library_path = os.environ.get("LD_LIBRARY_PATH", "")
    ld_preload = os.environ.get("LD_PRELOAD", "")

    ld_library_path_dirs = [
        Path(p) for p in ld_library_path.split(":") if p and Path(p).exists()
    ]
    ld_preload_libs = [
        Path(p) for p in ld_preload.split(":") if p and Path(p).exists()
    ]

    return ld_library_path_dirs, ld_preload_libs

def _search(
    name: str,
    dt_rpaths: list[Path],
    dt_runpaths: list[Path],
    ld_preload: list[Path],
    ld_library_path: list[Path],
    cwd: Path,
) -> Optional[Path]:
    # 1. search as a path
    path = _search_name_as_path(name, cwd)
    if path:
        return path

    # 2. search LD_PRELOAD
    found = _find_in_dirs(name, ld_preload)
    if found:
        return found

    # 3. search DT_RPATH and extra_rpaths if DT_RUNPATH is empty
    if not dt_runpaths:
        found = _find_in_dirs(name, dt_rpaths)
        if found:
            return found

    # 4. search LD_LIBRARY_PATH
    found = _find_in_dirs(name, ld_library_path)
    if found:
        return found

    # 5. search DT_RUNPATH
    found = _find_in_dirs(name, dt_runpaths)
    if found:
        return found

    # 6. fallback, ask ldconfig
    found = ldconfig_find(name)
    if found:
        return found

    # 7. fallback to standard library dirs
    std_lib_dirs = [
        Path("/lib64"),
        Path("/lib"),
        Path("/usr/lib64"),
        Path("/usr/lib"),
    ]
    found = _find_in_dirs(name, std_lib_dirs)
    if found:
        return found

    return None

def _find_in_dirs(file_name: str, dirs: list[Path]) -> Optional[Path]:
    for dir in dirs:
        candidate = dir / file_name
        if candidate.exists():
            return candidate
    return None


def _search_name_as_path(name: str, cwd: Path) -> Optional[Path]:
    if "/" not in name:
        return None
    p = Path(name)
    if p.is_absolute():
        return p if p.exists() else None
    else:
        candidate = cwd / p
        return candidate if candidate.exists() else None