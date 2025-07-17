from shenzi.discover.plat import get_platform


def get_libs() -> list[str]:
    plat = get_platform()
    if plat == "mac":
        return _get_mac_libs()
    elif plat == "linux":
        return _get_linux_libs()
    else:
        return []


def _get_mac_libs() -> list[str]:
    from shenzi.discover.all_libs.macho import get_dyld_finder

    dyld_finder = get_dyld_finder()
    all_dylibs = dyld_finder()
    return all_dylibs


def _get_linux_libs():
    libs = set()
    with open("/proc/self/maps", "r") as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) >= 6 and parts[-1].endswith(".so"):
                libs.add(parts[-1])
    return sorted(libs)