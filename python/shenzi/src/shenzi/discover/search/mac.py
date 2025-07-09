
import ctypes.util
from typing import Optional
from pathlib import Path


def search_mac(name: str) -> Optional[str]:
    if Path(name).exists():
        return name
    return ctypes.util.find_library(name)
