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
You should run as much of your application code as possible, like running all the tests. This allows `shenzi` to detect every shared library linked to your application at runtime.  

Once you stop the application, a file `shenzi.json` (called the manifest) will be dumped in the current directory. This file contains all the shared library loads that `shenzi` detected. It also contains some information about your virtual environment.  
Now run the `shenzi` CLI with this manifest file

```bash
shenzi build ./shenzi.json
```
This can take a moment, after it is done, your application would be packaged in a `dist` folder.  
You can ship this `dist` folder to any target machine and it should work out of the box. The only required dependency is `bash`.  


Run `dist/bootstrap.sh` to run your application.  
```bash
# bootstrap.sh is the entrypoint for your application
# you can run this from any directory generally
bash dist/bootstrap.sh
```

# Roadmap

- windows support
- guiding users when a library is not installed in the development machine itself (some library is optional for some pip package, the shared library exists in site-packages but is never loaded [it doesn't work in the user's machine at all]). In this case, making the dist would fail too. Need to come up with a way to guide users in this
- better error messaging
- benchmarking and optimizations