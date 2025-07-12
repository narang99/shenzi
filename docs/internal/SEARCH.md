Every platform has different search semantics on how to "find" libraries. We need to replicate it as faithfully as we can.  

# Why do we need searching?
The first approach was to use a very simple semantic. Get the list of all loaded libraries in the python process using `dyld_image_count`. For every library, use the leaf name of the library to be searched, search the libraries in `dyld_image_count`. If it exists, we good, use that. Else error out.  
The problem however, is that `dyld_image_count` only returns the real paths of the libraries which are loaded.  
A `dlopen` call can be done on a search term, which might be a symlink to the real file. As an example `pango-1.0` points to some lib `<absolute-path-to>/pango-1.0.dylib`. This path is actually a symlink to `<absolute-path-to>/pango-2.0.dylib`, we only get this library in the `dyld_image_count` list.  
The symlinks are arbitrary (as above). Its impossible to "guess" some heuristics to find symlinks.  
We would need `pango-1.0.dylib` in the library path of the final packaged application.  
If we just use real paths from `dyld_image_count`, its not going to work.  


So we do the search ourselves too. The search needs to be exactly the same as dyld to make this work.   
Both the python lib and the rust binary need to implement the search.  
- individual python FFI libraries might augment the search in arbitrary ways, we need to faithfully recreate that too in the python library code
- rust code basically includes all dylibs in a greedy fashion, in this case it also needs to resolve the search path for LC_LOAD commands

# Mac (dyld)
any library which provides dlopen utilities, has some semantics for searching paths before calling the syscall dlopen
dlopen itself has some semantics, described in its man page. 

dlopen uses this search order:
- if path does not contain any slash
  - DYLD_LIBRARY_PATH
  - RPATH of executable (sys.executable in our case). or the library (if its as a load command inside dylib)
  - current directory (this is simply Path.cwd())
  - DYLD_FALLBACK_LIBRARY_PATH
  - /usr/local/lib
  - /usr/lib
- if path looks like a framework
  - DYLD_FRAMEWORK_PATH
  - use path as is, if its relative, append to cwd and make absolute
  - DYLD_FALLBACK_FRAMEWORK_PATH
  - /Library/Frameworks
  - /System/Library/Frameworks
- if path has a slash but is not a framework path
  - DYLD_LIBRARY_PATH (use the leaf of the path)
  - use path as is, if its relative, append to cwd to make absolute
  - DYLD_FALLBACK_LIBRARY_PATH
  - /usr/local/lib
  - /usr/lib

we need to do this searching ourselves. The search might be actually augmented by individual FFI libraries (with their own extra semantics)
Our python code needs to do this searching individually for each FFI library (encode their semantics, fake it basically)
depending on library version, we might need to do more. The easiest way is to call private functions if they exist, and then use our fallback in the worst case

The rust code also needs to implement these search semantics, for parsing dylibs. 
When we are parsing dylibs, we might need to implement the search above, the current search is not enough

`ctypes` has this exact algorithm encoded in it, we use it as our fallback (`find_library`).  

# Linux (ld.so)

- LD_LIBRARY_PATH
- DT_RPATH / DT_RUNPATH
  - $ORIGIN (directory where the loading object is)
  - . : current directory
  - empty: current directory
- /etc/ld.conf: this is a whole parsing and locating business, best rely on external tool like `ldconfig` or something for this (python implementation does this, look it up)
  - this ill copy from ctypes find_library


The search order is provided in `man 8 ld.so`

```
When  resolving shared object dependencies, the dynamic linker first inspects each dependency string to see if it contains a slash (this can occur if a shared object pathname containing slashes was specified at link time).  If a slash is found, then the dependency string is interpreted as a (relative or absolute) pathname, and the shared object is  loaded
  using that pathname.

  If a shared object dependency does not contain a slash, then it is searched for in the following order:

  o  Using the directories specified in the DT_RPATH dynamic section attribute of the binary if present and DT_RUNPATH attribute does not exist.  Use of DT_RPATH is deprecated.

  o  Using the environment variable LD_LIBRARY_PATH, unless the executable is being run in secure-execution mode (see below), in which case this variable is ignored.

  o  Using  the  directories  specified  in  the DT_RUNPATH dynamic section attribute of the binary if present.  Such directories are searched only to find those objects required by
    DT_NEEDED (direct dependencies) entries and do not apply to those objects' children, which must themselves have their own DT_RUNPATH entries.  This is unlike DT_RPATH, which is
    applied to searches for all children in the dependency tree.

  o  From  the  cache  file /etc/ld.so.cache, which contains a compiled list of candidate shared objects previously found in the augmented library path.  If, however, the binary was
    linked with the -z nodeflib linker option, shared objects in the default paths are skipped.  Shared objects installed in hardware capability directories (see  below)  are  pre‐
    ferred to other shared objects.

  o  In  the  default  path /lib, and then /usr/lib.  (On some 64-bit architectures, the default paths for 64-bit shared objects are /lib64, and then /usr/lib64.)  If the binary was
    linked with the -z nodeflib linker option, this step is skipped.



Rpath token expansion
The dynamic linker understands certain token strings in an rpath specification (DT_RPATH or DT_RUNPATH).  Those strings are substituted as follows:

$ORIGIN (or equivalently ${ORIGIN})
      This expands to the directory containing the program or shared object.  Thus, an application located in somedir/app could be compiled with

          gcc -Wl,-rpath,'$ORIGIN/../lib'

      so that it finds an associated shared object in somedir/lib no matter where somedir is located in the directory hierarchy.  This facilitates the creation of "turn-key"  ap‐
      plications that do not need to be installed into special directories, but can instead be unpacked into any directory and still find their own shared objects.

$LIB (or equivalently ${LIB})
      This expands to lib or lib64 depending on the architecture (e.g., on x86-64, it expands to lib64 and on x86-32, it expands to lib).

$PLATFORM (or equivalently ${PLATFORM})
      This expands to a string corresponding to the processor type of the host system (e.g., "x86_64").  On some architectures, the Linux kernel doesn't provide a platform string
      to the dynamic linker.  The value of this string is taken from the AT_PLATFORM value in the auxiliary vector (see getauxval(3)).

```

Before others, `LD_PRELOAD` is also done to preload libraries. `/etc/ld.so.preload` contains the list of libraries to preload globally

## Details of the python implementation

The reference implementation in ctypes:
- `_findLibgcc`: calls `gcc` with linker trace enabled, it prints all the paths its loading i think, the command is this: `gcc -Wl,-t -o $HOME/hey.txt -l libpango-1.0.so`, it gives an output as below, the python implementation is doing a slightly complicated regex search, but its basically trying to find the line containing the name of the file, it then checks all the matched files, and returns the first one that is an ELF file  
```
/usr/lib/gcc/x86_64-linux-gnu/9/../../../x86_64-linux-gnu/Scrt1.o
/usr/lib/gcc/x86_64-linux-gnu/9/../../../x86_64-linux-gnu/crti.o
/usr/lib/gcc/x86_64-linux-gnu/9/crtbeginS.o
/usr/lib/gcc/x86_64-linux-gnu/9/libgcc.a
/usr/lib/gcc/x86_64-linux-gnu/9/libgcc_s.so
/usr/lib/gcc/x86_64-linux-gnu/9/../../../x86_64-linux-gnu/libgcc_s.so.1
/usr/lib/gcc/x86_64-linux-gnu/9/libgcc.a
/usr/bin/ld: cannot find -llibpango-1.0.so
collect2: error: ld returned 1 exit status
(base) hariom.narang@qure17:~$ cat $HOME/hey.txt
cat: /home/users/hariom.narang/hey.txt: No such file or directory
```
- `find_Soname_ldconfig` is calling `ldconfig` to find files
```bash
# this will print the ld cache
/sbin/ldconfig -p

# gives an output like this
1404 libs found in cache `/etc/ld.so.cache'
	libzvbi.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libzvbi.so.0
	libzvbi-chains.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libzvbi-chains.so.0
	libzstd.so.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libzstd.so.1
	libzstd.so.1 (libc6) => /lib/i386-linux-gnu/libzstd.so.1
	libzmq.so.5 (libc6,x86-64) => /lib/x86_64-linux-gnu/libzmq.so.5
	libz.so.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libz.so.1
	libz.so.1 (libc6) => /lib/i386-linux-gnu/libz.so.1
	libz.so (libc6,x86-64) => /lib/x86_64-linux-gnu/libz.so
	libyelp.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libyelp.so.0
```
- i dont know how its going to differentiate between two different versions
  - like `ldconfig -p libminisat`
as an example, this script gives two outputs, it seems to be going in ascending order too, two different versions
```bash
$ ldconfig -p | grep libminisat.so
	libminisat.so.2 (libc6,x86-64) => /lib/libminisat.so.2
	libminisat.so (libc6,x86-64) => /lib/libminisat.so
```

python is again doing a regex search in this  

- `findLib_ld`, invokes `ld` for finding
  - splits LD_LIBRARY_PATH, passes each path as `-L <path>` to `ld`
  - runs a command like this (empty LD_LIBRARY_PATH) `ld -t -o /dev/null -llibminisat.so.2`
  - its not giving anything to me

- `findLib_prefix` is simply looking at `sys.prefix/lib` folder

Then there is a snippet for the main finder
```python
def find_library(name):
    # See issue #9998
    # Yes calling _findLib_prefix twice is deliberate, because _get_soname ditches
    # the full path.
    # When objdump is unavailable this returns None
    so_name = _get_soname(_findLib_prefix(name)) or name
    if so_name != name:
        return _findLib_prefix(so_name) or \
                _findLib_prefix(name) or \
                _findSoname_ldconfig(name) or \
                _get_soname(_findLib_gcc(name)) or _get_soname(_findLib_ld(name))
    else:
          return _findLib_prefix(name) or \
                _findSoname_ldconfig(name) or \
                _get_soname(_findLib_gcc(name)) or _get_soname(_findLib_ld(name))
```

They are also doing weird stuff, where they are returning `soname` for some libs. Maybe this is simply resolving symlinks and letting `dlopen` handle it?  
This is again very confusing now  

## Our implementation
There are two parts of finding here. One is in our python library. every patched dlopen needs to find the returned library somehow, some might expose handles, others might not   
If handle is exposed, it might make sense to cache these handles and see compare in the end with the whole list of loaded libraries. If the handle is not exposed (like in cffi), theres not much we can do  
the general strategy can be
- call find_library
  - for now only relying on this
- `LD_LIBRARY_PATH`, `LD_PRELOAD`
- `/lib`, `/usr/lib`, `/lib64`, `/usr/lib64`
- if it fails, call `ldconfig -p` and see if we can find our library in that (find_library already does this though)
- library's adhoc search terms to be also kept, as usual
  - `lib<>`, `lib<>.so` and `<>` should be good enough for now

Implementation rust:
- we don't care about arbitrary dlopen calls at this point. we basically follow the `man 8 ld.so` faithfully


## RPath problem in linux
All the objects in my python installation are using DT_RPATH, not DT_RUNPATH  
We have a problem here, due to the whole nesting of DT_RPATH


- torch/C_.python3.9.so has DT_RPATH=$ORIGIN/lib
  - it is the only one linking to lib/libc10.so
  - the problem now is that `lib/libc10.so` has many libs inside the `lib` folder as `DT_NEEDED`
    - its impossible to gather the nodes without knowing the order in which these libraries are loaded
- linux linker however, does not have nested namespaces
  - in this case, name clashes are fine (we would find multiple libraries in multiple places)
- thats okay though, as I said, name clashes are fine in linux
  - so now we basically collect ALL the rpaths in the first pass
  - there is one issue though, we return errors on libraries which we are not able to load (without the rpaths)
- the other thing that we can do is track c extension imports now
  - basically want all entrypoints that are opened in the python process
  - if we theoretically have all of them, we can also parse their deps in a tree like fashion (exactly how the linker works)
  - when scanning the FS for remaining deps, we would skip the files which we have scanned

## on shared library names

Best Reference:
- https://flameeyes.blog/2010/10/08/linkers-and-names/

There is some confusion when it comes to the path name of the library, and its SONAME.  
The confusion exists if you only think about the behavior in runtime.  

When compiling an application against a library, the linker (in the compiler, called the link editor) would use `SONAME` as the value for `DT_NEEDED`  
`ld.so` however, at execution time, only cares about `DT_NEEDED`, the `SONAME` is not even considered  
During installation `ldconfig` scans a library, finds its `SONAME` and creates a symlink with this name in the linker's search path.  
This is very indirect, but it is what it is :)


Finally on searching:
- given a name to `dlopen`, we need to do the search that the linker does normally. Nothing special about it. Don't care about `SONAME`  
- same for searching after scanning a shared library


- There is one indirection I'm using in python, im using `find_library` to get the SONAME and search it. This is fine, as it is the closest I can get to how libraries are searching for dependencies inside python


# Creating nodes

There are three ways we create nodes
- from manifest
- from the python universe (all files that we can follow)
- inside graph


Manifest:
- we can't do this in parallel, the order is very important. By definition this is serial. Can only happen inside graph
- the python universe can be done in parallel. 

The fact that I want to do parallel execution for the universe is giving a lot of pain
Once the universe is added to the graph, we anyways have to do serial execution for all the nodes that we added for the universe
At this point though, due to linux RPATH being transitive, it is kinda impossible to do parallel execution (a node needs to be added by default)
Give up on parallel for now


What kind of operations do we do on the node?
- finding its dependencies
- putting it in the dist folder

For now, I've kept different enums for this, this seems problematic. One enum that is just the kind of the node should be enough. I'm going back to old code, no problem