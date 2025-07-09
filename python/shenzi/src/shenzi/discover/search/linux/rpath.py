import platform
from pathlib import Path
import shutil
import subprocess
from typing import Optional


def get_rpaths(path: Path) -> tuple[list[str], list[str]]:
    # uses objdump
    # TODO: use the shenzi cli that we ship with the library only
    # guaranteed success
    objdump = shutil.which("objdump")
    if not objdump:
        return ([], [])

    try:
        output = subprocess.check_output(["objdump", "-p", str(path)])
        output = output.decode("utf-8")
    except Exception:
        return ([], [])

    # output looks like this
    # Dynamic Section:
    # NEEDED               libpthread.so.0
    # RPATH                $ORIGIN/../lib

    rpath, runpath = "", ""
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("RPATH"):
            parts = line.split(None)
            if len(parts) > 1:
                rpath = parts[1]
        elif line.startswith("RUNPATH"):
            parts = line.split(None, 1)
            if len(parts) > 1:
                runpath = parts[2]

    return (rpath.split(":") if rpath else [], runpath.split(":") if runpath else [])


def parse_rpath(rpath: str, object_path: Path) -> Optional[Path]:
    parent_path_str = str(object_path.parent)
    # TODO: use getauxval instead of this
    is_64_bit = platform.architecture()[0] == "64bit"
    lib = "lib64" if is_64_bit else "lib"
    at_platform = platform.machine()

    rpath_expanded = (
        rpath.replace(r"$ORIGIN", parent_path_str)
        .replace(r"${ORIGIN}", parent_path_str)
        .replace(r"$LIB", lib)
        .replace(r"${LIB}", lib)
        .replace(r"$PLATFORM", at_platform)
        .replace(r"${PLATFORM}", at_platform)
    )

    path = Path(rpath_expanded)
    if path.exists():
        return path.resolve()
    return None
