//! `live-inject <pid> <demaster_live.dll> <mod dir>`: loads the live music
//! engine into a running FFX.exe and installs it, for tests where no C#
//! mod runs (ffx-rust's harness, `tools/shotrun`). The C# mod does the
//! same with `LoadLibrary` and `demaster_live_install` in the game.
//! Build it for i686 (`tools/build-live.sh`): the 32-bit `LoadLibraryW`
//! is at the same address in every 32-bit process.

#[cfg(windows)]
fn main() {
    std::process::exit(win::run());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("live-inject runs on Windows only");
    std::process::exit(2);
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn VirtualAllocEx(p: isize, addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
        fn WriteProcessMemory(p: isize, addr: *mut c_void, buf: *const c_void, size: usize, written: *mut usize) -> i32;
        fn CreateRemoteThread(
            p: isize,
            sa: *mut c_void,
            stack: usize,
            start: usize,
            param: *mut c_void,
            flags: u32,
            id: *mut u32,
        ) -> isize;
        fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        fn GetExitCodeThread(h: isize, code: *mut u32) -> i32;
        fn CloseHandle(h: isize) -> i32;
        fn GetModuleHandleW(name: *const u16) -> usize;
        fn GetProcAddress(m: usize, name: *const u8) -> usize;
        fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> usize;
        fn GetLastError() -> u32;
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Writes `s` into the process and runs `start(s)` there; the thread's
    /// exit code.
    unsafe fn call(p: isize, start: usize, s: &[u16]) -> Option<u32> {
        unsafe {
            let mem = VirtualAllocEx(p, std::ptr::null_mut(), s.len() * 2, 0x3000, 0x04);
            if mem.is_null() {
                return None;
            }
            let mut n = 0;
            if WriteProcessMemory(p, mem, s.as_ptr() as *const c_void, s.len() * 2, &mut n) == 0 {
                return None;
            }
            let t = CreateRemoteThread(p, std::ptr::null_mut(), 0, start, mem, 0, std::ptr::null_mut());
            if t == 0 {
                return None;
            }
            if WaitForSingleObject(t, 180_000) != 0 {
                CloseHandle(t);
                return None;
            }
            let mut code = 0;
            GetExitCodeThread(t, &mut code);
            CloseHandle(t);
            Some(code)
        }
    }

    pub fn run() -> i32 {
        let a: Vec<String> = std::env::args().collect();
        if a.len() != 4 {
            eprintln!("usage: live-inject <pid> <demaster_live.dll> <mod dir>");
            return 2;
        }
        let pid: u32 = a[1].parse().expect("pid");
        let dll = wide(&a[2]);
        // SAFETY: Win32 calls on our own handles and buffers.
        unsafe {
            // PROCESS_CREATE_THREAD | VM_OPERATION | VM_WRITE | VM_READ | QUERY_INFORMATION
            let p = OpenProcess(0x2 | 0x8 | 0x20 | 0x10 | 0x400, 0, pid);
            if p == 0 {
                eprintln!("OpenProcess({pid}) failed: {}", GetLastError());
                return 1;
            }
            let k32 = GetModuleHandleW(wide("kernel32.dll").as_ptr());
            let load = GetProcAddress(k32, c"LoadLibraryW".as_ptr() as *const u8);
            let Some(remote) = call(p, load, &dll).filter(|&m| m != 0) else {
                eprintln!("LoadLibraryW in the game failed");
                return 1;
            };
            // the export's offset in the DLL, from a copy mapped here
            // without running it (DONT_RESOLVE_DLL_REFERENCES)
            let local = LoadLibraryExW(dll.as_ptr(), 0, 1);
            let f = GetProcAddress(local, c"demaster_live_install".as_ptr() as *const u8);
            if local == 0 || f == 0 {
                eprintln!("demaster_live_install not found in {}", a[2]);
                return 1;
            }
            let entry = remote as usize + (f - local);
            let r = call(p, entry, &wide(&a[3]));
            println!("loaded at {remote:08x}, demaster_live_install -> {r:?}");
            CloseHandle(p);
            if r == Some(0) { 0 } else { 1 }
        }
    }
}
