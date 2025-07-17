import sys
import os
from shenzi.discover.types import Python, Sys, Version
from pathlib import Path


def get_python_props() -> Python:
    realpath = os.path.realpath
    
    abi_thread = "t" if hasattr(sys, "abiflags") and "t" in sys.abiflags else ""
    python_path = [realpath(p) for p in sys.path if Path(p).exists()]
    return Python(
        sys=Sys(
            prefix=realpath(sys.base_prefix),
            exec_prefix=realpath(sys.base_exec_prefix),
            platlibdir=sys.platlibdir,
            version=Version(
                major=sys.version_info.major,
                minor=sys.version_info.minor,
                abi_thread=abi_thread,
            ),
            path=python_path,
            executable=realpath(sys.executable),
        ),
        main=str(Path(sys.argv[0]).resolve()),
        allowed_packages=None,
        cwd=str(Path.cwd()),
    )
