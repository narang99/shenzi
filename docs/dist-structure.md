# Structure of the final distributed application

The `dist` folder would look like this (it uses symlinks extensively for resolving library dependencies, its kinda similar to the structure `pnpm` uses as far as I'm aware)
```bash
python
    bin
        python # the interpreter
    lib # the standard library is here
reals
    r
        # example files, the names would be hashes in mac
        libA.so # depends on libc.so
        libB.so # depends on libA.so
        libc.so # depends on libc.so
symlinks
    libA.so
        libc.so -> ../../reals/r/libc.so
    libB.so
        libc.so -> ../../reals/r/libc.so
        libA.so -> ../../reals/r/libA.so
lib
    l
        libA.so -> ../../reals/r/libA.so # all libraries opened using `dlopen` are kept here
bootstrap.sh  # the starter script
```

## Where are my python files?
`dist/site-packages` contain copies of all the directories that were in the python path when you intercepted your application. It would contain all your external packages, and your application code too.  


## Shared libraries, dependencies and symlink farm
The directory `reals/r` contains all the shared libraries that `shenzi` found. In mac, the libraries would be renamed to the hash of their respective contents.  
Every library inside `reals/r` is configured to find its dependencies in `symlinks/<library_name>`, this directory contains symlinks to all the dependencies of that library, the real paths of all dependencies is again in `reals/r`, each dependency would have its own dependency location in `symlinks/<their-library-name>`, I call this the symlink farm in the codebase.  

This is kind of similar to how `pnpm` structures node modules.  


## bootstrap.sh
This is the main script, its pretty simple, it sets the `PYTHONPATH` and the linker's search path (`LD_LIBRARY_PATH` in linux and `DYLD_LIBRARY_PATH` in mac) and calls the python interpreter at `python/bin/python`.  

## Other folders

`lib/l` again contains symlinks to libraries in `reals/r`. The only difference is that `lib/l` is kept in the linkers search path (`LD_LIBRARY_PATH` in linux and `DYLD_LIBRARY_PATH` in mac). This is for all libraries that are loaded using `dlopen` and equivalents.  

The top-level `python` folder contains your packaged python interpreter at `python/bin/python` and the standard library at `python/lib`.  