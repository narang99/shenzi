// call ldd for finding the link path

use std::{path::PathBuf, process::Command, str::FromStr};

use anyhow::{Context, Result, bail};

pub fn find(name: &str, object_path: &PathBuf) -> Result<PathBuf> {
    let output = Command::new("ldd")
        .arg(object_path.to_str().context("failed in converting object path to string")?)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    let path = find_in_output(name, output).context("failed in finding linked library in ldd");
    match path {
        Ok(path) => {
            if path.exists() {
                Ok(path)
            } else {
                bail!("path does not exist")
            }
        }
        Err(e) => Err(e),
    }
}

fn find_in_output(name: &str, output: String) -> Option<PathBuf> {
    /* output format
        linux-vdso.so.1 (0x00007ffeb3bc5000)
        libcudnn.so.8 => /lib/x86_64-linux-gnu/libcudnn.so.8 (0x00007f777c1f2000)
        libcublas.so.11 => not found
        libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f777c1ec000)
        libnvinfer.so.8 => not found
        libnvinfer_plugin.so.8 => not found
        libnvonnxparser.so.8 => not found
        libcudart.so.11.0 => not found
        libz.so.1 => /lib/x86_64-linux-gnu/libz.so.1 (0x00007f777c1ce000)
        libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)
        librt.so.1 => /lib/x86_64-linux-gnu/librt.so.1 (0x00007f777c1a1000)
        libstdc++.so.6 => /lib/x86_64-linux-gnu/libstdc++.so.6 (0x00007f777bf33000)
        libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f777bde4000)
        libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f777bdbd000)
        libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
        /lib64/ld-linux-x86-64.so.2 (0x00007f777c6f8000)
    */

    for line in output.lines() {
        if !line.contains("=>") {
            continue;
        }
        let parts: Vec<&str> = line.trim().split("=>").map(|p| p.trim()).collect();
        if parts.len() > 1 && name == parts[0] {
            if let Some(result) = find_in_ldd_entry_value(parts[1]) {
                return Some(result);
            }
        }
    }
    None
}

fn find_in_ldd_entry_value(value: &str) -> Option<PathBuf> {
    // value = "/lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)"
    if value == "not found" {
        None
    } else {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() > 1 {
            if !parts[1].starts_with("(0x") {
                None
            } else {
                let p = PathBuf::from_str(parts[0]);
                match p {
                    Ok(p) => Some(p),
                    Err(_) => None,
                }
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::find_in_output;
    use std::path::PathBuf;

    #[test]
    fn test_find_in_output_found() {
        let name = "libc.so.6";
        let output = r#"
            linux-vdso.so.1 (0x00007ffd2b7fe000)
            libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)
            libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
            /lib64/ld-linux-x86-64.so.2 (0x00007f777c6f8000)
        "#;
        let result = find_in_output(name, output.to_string());
        assert_eq!(
            result,
            Some(PathBuf::from("/lib/x86_64-linux-gnu/libc.so.6"))
        );
    }

    #[test]
    fn test_find_in_output_not_found() {
        let name = "libdoesnotexist.so";
        let output = r#"
            linux-vdso.so.1 (0x00007ffd2b7fe000)
            libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)
            libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
            /lib64/ld-linux-x86-64.so.2 (0x00007f777c6f8000)
        "#;
        let result = find_in_output(name, output.to_string());
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_in_output_not_found_string() {
        let name = "libnotfound.so";
        let output = r#"
            libnotfound.so => not found
            libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
        "#;
        let result = find_in_output(name, output.to_string());
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_in_output_not_found_with_only_hex_value() {
        let name = "libnotfound.so";
        let output = r#"
            libnotfound.so => (0x00007f777bbcb000)
            libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
        "#;
        let result = find_in_output(name, output.to_string());
        assert_eq!(result, None);
    }
}
