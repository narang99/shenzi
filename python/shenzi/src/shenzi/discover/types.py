
from dataclasses import dataclass
from typing import Any, Literal

LoadKind = Literal["extension", "dlopen"]

@dataclass(frozen=True)
class LocalLoad:
    kind: LoadKind
    path: str


@dataclass
class LoadParams:
    symlinks: set[str]


@dataclass(frozen=True)
class Load:
    path: str
    kind: LoadKind
    symlinks: list[str]

    def to_dict(self) -> dict[str, Any]:
        return {"path": self.path, "symlinks": self.symlinks, "kind": self.kind}

@dataclass(frozen=True)
class Lib:
    path: str
    
    def to_dict(self) -> dict[str, Any]:
        return {"path": self.path}


@dataclass(frozen=True)
class Version:
    major: int
    minor: int
    abi_thread: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "major": self.major,
            "minor": self.minor,
            "abi_thread": self.abi_thread
        }


@dataclass(frozen=True)
class Sys:
    prefix: str
    exec_prefix: str
    platlibdir: str
    version: Version
    path: list[str]
    executable: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "prefix": self.prefix,
            "exec_prefix": self.exec_prefix,
            "platlibdir": self.platlibdir,
            "version": self.version.to_dict(),
            "path": self.path,
            "executable": self.executable
        }


@dataclass(frozen=True)
class Python:
    sys: Sys
    
    def to_dict(self) -> dict[str, Any]:
        return {"sys": self.sys.to_dict()}


@dataclass(frozen=True)
class Skip:
    prefixes: list[str]
    libs: list[str]
    
    def to_dict(self) -> dict[str, Any]:
        return {"prefixes": self.prefixes, "libs": self.libs}



@dataclass(frozen=True)
class shenziDiscovery:
    loads: list[Load]
    libs: list[Lib]
    python: Python
    skip: Skip
    env: dict[str, str]
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "loads": [load.to_dict() for load in self.loads],
            "libs": [lib.to_dict() for lib in self.libs],
            "python": self.python.to_dict(),
            "skip": self.skip.to_dict(),
            "env": self.env,
        }