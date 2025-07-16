// module for parsing python lock files, we use this to trim down the dependencies that we need to package
// when calling `shenzi init`, the CLI would try to detect python project managers and add them to our workspace configuration file
// we only return the packages which are "required"
// not that stuff dlopen'ed and outside the package might still be intercepted (unless it is inside the package)
// we'll see what we can do about that later



pub mod poetry;
pub mod common;