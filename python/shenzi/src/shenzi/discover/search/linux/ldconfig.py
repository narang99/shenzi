import subprocess
import os
from typing import Optional
from pathlib import Path


def ldconfig_find(name: str) -> Optional[Path]:
    try:
        output = subprocess.check_output([
            '/sbin/ldconfig', '-p'
        ], env={**os.environ, 'LANG': 'C', 'LC_ALL': 'C'})
        output = output.decode('utf-8')
    except Exception:
        return None
    candidates = _find_in_output(name, output)
    if candidates:
        return candidates
    return None

def _find_in_output(name: str, output: str) -> Optional[Path]:
    candidates: list[str] = []
    for line in output.splitlines():
        candidate = _get_candidate(name, line)
        if candidate:
            candidates.append(candidate)
    # Prefer exact match
    for candidate in candidates:
        if candidate.endswith(name):
            p = Path(candidate)
            if p.exists():
                return p
    # Fallback: first existing candidate
    for candidate in candidates:
        p = Path(candidate)
        if p.exists():
            return p
    return None

def _get_candidate(name: str, line: str) -> Optional[str]:
    comps = line.split('=>')
    if len(comps) < 2:
        return None
    candidate = comps[1].strip()
    if name in candidate:
        return candidate
    return None

