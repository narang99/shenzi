# shenzi

`shenzi` helps you create standalone Python applications from your development virtual environment. Using `shenzi`, you can create standalone folders which can be distributed to any machine, and the application will work (even when python is not installed on the target system).  

## The python packaging problem
Given a development environment (a virtual environment), we want to produce a single directory containing ALL the dependencies that the application needs. Other languages like `rust` and `go` provide easy way to create statically linked executables, which makes them very easy to distribute.  
Python struggles in this area mainly because of how flexible it is when it comes to delegating work to C code (shared libraries on your system).   

Out in the wild, python libraries regularly links to shared libraries in your system:
- [C Extensions](https://docs.python.org/3/extending/extending.html)
- loading shared libraries using `dlopen` and equivalents

Even creating a development environment for some pip package might require you to install some system dependencies (a good example is [weasyprint](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html#installation))   
It becomes difficult to ship applications if we need to install system dependencies in target machines. Docker solves this problem by packaging everything in a single docker image.  
`shenzi` does not compete with `docker`, if you can use `docker`, you should. `shenzi` is useful for shipping desktop applications.  

# Getting Started

First install `shenzi` in your virtual environment.  
```bash
pip install shenzi
```

In you main script, add the following lines
```python
import os

if os.environ.get("SHENZI_INIT_DISCOVERY", "False") == "True":
    from shenzi.discovery import shenzi_init_discovery
    shenzi_init_discovery()
```

Run your application as you normally do. `shenzi` will start intercepting all shared libraries that your code is importing.  
You should run as much of your application code as possible, like running all the tests. This allows `shenzi` to detect every dependency linked to your application at runtime.  

Once you stop the application, a file `shenzi.json` (called the manifest) will be dumped in the current directory. This file contains all the shared library loads that `shenzi` detected. It also contains some information about your virtual environment.  
Now run the `shenzi` CLI with this manifest file

```bash
RUST_LOG=INFO shenzi build ./shenzi.json
```
This can take a moment, after it is done, your application would be packaged in a `dist` folder.  
You can ship this `dist` folder to any target machine and it should work out of the box. The only required dependency is `bash`.  


Run `dist/bootstrap.sh` to run your application.  
```bash
# bootstrap.sh is the entrypoint for your application
# you can run this from any directory generally
bash dist/bootstrap.sh
```

You should at least read the doc which describes the structure of `shenzi.json` [here](/docs/manifest.md).  

If you use this, feel free to raise an issue on any problem, I need feedback for this :)

# How is this different?
I will add a small comparison to PyInstaller, which I feel is the most mature tool in the ecosystem.  
From what I've seen, PyInstaller statically analyses your python code (and does some imports too) to create the smallest possible packaged application. It is smarter than `shenzi`.  

`shenzi` is much simpler, all it does it greedily take everything in your python path and put it in the final distribution. For shared libraries, it closely tries to resemble the linker to find all the dependencies of each shared library, and put that in the application too.  
The motive here is to be as similar to the original development environment as possible, `shenzi` only changes how the shared libraries in the codebase find dependencies.  
This makes `shenzi` faster in some cases (where you have complex applications, as we do not do any static analysis), but slower in others (mainly if your virtual environment is huge, and not all dependencies are used by your application normally)   

Apart from that, there are some other internal differences that may or may not matter
- The structure of the final application (described [here](/docs/dist-structure.md))
- The bootstrap script in `shenzi` is pretty a simple bash script, it simply sets up the correct Python environment variables and starts the interpreter. PyInstaller has a very sophisticated bootstrapping CLI written in C

# Supported Platforms

Currently only Mac and Linux are supported.  
The project is very new right now, I've tested it on Ubuntu 20.04 and MacOS Sequoia.  