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

# Handling paths
Each source file has:
- the final patched file content kept in a temp file
- its own symlink farm
- its destination in reals (or exes)
- its intended symlink destination

## Examples

- `python` executable
    - symlink farm is generated 
    - kept in `reals/e`
    - instead of symlinking to dist/python/bin/python, we move it there


- a random executable (`convert`) outside `sys.path`
    - symlink farm is generated
    - kept in `reals/e`
    - symlinked to `bin`


- a library outside `sys.path`
    - symlink farm is generated
    - kept in `reals/r`
    - symlinked to `lib`


- a library inside `sys.path`
    - symlink farm is generated
    - kept in `reals/r`
    - symlinked to a location in `site-packages`, derived from the source location and original `sys.path`


Generating a symlink farm therefore, should only put the patched stuff in its own `reals` directory.  

- First generate the symlink farm for everything (along with patched values in `reals`)
- copy all the artifacts
    - copy python executable
    - copy non-standard site-packages
    - copy stdlib site-packages
    - go through them and symlink everything


How the symlink farm looks is a function of the original file. where the file is placed is also that.
But how the final symlinks are done is a function of the destination in symlink farm
symlink farm creation is also a reasonably cacheable activity (you can have duplicates, it makes sense to create a farm for everyone so that we can track what is "done"?)  



Need some pseudocode with concrete paths

Start from the [example manifest file](./example_manifest.json)  

- There are many `numpy` invocations in the `modules.pure` section
    - we want to deduplicate `modules.pure`
    - importing a submodule should always import the parent module in our system, we should not be concerned removing submodules eagerly (without checking if parent exists in the list)
    - find the `sys.path` directory which is the longest path prefix of this module
        - remove the prefix, this results in stuff like this
        - `numpy/__init__.py`
        - `hello.py`
        - `numpy/lib/__init__.py`
        - `numpy/lib.py`
        - out of the above, only `numpy/__init__.py` and `hello.py` are useful
    - remove submodules from the list
    - now remove all modules which are part of stdlib directories (lib and lib-dynload)

At this point, the manifest becomes this (everything other than `numpy` is done)
```json
{
    "loads": [
        {"path": "/Users/hariomnarang/miniconda3/lib/libpango.so"},
        {"path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/numpy/linalg/np-will-dlopen.so"}
    ],
    "modules": {
        "extensions": [
            {
                "name": "numpy._core._multiarray_umath",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/numpy/_core/_multiarray_umath.cpython-312-darwin.so"
            },
            {
                "name": "numpy.linalg._umath_linalg",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/numpy/linalg/_umath_linalg.cpython-312-darwin.so"
            },
            {
                "name": "_pickle",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/lib-dynload/_pickle.cpython-312-darwin.so"
            },
            {
                "name": "_datetime",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/lib-dynload/_datetime.cpython-312-darwin.so"
            },
            {
                "name": "_contextvars",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/lib-dynload/_contextvars.cpython-312-darwin.so"
            },
            {
                "name": "some_c_extension",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/some_c_extension.so"
            }
        ],
        "pure": [
            {
                "name": "numpy",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/numpy/__init__.py"
            },
        ]
    },
    "python": {
        "sys": {
            "prefix": "/Users/hariomnarang/miniconda3",
            "exec_prefix": "/Users/hariomnarang/miniconda3",
            "platlibdir": "lib",
            "version": {
                "major": 3,
                "minor": 12,
                "abi_thread": ""
            },
            "path": [
                "/Users/hariomnarang/Desktop/work/blog/linker/shenzi/python/shenzi/src",
                "/Users/hariomnarang/miniconda3/lib/python312.zip",
                "/Users/hariomnarang/miniconda3/lib/python3.12",
                "/Users/hariomnarang/miniconda3/lib/python3.12/lib-dynload",
                "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"
            ],
            "executable": "/Users/hariomnarang/miniconda3/bin/python"
        }
    }
}
```

Now we want to normalize the `extensions` part. We remove everything that is part of standard library site-packages, or is part of a python package which we are going to import (we will need to go through all the pure modules which we allow importing, and see if this extension is present in that, if not, we need to find the pure module this extension is part of and load it too [this is a weird case, for now `panic`])  

If there are pure C extensions which are not in standard library, we want to copy them to our site-packages, we do the same thing with our extensions. 
We remove every extension which is part of the stdlib. We also remove extensions which are part of pure modules which are recursively anyways copied to the dist folder  
Now `loads` can also be part of already imported modules, so we do the same for them (remove all loads which are inside an imported module). This should be rare i think, but anyways we do this.  

new manifest
```json
{
    "loads": [
        {"path": "/Users/hariomnarang/miniconda3/lib/libpango.so"}
    ],
    "modules": {
        "extensions": [
            {
                "name": "some_c_extension",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/some_c_extension.so"
            }
        ],
        "pure": [
            {
                "name": "numpy",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/numpy/__init__.py"
            },
        ]
    },
    "python": {
        "sys": {
            "prefix": "/Users/hariomnarang/miniconda3",
            "exec_prefix": "/Users/hariomnarang/miniconda3",
            "platlibdir": "lib",
            "version": {
                "major": 3,
                "minor": 12,
                "abi_thread": ""
            },
            "path": [
                "/Users/hariomnarang/Desktop/work/blog/linker/shenzi/python/shenzi/src",
                "/Users/hariomnarang/miniconda3/lib/python312.zip",
                "/Users/hariomnarang/miniconda3/lib/python3.12",
                "/Users/hariomnarang/miniconda3/lib/python3.12/lib-dynload",
                "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"
            ],
            "executable": "/Users/hariomnarang/miniconda3/bin/python"
        }
    }
}
```


Now we start copying (note that dyld allows nesting and we are collapsing our whole dependency tree in a single flat folder. This fails if two "different" dependencies have the same name, this is not allowed in linux and macos).  
Understanding when two same-named dylibs will have different content is difficult, ill have to see how to make it work. For now, we just keep replacing it (this should raise a warning, as such, I would rather have the information on which older file the current file is pointing to)   

We start by add `loads` in the symlink farm
```
dist
    reals
        r
            libpango.so
            pangodep.so
    symlinks
        libpango.so
            pangodep.so -> ../../../r/pangodep.so
```

(wait how do i take care of transitive dependencies. nooooooooo. For now we simply put everything in all site-packages other than stdlib and lib-dynload)
Now go through all the modules to import and their corresponding site-packages folder and copy them  
Btw, nesting can be made better, i could put a separate `reals+symlinks` for loads, and a different one for others, this would increase the space consumption but might be useful. As such, we need there to be a logical structure before doing any file operations, so that we can make these optimizations.  


Then we go through all site-packages recursively and create symlink farms for all dylibs.  
Copy all site-packages recursively, identify all dylibs inside and symlink them to reals directory.  
(Its very useful to have an abstract representation before dumping right now i feel)  

```
dist
    reals
        r
            libpango.so
            pangodep.so
            linalg.so
            blas.so
    symlinks
        libpango.so
            pangodep.so -> ../../../reals/r/pangodep.so
        linalg.so
            blas.so -> ../../../reals/r/blas.so
    site-packages
        # using names is not reliable as two different site-packages can have the same basename
        a (came from /Users/hariomnarang/miniconda3/lib/python3.12/site-packages)
            numpy
                ...whole numpy
                linalg.so -> ../../../../reals/r/linalg.so
```

Do the above process for stdlib and dyn-load, but put in the correct python directory
Finally we do it for the python executable, put its deps in symlink farm, we then simply move the executable to python/bin
```
dist
    reals
        r
            libpango.so
            pangodep.so
            linalg.so
            blas.so
            _csv.so
            libpython.so
    symlinks
        libpango.so
            pangodep.so -> ../../../reals/r/pangodep.so
        linalg.so
            blas.so -> ../../../reals/r/blas.so
        _csv.so
        libpython.so
    exes
        e
           python (real copy) (points ../../../reals/r/libpython.so)
    python
        bin
            python (moved from exes/e/python)
        lib
            lib-dynload
                _csv.so -> ../../../../reals/r/_csv.so
            os.py
            ...
    site-packages
        # using names is not reliable as two different site-packages can have the same basename
        a (came from /Users/hariomnarang/miniconda3/lib/python3.12/site-packages)
            numpy
                ...whole numpy
                linalg.so -> ../../../../reals/r/linalg.so
```

There are multiple places where name clashes can occur. These can be fixed by keeping separate farms for separate logical units.  
- a separate farm for all loads
- a separate farm for each site-packages
- a separate farm for python executable
- a separate farm for stdlib



There are few things which are fixed
```
bin
    # put in the path
    ...
python
    bin
        ...
    lib
        ...
site-packages
    uuid1
        ...
    uuid2
        ...
```

Now symlink farms have a certain directory structure which makes it possible to refer to the same farm, regardless of whether the referrer is itself a symlink or a real file  
To achieve this, we simply make sure that everything is at the same LEVEL (which is the max level)  
A problem can come when there is a very rigid structure (like if `lib` could only exist 1 level above the python home [not true]), in that case, we need to create a separate farm, there is no other chance  
In any case, we could have a symlink farm for each individual logical unit of files  

If we start out with separate farms, it might be possible to fuse some farms, and raise the level of each individual farm to the max of the two.  
Fusing is basically identifying if there are same files, if yes then we merge the nodes  
A logical unit should be easily definable, so that I can do all this (without fusing) if i find problems later.  

# Logical representation and data structures
What is the logical representation of a symlink farm? Not on FS I mean

The farm is pretty much a graph

A single farm is defined by a graph of lib names     
```
Lib{
    libpango,
    deps: [Lib{pangodep}]
}
```

We just define the level and the farm would be "raised" to another level with the logical units folder name  
As an example, let's consider a scenario where the python executable and the lib-dynload were fixed

```
pythonunit
    symlinks
        l
            _csv.so (directory containing all direct deps of _csv.so)
                yolo.so [symlink to ../../../reals/r/r/yolo.so]
            yolo.so
                libc.so [symlink to ../../../reals/r/r/libc.so]
        e
            convert
                yolo.so [symlink to ../../../reals/r/r/yolo.so]
            python
                libpython.so [symlink to ../../../reals/r/r/libpython.so]
    reals
        r
            l
                _csv.so [points to ../../../symlinks/l/_csv.so]
                yolo.so [points to ../../../symlinks/l/yolo.so]
                libc.so
                libpython.so [points to ../../../symlinks/l/libpython.so]
            e
                convert
                python
python
    bin
        python [points to ../pythonunit/reals/r/libpython.so]
    lib
        lib-dynload
            _csv.so (symlink to ../../pythonunit/symlinks/_csv.so)
bin
    convert -> ../pythonunit/exes/e/e/convert
```


Okay so given a set of input files, we can create a graph which can logically represent our symlink farm. to build the farm, we simply:
```
build_farm(node) {
    if done {
        return
    }
    build_farm(node.deps)

    for d in node.deps {
        node.point_to_symlink_farm_for_d(d)
    }
    add_to_real (node)
    node.point_loader_path_to_symlink_farm()
    for deps in node.deps {
        node.add_rel_to_loader_path(dep)
        node.add_dep_to_farm(dep)
    }
}
```

The farm is located at the `logicalunit/...`. This makes sure we can create unique isolated farms.  


Now we have a graphical representation of the whole farm for ALL the shared lib dependencies (including everything that needs to be copied from site-packages).  
Its time to create the structure now. recursively copy all the modules in sys.path at appropriate location.   
Go through all the copied artifacts and replace their SOs with symlinks to the ones in reals.  
- this gives us an iterator which provides everything to replace with
Do the same for executables.  

Finally for python, do the same as a normal executable. But instead of symlinking to correct location, move it  
for dlopen libs, copy them to the destination, then replace  


"Replacing" with symlink is essentially an iterator, which takes a struct and calls from_farm. This can either symlink or move.

Good data structures are important here. Stuff after that should be easy for AI to write.  


I might be able to make it work with some iterator pattern. First an iterator which gives us dylibs from source
- this in turn generates a graph, along with destination of the dylib relative to dist in the node (putting the source is also useful in this struct)
- Given a list of iterators, we can create the full graph of dylibs required now

Graph node:
```
Node {
    name,
    src,
    deps: list[Node]

    destination() {
        ...
    }
}
```

Now we get nodes after topological sort and start creating entries  
Given a source, it should also be able to provide its intended destination. For non dylibs, we simply have zero deps and an intended destination

Given a stream of files, or logical entities, we have:
- The set of source dylibs which are required in the farm
- this is required to correctly create the closure (it has to be source paths, before copying, otherwise stuff can break)
- the set of python site-packages to export

The node needs to be hashable, we want some sort of identity for that node.  
Uniqueness is determined by the kind of iterator which gave us the node. Each node needs to be wrapped in a struct which provides its hash implementation?
I can only have the same node as part of the hashmap though. It might be best to give a string as part of the node's identity 

For libpython.so, name is the identity, for pure python files, the path from site-packages and the site-packages folder is the identity  
Iterator can return duplicate elements.  

Examples of nodes:
```
Node {
    id = "dll_libpython.so",
    src {
        path = "/usr/lib/libpython.so,
    },
    deps = [{ Node: libc.so }]
}
```

pure Python node
```
Node {
    id = "site_pkgs_hey_numpy/hello.py",
    src {
        path = "numpy/hello.py",
        site-packages = {
            "path": "/yo/site-packages",
            "id": "hey",
        }
    },
    deps = [],
}
```

Python executable
Node {
    id = "exe_python",
    src = {
        path = "/python",
    }
}


The basic question here is, should i mix the pure python files and the exe files in the same graph as the dylib graph?  
dylib graph might have nodes conflicting with the executable nodes, so it might make sense to keep them separate. or i could prefix those, or use multiple values to hash them.    

Node {
    id = "python",
    kind = PYTHON,
    src = {
        path = "/python"
    }
}


All files actually end up going to reals, or exes. From each of those destinations, we simply call to_final_destination, the invidiual representation is required to know where to put stuff  

Series of steps:
- given a source, add it as a node in graph
    - get deps
        - add them to graph if not there
    - add us in graph
        - add edges to deps

Now we can topologically sort and move the files to reals if needed. So there is:
- `move_to_reals` which can be a noop for pure python files
- `make_symlink_farm` would generate the farm if needed
- `to_destination` (for dylibs, this is symlinking, for pure python files its copying)

Now do i need to `Box` some value which can do this? Its useful to box it because i can pass around arbitrary context in that boxed variable.


top level graph data structure
```
class Node {
    id,
    kind,
    src {
        path,
    },
    inner: Box<dyn ...>
}
```

```
inner {
    get_deps(src.path) -> Vec<Node>;

    // the root directory of the logical unit
    mk_symlink_farm(unit_base);

    // copy to reals directory
    to_reals(unit_base);

    to_destination(dist_base);
}
```


we topo sort and do this basically.  




