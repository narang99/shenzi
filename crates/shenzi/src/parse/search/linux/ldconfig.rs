use std::path::PathBuf;

use anyhow::{Result, anyhow};
use std::process::Command;

use crate::paths::to_path_buf;

pub fn find(name: &str) -> Result<PathBuf> {
    let output = Command::new("/sbin/ldconfig")
        .arg("-p")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    find_in_output(name, output).ok_or(anyhow!("failed in finding library {}", name))
}

fn find_in_output(name: &str, output: String) -> Option<PathBuf> {
    let candidates: Vec<&str> = output
        .lines()
        .filter_map(|line| get_candidate(name, line))
        .collect();

    // first only search for exact match
    // if we are searching for libhello.so, we want libhello.so, not libhello.so.2
    for candidate in &candidates {
        if **candidate == *name {
            let path_buf = to_path_buf(candidate);
            if let Ok(path_buf) = path_buf {
                return Some(path_buf);
            }
        }
    }

    // send the first existing candidate
    for candidate in &candidates {
        let path_buf = to_path_buf(candidate);
        if let Ok(path_buf) = path_buf {
            return Some(path_buf);
        }
    }

    None
}

fn get_candidate<'a>(name: &str, line: &'a str) -> Option<&'a str> {
    let comps: Vec<&str> = line.split("=>").collect();
    if comps.len() < 2 {
        return None;
    }
    let candidate = comps[1];
    if candidate.contains(name) {
        return Some(candidate.trim());
    }
    None
}

#[cfg(test)]
mod test {

    use super::find_in_output;

    #[test]
    fn test_find_in_output_exact_match() {
        let name = "libhello.so";
        let output = "\
        1404 libs found in cache `/etc/ld.so.cache'
            libhello.so (libc6,x86-64) => /usr/lib/libhello.so\n\
            libhello.so.2 (libc6,x86-64) => /usr/lib/libhello.so.2\n\
        "
        .to_string();

        let result = find_in_output(name, output);
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with("libhello.so"));
    }

    #[test]
    fn test_find_in_output_no_exact_match_but_candidate_exists() {
        let name = "libfoo.so";
        let output = "\
            libfoo.so.1 (libc6,x86-64) => /usr/lib/libfoo.so.1\n\
            libbar.so (libc6,x86-64) => /usr/lib/libbar.so\n\
        "
        .to_string();

        let result = find_in_output(name, output);
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with("libfoo.so.1"));
    }

    #[test]
    fn test_find_in_output_no_match() {
        let name = "libnotfound.so";
        let output = "\
            libfoo.so.1 (libc6,x86-64) => /usr/lib/libfoo.so.1\n\
            libbar.so (libc6,x86-64) => /usr/lib/libbar.so\n\
        "
        .to_string();

        let result = find_in_output(name, output);
        assert!(result.is_none());
    }
}
