# Structure of the dist folder

TODO

## Mac
symlink farm
mac has nested namespace for symbols

## Linux
simply put everything in reals in ld library path
linux does not have nested namespace for symbols or library names (only one library is loaded for a given SONAME), this makes a flat namespace ok to use
we dont create any symlink farm
this is similar to what we'll mostly do for windows later also
The main difficulty with linux was search

## Windows
Not started