basics to revisit in the end


- Most important: follow linker as closely as possible. python and rust code: how they interact in this?
  - install names vs path names
  - loading in the exact order as the linker did
  - what should be the name in reals?
  - versioning? how does it affect stuff? is `libHello.so` same as `libHello.so.1`?
  - naming conventions of binaries
  - which patterns should be searched?
- Fat binaries, how to handle them?
- Error messaging (keep an error enum which we downcast in the end for CLI based error enum printing, like E001 is a variant kinda thing)
  - sometimes libraries might not be resolvable at all (even on the system of the user)
  - this can happen if they are, lets say plug and play
  - as an example, in numba in my system, tbbpool.cpython-39-x86_64-linux-gnu.so is not resolvable
  - and running `numba -s` (which prints information and metadata on numba) says that TBB is disabled
  - TBB is plug and play here
  - same for onnx when tensorrt is not available
    - we need to provide users with options to consider them as plain files only
    - and good error messaging
    - it would be useful to have beautiful error messages by framing these things as special  cases
    - that is: hard code this in code, that numba might fail, and give user the option to fix it
    - its important if we can somehow decide when its okay to ignore a problematic library
    - add support in manifest for this too (allowing users to ignore some libraries if they are not resolvable)
- number of clones in the application, add more lifetime specifiers for read only stuff at least
- add pre requisite commands download and checks
  - like patchelf and install_name_tool
  - add checks in the beginning, so that user does not need to wait for a long time