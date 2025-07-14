use std::{fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub struct ErrDidNotFindDependency {
    pub lib: PathBuf,
    pub name: String,
}

impl std::error::Error for ErrDidNotFindDependency {}

impl fmt::Display for ErrDidNotFindDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Did not find dependency: '{}' (expected at path: {})",
            self.name,
            self.lib.display()
        )
    }
}


static DEP_NOT_FOUND_TEMPLATE: &str = r#"failed in finding dependencies for all the shared libraries in your python path
The list contains two entries for each failure, the path to the file whose dependencies we could not find, and the name of the dependency that wasn't found

This can happen when the library at the failed path (the second entry) is not used by your python application.  
Some packages ship shared libraries which are used in a plugin based fashion, in that case, if your system is not configured for that feature, the plugin won't be loaded.
This is why, right now `shenzi` emits a warning when it's not able to find a dependency for a shared library.  
A simple way to check whether this library is actually needed is to search the name of the dependency that was not found in the whole system

You can do that by running `find / -name 'name'` in bash
If the library is not anywhere in the system, it is safe to ignore this warning
What to put exactly in skip.libs?
    If you have an entry as `libtensorrt.so <- /path/to/libonnxruntime.so`
    Then you should keep "libonnxruntime.so" in `skip.libs` in your manifest file as a string in the array
    Manifest will look like this: { "skip": {"libs": ["libonnxruntime.so"]} },

In the worst case, it might be easy to simply put all the problems in `skip.libs` and let shenzi continue, in this case you would need to test the final built application.
Also, make sure you are running your application and testing various scenarios when you generated the manifest. 
Running as much as you can while shenzi is intercepting calls decreases the probability of errors.  


ERRORS TABLE
"#;

#[derive(Debug)]
pub struct ErrDidNotFindDependencies {
    pub causes: Vec<ErrDidNotFindDependency>,
}

impl std::error::Error for ErrDidNotFindDependencies {}

impl fmt::Display for ErrDidNotFindDependencies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", DEP_NOT_FOUND_TEMPLATE)?;
        writeln!(f, "NAME OF THE DEPENDENCY WE DID NOT FIND  <-  PATH OF THE FILE WE TRIED FINDING DEPENDENCY OF")?;
        for cause in &self.causes {
            let p = PathBuf::from(&cause.name);
            let final_comp = p.components().last();
            let name = match final_comp {
                Some(n) => n.as_os_str().to_string_lossy().to_string(),
                None => cause.name.to_string(),
            };
            writeln!(f, "  {} <- {}", name, cause.lib.display())?;
        }
        Ok(())
    }
}