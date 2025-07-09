import sys
from typing import Literal


Platform = Literal["linux", "darwin", "unknown"]

def get_platform():
    if sys.platform.startswith("linux"):
        return "linux"
    elif sys.platform == "darwin":
        return "mac"
    else:
        return "unknown"