# TODO
- make qureapp work
- better error when install_name_tool fails (this fails when the library is itself statically linked), need to see if this failure should even happen?
  - currently, im ignoring a lib if it does not contain any load commands (outside of system library load commands)
  - in this case, mostly we cover all statically linked cases
- path handling is slightly confusing between strings and pathbufs in some places, fix that too
- /Users/hariomnarang/Desktop/work/blog/linker/shenzi/crates/shenzi_rs/dist/reals/r/_weight_vector.cpython-39-darwin.so
  - no space to change load commands in this file
  - thankfully this is not extremely common out there
  - the only option is to replicate the load commands structure inside the dist folder relative to what the file wants
- im now getting ALL the loaded libraries in dyld_image_count
  - now the problem is symlinks, if dyld found something using symlink, its going to add only the real path
  - for each search which succeeded in dlopen, we need to add that search term to our symlink marker, thats the easiest way to do this
    - the problem is me not getting the real path from the stupid dyld search, i need to use heuristics to make it work
- handle weak lc load commands (which don't fail if the library does not exist) for mac
- parallelize moving to dist, basically parallelize toposort (its a linear implementation right now, petgraph does not give a work distributing implementation right now)


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
