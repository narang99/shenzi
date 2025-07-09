from pathlib import Path
import sys
import importlib.abc
from shenzi.discover.types import LoadParams, LocalLoad

def register_import_watcher(add_ext_callback):
    class ImportWatcher(importlib.abc.MetaPathFinder):
        def find_spec(self, fullname, path, target=None):
            # We delegate finding this import to other finders
            # if any finder returns a spec, pass a `shenzi.types.Module` to callback
            spec = None
            for finder in sys.meta_path:
                if finder is not self:
                    try:
                        spec = finder.find_spec(fullname, path, target)
                        if spec is not None:
                            break
                    except (ImportError, AttributeError):
                        continue

            if spec is not None and spec.origin:
                if self._is_dyn_lib(spec.origin):
                    add_ext_callback(spec.origin)
            return None

        def _is_dyn_lib(self, path):
            return path.endswith(".dylib") or path.endswith(".so")

    sys.meta_path.insert(0, ImportWatcher())


def add_import_callback(path: str, loads: dict[LocalLoad, LoadParams]) -> None:
    p = Path(path)
    if not p.exists():
        raise Exception(f"import watcher found a path which does not exist: {path}")
    p = p.resolve()
    local_load = LocalLoad(kind="extension", path=str(p))
    params = LoadParams(symlinks=set([]))
    loads[local_load] = params