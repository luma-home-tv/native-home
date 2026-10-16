//! Worker-side ordinary packaged icons only. The controller owns package epoch,
//! app identity and path associations; this module grants no Root/live-source
//! authority. No GL, filesystem writes or remote operations. File I/O/codec calls
//! are not interruptible and these output bounds are not a total-heap/time bound.
use std::{
    ffi::{CStr, CString},
    fs::{File, Metadata},
    io::{self, Read},
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};

pub const MAX_ENCODED_BYTES: usize = 1024 * 1024;
pub const MAX_AXIS: u32 = 512;
pub const MAX_RGBA_BYTES: usize = 1024 * 1024;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Closed package grammar, not a general filesystem path or URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeIconPath {
    leaf: CString,
}
impl RelativeIconPath {
    pub fn parse(path: &str) -> io::Result<Self> {
        let Some(leaf) = path.strip_prefix("icons/") else {
            return Err(invalid("Packaged icon must be icons/<leaf>"));
        };
        if leaf.is_empty()
            || leaf.len() > 255
            || matches!(leaf, "." | "..")
            || leaf
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
        {
            return Err(invalid("Invalid packaged icon leaf"));
        }
        Ok(Self {
            leaf: CString::new(leaf).map_err(|_| invalid("Invalid icon name"))?,
        })
    }
}

/// Writer-relevant identity; atime is deliberately excluded because reads may
/// update it. Owner/mode/link count are observations, not authority predicates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub links: u64,
    pub bytes: u64,
    pub modified_seconds: i64,
    pub modified_nanos: i64,
    pub changed_seconds: i64,
    pub changed_nanos: i64,
}
impl FileIdentity {
    fn of(meta: &Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            uid: meta.uid(),
            gid: meta.gid(),
            mode: meta.mode(),
            links: meta.nlink(),
            bytes: meta.len(),
            modified_seconds: meta.mtime(),
            modified_nanos: meta.mtime_nsec(),
            changed_seconds: meta.ctime(),
            changed_nanos: meta.ctime_nsec(),
        }
    }
    fn regular(&self) -> bool {
        self.mode & 0o170000 == 0o100000
    }
    fn directory(&self) -> bool {
        self.mode & 0o170000 == 0o040000
    }
}
fn stamp(file: &File) -> io::Result<FileIdentity> {
    Ok(FileIdentity::of(&file.metadata()?))
}
/// Directory identity ignores entry churn (size/link count/writer times),
/// retaining actual object/owner/permission identity. Never grants authority.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl DirectoryIdentity {
    fn of(observed: FileIdentity) -> io::Result<Self> {
        if !observed.directory() {
            return Err(invalid("Icon directory is not a directory"));
        }
        Ok(Self {
            device: observed.device,
            inode: observed.inode,
            uid: observed.uid,
            gid: observed.gid,
            mode: observed.mode,
        })
    }
}
fn directory_stamp(file: &File) -> io::Result<DirectoryIdentity> {
    DirectoryIdentity::of(stamp(file)?)
}
fn same_directory(actual: DirectoryIdentity, expected: &DirectoryIdentity) -> io::Result<()> {
    if actual != *expected {
        return Err(invalid("Packaged icon directory changed"));
    }
    Ok(())
}
fn named_directory(parent: &File, name: &CStr) -> io::Result<DirectoryIdentity> {
    DirectoryIdentity::of(named_stamp(parent, name)?)
}
fn open_at(parent: &File, name: &CStr, directory: bool) -> io::Result<File> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
// libc stat field widths differ between macOS and the ARM32 TV ABI.
#[allow(clippy::unnecessary_cast)]
fn named_stamp(parent: &File, name: &CStr) -> io::Result<FileIdentity> {
    let mut raw = MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            raw.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let s = unsafe { raw.assume_init() };
    if s.st_size < 0 {
        return Err(invalid("Negative icon metadata size"));
    }
    Ok(FileIdentity {
        device: s.st_dev as u64,
        inode: s.st_ino as u64,
        uid: s.st_uid,
        gid: s.st_gid,
        mode: s.st_mode as u32,
        links: s.st_nlink as u64,
        bytes: s.st_size as u64,
        modified_seconds: s.st_mtime as i64,
        modified_nanos: s.st_mtime_nsec as i64,
        changed_seconds: s.st_ctime as i64,
        changed_nanos: s.st_ctime_nsec as i64,
    })
}
fn same(actual: FileIdentity, expected: &FileIdentity) -> io::Result<()> {
    if actual != *expected {
        return Err(invalid("Packaged icon path or file changed"));
    }
    Ok(())
}

pub struct EncodedIcon {
    pub bytes: Vec<u8>,
    pub identity: FileIdentity,
}
pub struct DecodedIcon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub encoded_identity: FileIdentity,
}

pub struct PackagedIconReader {
    directory: File,
    anchor: DirectoryIdentity,
}
impl PackagedIconReader {
    /// The caller opens its own bundle/assets directory on the worker. This
    /// pins that descriptor, not an arbitrary pathname or a trusted installation.
    pub fn from_directory(directory: File) -> io::Result<Self> {
        let anchor = directory_stamp(&directory)?;
        Ok(Self { directory, anchor })
    }
    pub fn read(&self, path: &RelativeIconPath) -> io::Result<EncodedIcon> {
        self.read_checked(path, || {})
    }
    fn read_checked(
        &self,
        path: &RelativeIconPath,
        after_read: impl FnOnce(),
    ) -> io::Result<EncodedIcon> {
        same_directory(directory_stamp(&self.directory)?, &self.anchor)?;
        let icons = open_at(&self.directory, c"icons", true)?;
        let icons_before = directory_stamp(&icons)?;
        same_directory(named_directory(&self.directory, c"icons")?, &icons_before)?;
        // Check named type/size before opening a possibly special file; nofollow
        // and nonblock also constrain a replacement raced between these calls.
        let before = named_stamp(&icons, &path.leaf)?;
        if !before.regular() || before.bytes == 0 || before.bytes > MAX_ENCODED_BYTES as u64 {
            return Err(invalid("Icon is not a nonempty bounded regular file"));
        }
        let file = open_at(&icons, &path.leaf, false)?;
        same(stamp(&file)?, &before)?;
        same(named_stamp(&icons, &path.leaf)?, &before)?;
        // Buffer <= cap+1; one extra byte detects growth without reading an
        // unbounded file. Stable length is required even when growth is < cap.
        let mut bytes = vec![0; before.bytes as usize + 1];
        let mut limited = (&file).take(MAX_ENCODED_BYTES as u64 + 1);
        let mut used = 0;
        while used < bytes.len() {
            let n = limited.read(&mut bytes[used..])?;
            if n == 0 {
                break;
            }
            used += n;
        }
        after_read();
        same(stamp(&file)?, &before)?;
        same(named_stamp(&icons, &path.leaf)?, &before)?;
        same_directory(directory_stamp(&icons)?, &icons_before)?;
        same_directory(named_directory(&self.directory, c"icons")?, &icons_before)?;
        same_directory(directory_stamp(&self.directory)?, &self.anchor)?;
        if used != before.bytes as usize {
            return Err(invalid("Icon length changed during read"));
        }
        bytes.truncate(used);
        Ok(EncodedIcon {
            bytes,
            identity: before,
        })
    }
    pub fn decode(&self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        self.decode_using(path, crate::icon_decode::decode)
    }
    fn decode_using(
        &self,
        path: &RelativeIconPath,
        decoder: impl FnOnce(&[u8]) -> Option<(u32, u32, Vec<u8>)>,
    ) -> io::Result<DecodedIcon> {
        let encoded = self.read(path)?;
        let (width, height, rgba) =
            decoder(&encoded.bytes).ok_or_else(|| invalid("Bounded icon decode failed"))?;
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4));
        if width == 0
            || height == 0
            || width > MAX_AXIS
            || height > MAX_AXIS
            || expected != Some(rgba.len() as u64)
            || rgba.len() > MAX_RGBA_BYTES
            || rgba.capacity() > MAX_RGBA_BYTES
        {
            return Err(invalid("Decoded icon exceeds bounds"));
        }
        Ok(DecodedIcon {
            width,
            height,
            rgba,
            encoded_identity: encoded.identity,
        })
    }
}

#[cfg(test)]
#[path = "packaged_icon_reader_tests.rs"]
mod tests;
