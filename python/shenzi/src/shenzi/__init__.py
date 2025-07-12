

def main() -> None:
    import importlib.resources as resources
    import sys
    import os
    binary_name = "shenzi" if sys.platform != "win32" else "shenzi.exe"
    with resources.path("shenzi.bin", "shenzi") as shenzi:
        if sys.platform == "win32":
            import subprocess
            completed_process = subprocess.run([shenzi, *sys.argv[1:]])
            sys.exit(completed_process.returncode)
        else:
            os.execvp(shenzi, [shenzi, *sys.argv[1:]])
