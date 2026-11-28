//! The parent process tag of the repeat hint on Windows: the parent's id and the moment it was created (its `FILETIME`), so a later process that reuses
//! the id has a count of its own. The standard library has no parent id on Windows, so the few `kernel32` calls are declared here and nowhere else
//! (no dependency, the only `unsafe` of the hint). Any failure is `None`: no tag, no hint, never an error.

use std::ffi::c_void;

type Handle = *mut c_void;

#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    id: u32,
    heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    priority: i32,
    flags: u32,
    exe: [u16; 260],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
    fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    fn GetProcessTimes(
        process: Handle,
        created: *mut FileTime,
        exited: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

const SNAPSHOT_PROCESSES: u32 = 2;
const QUERY_LIMITED: u32 = 0x1000;
const INVALID: isize = -1;

/// The id of the process that started this one, from a snapshot of the process list.
fn parent_id() -> Option<u32> {
    // SAFETY: plain calls with a zeroed, correctly sized `ProcessEntry`; the snapshot handle is closed on every path.
    unsafe {
        let me = GetCurrentProcessId();
        let snap = CreateToolhelp32Snapshot(SNAPSHOT_PROCESSES, 0);
        if snap.is_null() || snap as isize == INVALID {
            return None;
        }
        let mut e: ProcessEntry = std::mem::zeroed();
        e.size = std::mem::size_of::<ProcessEntry>() as u32;
        let mut found = None;
        let mut more = Process32FirstW(snap, &mut e) != 0;
        while more {
            if e.id == me {
                found = Some(e.parent);
                break;
            }
            more = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
        found
    }
}

/// The creation time of process `pid` (100 ns ticks since 1601), `None` when it cannot be opened.
pub fn created(pid: u32) -> Option<u64> {
    // SAFETY: the handle is checked and closed; the four out-parameters are valid `FileTime`s.
    unsafe {
        let h = OpenProcess(QUERY_LIMITED, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut t = [FileTime::default(); 4];
        let ok = GetProcessTimes(h, &mut t[0], &mut t[1], &mut t[2], &mut t[3]) != 0;
        CloseHandle(h);
        ok.then(|| (u64::from(t[0].high) << 32) | u64::from(t[0].low))
    }
}

/// `PARENTID-CREATED`, or `None` when the parent cannot be found or opened.
pub fn parent_tag() -> Option<String> {
    let id = parent_id()?;
    Some(format!("{id}-{}", created(id)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parent_is_tagged_by_its_id_and_creation_time() {
        let tag = parent_tag().expect("the test runner has a parent");
        let (id, at) = tag.split_once('-').unwrap();
        let (id, at): (u32, u64) = (id.parse().unwrap(), at.parse().unwrap());
        assert!(id != 0 && at != 0, "{tag}");
        assert_eq!(parent_tag().as_deref(), Some(tag.as_str()), "stable");
        // a child is created after its parent
        let mine = created(unsafe { GetCurrentProcessId() }).unwrap();
        assert!(mine >= at, "{mine} < {at}");
    }
}
