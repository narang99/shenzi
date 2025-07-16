# TODO
- handle weak lc load commands (which don't fail if the library does not exist) for mac
- parallelization 
  - parallelization while creating graph
    - create separate graphs for chunks and merge them later
    - this would be useful for say, creating python graphs in parallel in a separate graph
    - creating graphs in chunks of 10, and keep updating the known-libs
  - toposort which provides parallel work units like python graphlib's sorter would help with dist creation
- [x] dist folder is huge 
  - its putting .git folder in
  - adding test dependencies
- test multiprocessing environments
  - how does atexit behave in pytest-dist? is it called once?
  - add multiprocessing guards for LOAD variable
- [x] tox creates duplicate virtual environments, might need to handle this somehow
  - we are also pushing .git folder inside the dist, need to fix that too


# Algorithm

- we have sys.path, explicitly imported so libs, imported packages, sys.prefix, sys.exec_prefix
- for all existing directories in sys.path, copy them to dist
- does it make sense to keep `prefix` and `exec_prefix` separate?
    - what if `prefix` and `exec_prefix` are different in the dev machine?
    - in this case, we can simply take `lib-dynload` and dump it in the correct location, its fine

```bash
dist
    run.sh # the main script which sets up the env for python and runs the main application
    symlinks
        cv2.abi3.so 
            libavcodec.so (symlink to ../../reals/r/libavcodec.so)
    reals
        r
            cv2.abi3.so (actual file, everything in symlink file points to this)
            libpango.so
            libintended avcodec.so (all load commands are to ../../symlinks/libavcodec.so)
        e
            # reals working directory
            python
    lib
        l
            libpango.so (../../reals/r/libpango.so)
    bin
        b
            # all executables real paths (other than `python`, that needs special handling)
            convert
    python
        bin
            python
        lib
            python<major>.<minor><abi-thread>
                os.py
                lib-dynload
    site-packages
        p1
            torch
            ...
        p2
            numpy
            ...
```

- getting `abi_thread`
```python
import sys
if hasattr(sys, 'abiflags') and 't' in sys.abiflags:
    abi_thread = 't'
else:
    abi_thread = ''
```

Algorithm:

- Go through all `sys.path` directories
    - If it is lib-dynload or pythons stdlib, copy the whole thing to its correct location
    - for other packages
        - go through all the imported modules
            - find the site-packages it came from
                - if the module is part of stdlib imports, ignore it (has already been copied in the first step)
                - else if part of a site-package: copy to the correct destination in dist
                - else: panic/return error, found a package not in site-packages (will need special handling for the module from where the user runs the script)

- Go through all the dynamic library loads (dlopen or imports)
    - if they are inside site-packages or stdlib, ignore them
- copy the remaining libs in `ld_library/l`
- collate ALL the so files now from our dist folder in `python` and `ld_library_path`
- generate symlink farm
- remove the files from the tree and symlink them here



The final goal is to copy all our artifacts in the dist folder, then create a symlink farm
- Each file that we copy has a source and a destination. The destination can be calculated using a combination of the source, sys.path, sys.prefix and sys.exec_prefix
  - for every shared lib or an executable, we try to 


- find the closure of the python executable, and create a symlink farm for that, all dependencies are now in `reals/r`
    - copy the python executable to dist/python/bin/python
    - patch it to point to values in symlink farm
    - this process is the same for all executables

- When to copy and when to symlink is getting confusing and hard to understand
