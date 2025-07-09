pub fn get_at_platform() -> String {
    get_platform_impl()
}

#[cfg(all(target_os = "linux", feature = "linux-platform"))]
fn get_platform_impl() -> String {
    use std::ffi::CStr;
    use libc;

    const AT_PLATFORM: libc::c_ulong = 15;

    unsafe {
        let ptr = libc::getauxval(AT_PLATFORM);
        if ptr != 0 {
            if let Ok(s) = CStr::from_ptr(ptr as *const libc::c_char).to_str() {
                return s.to_owned();
            }
        }
    }

    panic!("could not find the aux vector value AT_PLATFORM");
}

#[cfg(not(all(target_os = "linux", feature = "linux-platform")))]
fn get_platform_impl() -> String {
    String::new()
}