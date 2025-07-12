We have this given structure
```
main -> ./libA.so ./libB.so
libA.so -> ./a/libC.so
libB.so -> ./b/libC.so
```

`./a/libC.so` and `./b/libC.so` can be thought of as incompatible versions of the same library. More specifically
```c
// ./a/libC.c
int as_c() {
    return 0;
}


int common_c() {
    return 0;
}


/////////////////////////////////////////////////

// ./b/libC.c
int bs_c() {
    return 10;
}


int common_c() {
    return 10;
}

/////////////////////////////////////////////////

// ./main.c

#include "a/libA.h"
#include "b/libB.h"
#include "b/libC.h"
#include <stdio.h>

int main() {
    printf("a: %d\n", a_fn());
    printf("b: %d\n", b_fn());
    printf("common: %d\n", common_c());
    return 0;
}
```
These two files have one common function, but different function for `as_c` and `bs_c`


# The problem
What does the linker do when it finds a file of the same name (that it has encountered before), but at a different path?
- Does it ignore the file and use the older file as a substitute?
  - in this case, it is guaranteed that this `main` won't work
    - `libA.c` depends on existence of `as_c` and `libB.c` depends on existence of `bs_c`
    - if only one is loaded globally, only one of `as_c` and `bs_c` can stay in the global symbol table
- what about the same symbols? If I access `common_c` in `main`, which symbol wins? this is symbol clashing
  - what about the same symbol being used in `libA.so` and `libB.so` (remember that they are linked separately to the correct so of their need)

# TL;DR
`dyld` in mac allows transitively depending on library's with same path name even if they are incompatible. `ld.so` in linux does not, you can only load one version of a given path name in the whole process.  

# Details

## In mac
`dyld` is pretty smart. It knows that `libA.so` specifically depends on `a/libC.so` and all symbols inside `libA.so` would resolve to the ones in `a/libC.so`, there is literally no symbol clash. everything works. Same for `libB.so`. For the case of `main`, the only problem is `common_c()` function, this symbol is clashing, for our purposes, we can assume `dyld` arbitrarily uses one of the duplicate definitions to link `main`. (any one of `a/libC.so` and `b/libC.so` would be used).  
In essence, `dyld` allows you to have libraries with the same name being depended on transitively, even if their content is semantically different.  
If you however, put any one of `libC.so` in `DYLD_LIBRARY_PATH`, `dyld` would forcibly link that and stuff would fail (you can't have two copies now). That's why putting everything in `DYLD_LIBRARY_PATH` is bad  

The makefile used for testing this

```makefile
all: main

main: main.c a/libA.so b/libB.so
	clang -o main main.c -L./a -L./b -lA -lB

b/libB.so: b/libB.c b/libC.so
	clang -dynamiclib -o ./b/libB.so ./b/libB.c -lC -L./b

b/libC.so: b/libC.c
	clang -dynamiclib -o ./b/libC.so ./b/libC.c

a/libA.so: a/libA.c a/libC.so
	clang -dynamiclib -o ./a/libA.so ./a/libA.c -lC -L./a

a/libC.so: a/libC.c
	clang -dynamiclib -o ./a/libC.so ./a/libC.c

clean:
	rm -f ./a/*.so ./b/*.so main
```

This works, and is successfully able to create the correct binary. If you change the order of the search path in main's command from `-L./a -L./b` to `-L./b -L./a`, it would mostly prefer `libC.so` inside `b` for resolving conflicting symbols in main (but A and B would get the correct ones)

## In linux
The makefile
```makefile
all: main

main: main.c a/libA.so b/libB.so
	gcc -o main main.c -L./b -L./a -lA -lB -lC

b/libB.so: b/libB.c b/libC.so
	gcc -shared -Wl,-soname,libB.so -Wl,-rpath,'$$ORIGIN' -o ./b/libB.so ./b/libB.c -lC -L./b

b/libC.so: b/libC.c
	gcc -shared -Wl,-soname,libC.so -o ./b/libC.so ./b/libC.c

a/libA.so: a/libA.c a/libC.so
	gcc -shared -Wl,-soname,libA.so -Wl,-rpath,'$$ORIGIN' -o ./a/libA.so ./a/libA.c -lC -L./a

a/libC.so: a/libC.c
	gcc -shared -Wl,-soname,libC.so -o ./a/libC.so ./a/libC.c

clean:
	rm -f ./a/*.so ./b/*.so main
```

We are setting the rpaths and sonames correctly, but it would still fail to create `main` successfully.  
This is because linux runtime can only load any library with the same path name ONCE (not SONAME, path name, the runtime linker does not care about SONAME).  
The link editor in compiler is also failing, its keeping a single copy of a library for a given soname  


