import sys
from shenzi.discover.types import Python, Sys, Version


def get_python_props() -> Python:
    abi_thread = "t" if hasattr(sys, "abiflags") and "t" in sys.abiflags else ""

    return Python(
        sys=Sys(
            prefix=sys.prefix,
            exec_prefix=sys.exec_prefix,
            platlibdir=sys.platlibdir,
            version=Version(
                major=sys.version_info.major,
                minor=sys.version_info.minor,
                abi_thread=abi_thread,
            ),
            path=sys.path,
            executable=sys.executable,
        )
    )
