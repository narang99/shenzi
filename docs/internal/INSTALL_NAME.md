What is the difference between `LC_ID_DYLIB` inside a shared library and the path name of that shared library in mac? The equivalent thing in linux is `SONAME` and the path name.  
In this doc `LC_ID_DYLIB` and `SONAME` are called install names of those libraries. Path names are simply called path names.  


The whole confusion comes because of the differences between linking behaviors at compile time and at runtime.  



Let's say we have this file structure
```
main.c
libs
  libA.h
  libA.so
```


To compile `main.c` where the final executable would link against `libA.so`, we could do it like this
```bash
clang -o main main.c -L./libs -lA
```
This would make clang discover `libA.so` (from `-lA`), it would also search `./libs` folder.  
The link editor would now look at `libA.so`'s install name. It would copy this install name and add it as an `LC_LOAD_CMD` or a `DT_NEEDED` entry inside the final main executable.  
This is the only place install names matter  

The install name of `libA.so` is determined while building the shared library. If you built it without passing `-install_name` to `clang`, it would use the value in `-o` as the install name by default  

---

The runtime linker however, does not care about the install names at all. It simply searches the values in `LC_LOAD_CMD` or `DT_NEEDED` in its search path. Let's say `main` is shipped to a machine. It would depend on `libA.so` as a dependency (would have it in the load command)  
Now the linker simply searches for `libA.so` in its search path, if you created a symlink to the real  `libA.so` called `libHello.so` in the directory `main` is, the linker won't be able to find `libA.so` using `libHello.so`, it does not scan the files in search path at all.  


For the purposes of our work, the install_name of a shared library does not matter that much, since its not used at all. We patch already built files.  
Basically, the symlink farm should contain symlinks to the real shared library, the name should simply be values in the load commands.  

