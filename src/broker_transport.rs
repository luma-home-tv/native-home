//! Proof-only Linux abstract UNIX sequenced-packet transport. No paths, commands,
//! URIs, identity policy, or stream fallback. A failed operation is terminal:
//! callers must discard the connection rather than retry a protocol exchange.
use std::io;
use std::time::Instant;

pub const MAX_MESSAGE_BYTES: usize = 4096;
#[cfg(any(target_os = "linux", test))]
const PREFIX: &str = "luma-home.artwork.proof.";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}
#[derive(Debug)]
pub struct Packet {
    pub bytes: Vec<u8>,
    pub credentials: Credentials,
}
/// A result of ONE sequenced-packet receive. EOF is not an empty packet.
/// Peer write-half-close can also produce EOF; this proves no process/child exit.
/// Foundation only: no protocol or session acceptance follows from this type.
#[derive(Debug)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) enum Incoming {
    Packet(Packet),
    Eof,
}
#[derive(Debug)]
pub struct Listener {
    #[cfg(target_os = "linux")]
    fd: std::os::fd::OwnedFd,
}
#[derive(Debug)]
pub struct Connection {
    #[cfg(target_os = "linux")]
    fd: std::os::fd::OwnedFd,
}
#[cfg(any(target_os = "linux", test))]
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[cfg(any(target_os = "linux", test))]
fn abstract_name(nonce: &str) -> io::Result<Vec<u8>> {
    if nonce.len() != 32
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("Expected32lowercasehex nonce"));
    }
    Ok(format!("{PREFIX}{nonce}").into_bytes())
}

// Linux cmsghdr contains native size_t length and two native-endian i32 fields;
// records align to size_t. Parse bounded bytes without unaligned pointer reads.
// Inspect the entire received control area even on rejection, closing every
// delivered SCM_RIGHTS FD. The kernel closes rights omitted by MSG_CTRUNC.
#[cfg(any(target_os = "linux", test))]
fn credentials_from_control(
    control: &[u8],
    mut close_fd: impl FnMut(i32),
) -> io::Result<Credentials> {
    let word = std::mem::size_of::<usize>();
    let header = word + 8;
    let mut at = 0_usize;
    let mut credential = None;
    let mut rejected = false;
    while at < control.len() {
        let remaining = &control[at..];
        if remaining.len() < header {
            rejected = true;
            break;
        }
        let size = usize::from_ne_bytes(remaining[..word].try_into().expect("size_t slice"));
        let level = i32::from_ne_bytes(remaining[word..word + 4].try_into().expect("level slice"));
        let kind = i32::from_ne_bytes(remaining[word + 4..header].try_into().expect("type slice"));
        if size < header {
            rejected = true;
            break;
        }
        let present = size.min(remaining.len());
        let payload = &remaining[header..present];
        if level == 1 && kind == 1 {
            // SOL_SOCKET / SCM_RIGHTS on Linux.
            for fd in payload.as_chunks::<4>().0 {
                let fd = i32::from_ne_bytes(*fd);
                if fd >= 0 {
                    close_fd(fd);
                }
            }
            rejected = true;
        } else if level == 1 && kind == 2 {
            // SOL_SOCKET / SCM_CREDENTIALS.
            if payload.len() != 12 || size != header + 12 || credential.is_some() {
                rejected = true;
            } else {
                let pid = i32::from_ne_bytes(payload[..4].try_into().expect("pid slice"));
                let uid = u32::from_ne_bytes(payload[4..8].try_into().expect("uid slice"));
                let gid = u32::from_ne_bytes(payload[8..12].try_into().expect("gid slice"));
                if pid <= 0 {
                    rejected = true;
                }
                credential = Some(Credentials { pid, uid, gid });
            }
        } else {
            rejected = true;
        }
        if size > remaining.len() {
            rejected = true;
            break;
        }
        let Some(aligned) = size.checked_add(word - 1).map(|n| n & !(word - 1)) else {
            rejected = true;
            break;
        };
        if aligned >= remaining.len() {
            break;
        }
        at += aligned;
    }
    if rejected {
        return Err(invalid("Rejected ancillary data"));
    }
    credential.ok_or_else(|| invalid("Missing kernel credentials"))
}

// Classify the SAME recvmsg result while disposing of delivered rights first.
// Synthetic host tests cover this classifier, not Linux kernel semantics.
#[cfg(any(target_os = "linux", test))]
fn incoming_from_received(
    mut bytes: Vec<u8>,
    received: usize,
    max_bytes: usize,
    control: &[u8],
    truncated: bool,
    control_truncated: bool,
    close_fd: impl FnMut(i32),
) -> io::Result<Incoming> {
    let credentials = credentials_from_control(control, close_fd);
    if max_bytes == 0 || max_bytes > MAX_MESSAGE_BYTES {
        return Err(invalid("Invalid receive bound"));
    }
    if truncated || control_truncated || received > max_bytes || received > bytes.len() {
        return Err(invalid("Empty, truncated or oversized packet"));
    }
    if received == 0 {
        return if control.is_empty() {
            Ok(Incoming::Eof)
        } else {
            Err(invalid("Empty, truncated or oversized packet"))
        };
    }
    bytes.truncate(received);
    Ok(Incoming::Packet(Packet {
        bytes,
        credentials: credentials?,
    }))
}

fn packet_only(incoming: Incoming) -> io::Result<Packet> {
    match incoming {
        Incoming::Packet(packet) => Ok(packet),
        Incoming::Eof => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Empty, truncated or oversized packet",
        )),
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::mem::{size_of, zeroed};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

    fn timeout() -> io::Error {
        io::ErrorKind::TimedOut.into()
    }
    fn check_deadline(deadline: Instant) -> io::Result<()> {
        if Instant::now() >= deadline {
            Err(timeout())
        } else {
            Ok(())
        }
    }
    fn milliseconds(deadline: Instant) -> io::Result<i32> {
        let left = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(timeout)?;
        let ms = left
            .as_millis()
            .saturating_add(if left.subsec_nanos() % 1_000_000 != 0 {
                1
            } else {
                0
            });
        Ok(ms.clamp(1, i32::MAX as u128) as i32)
    }
    fn wait(fd: RawFd, events: i16, deadline: Instant) -> io::Result<()> {
        loop {
            let mut p = libc::pollfd {
                fd,
                events,
                revents: 0,
            };
            let rc = unsafe { libc::poll(&mut p, 1, milliseconds(deadline)?) };
            if rc < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            if rc == 0 {
                return Err(timeout());
            }
            if p.revents & libc::POLLNVAL != 0 {
                return Err(invalid("Invalid socket descriptor"));
            }
            if p.revents & (events | libc::POLLERR | libc::POLLHUP) != 0 {
                return Ok(());
            }
        }
    }
    fn socket() -> io::Result<OwnedFd> {
        let fd = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        passcred(fd.as_raw_fd())?;
        Ok(fd)
    }
    fn passcred(fd: RawFd) -> io::Result<()> {
        let yes: libc::c_int = 1;
        if unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&yes as *const libc::c_int).cast(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn address(nonce: &str) -> io::Result<(libc::sockaddr_un, libc::socklen_t)> {
        let name = abstract_name(nonce)?;
        let mut addr: libc::sockaddr_un = unsafe { zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
        if name.len() + 1 > addr.sun_path.len() {
            return Err(invalid("Abstract name too long"));
        }
        for (dest, byte) in addr.sun_path[1..].iter_mut().zip(&name) {
            *dest = *byte as libc::c_char;
        }
        let len = std::mem::offset_of!(libc::sockaddr_un, sun_path) + 1 + name.len();
        Ok((addr, len as libc::socklen_t))
    }
    impl Listener {
        pub fn bind(nonce: &str) -> io::Result<Self> {
            let (addr, len) = address(nonce)?;
            let fd = socket()?;
            if unsafe {
                libc::bind(
                    fd.as_raw_fd(),
                    (&addr as *const libc::sockaddr_un).cast(),
                    len,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            if unsafe { libc::listen(fd.as_raw_fd(), 1) } != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { fd })
        }
        pub fn accept_until(&self, deadline: Instant) -> io::Result<Connection> {
            loop {
                check_deadline(deadline)?;
                let fd = unsafe {
                    libc::accept4(
                        self.fd.as_raw_fd(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                    )
                };
                if fd >= 0 {
                    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
                    passcred(fd.as_raw_fd())?;
                    return Ok(Connection { fd });
                }
                let e = io::Error::last_os_error();
                match e.kind() {
                    io::ErrorKind::Interrupted => continue,
                    io::ErrorKind::WouldBlock => wait(self.fd.as_raw_fd(), libc::POLLIN, deadline)?,
                    _ => return Err(e),
                }
            }
        }
    }
    impl Connection {
        /// After the authenticated Finished packet, accept only clean kernel
        /// EOF. A zero-byte credentialed packet is a message, never EOF.
        pub fn expect_eof_until(&self, deadline: Instant) -> io::Result<()> {
            loop {
                check_deadline(deadline)?;
                let mut byte = 0_u8;
                let mut control = [0_usize; 512];
                let mut iov = libc::iovec {
                    iov_base: (&mut byte as *mut u8).cast(),
                    iov_len: 1,
                };
                let mut message: libc::msghdr = unsafe { zeroed() };
                message.msg_iov = &mut iov;
                message.msg_iovlen = 1;
                message.msg_control = control.as_mut_ptr().cast();
                message.msg_controllen = size_of::<[usize; 512]>();
                let rc = unsafe {
                    libc::recvmsg(
                        self.fd.as_raw_fd(),
                        &mut message,
                        libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
                    )
                };
                if rc < 0 {
                    let error = io::Error::last_os_error();
                    match error.kind() {
                        io::ErrorKind::Interrupted => continue,
                        io::ErrorKind::WouldBlock => {
                            wait(self.fd.as_raw_fd(), libc::POLLIN, deadline)?;
                            continue;
                        }
                        _ => return Err(error),
                    }
                }
                let length = message.msg_controllen.min(size_of::<[usize; 512]>());
                let ancillary =
                    unsafe { std::slice::from_raw_parts(control.as_ptr().cast::<u8>(), length) };
                // Dispose of any rights even though every further packet is rejected.
                let _ = credentials_from_control(ancillary, |fd| unsafe {
                    libc::close(fd);
                });
                if rc == 0
                    && length == 0
                    && message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) == 0
                {
                    return Ok(());
                }
                return Err(invalid("Unexpected packet after Finished"));
            }
        }
        pub fn connect_until(nonce: &str, deadline: Instant) -> io::Result<Self> {
            let (addr, len) = address(nonce)?;
            let fd = socket()?;
            loop {
                check_deadline(deadline)?;
                let rc = unsafe {
                    libc::connect(
                        fd.as_raw_fd(),
                        (&addr as *const libc::sockaddr_un).cast(),
                        len,
                    )
                };
                if rc == 0 {
                    return Ok(Self { fd });
                }
                let e = io::Error::last_os_error();
                match e.raw_os_error() {
                    Some(libc::EISCONN) => return Ok(Self { fd }),
                    Some(libc::EINTR) => continue,
                    Some(libc::EINPROGRESS) | Some(libc::EALREADY) => {
                        wait(fd.as_raw_fd(), libc::POLLOUT, deadline)?;
                        let mut status: libc::c_int = 0;
                        let mut length = size_of::<libc::c_int>() as libc::socklen_t;
                        if unsafe {
                            libc::getsockopt(
                                fd.as_raw_fd(),
                                libc::SOL_SOCKET,
                                libc::SO_ERROR,
                                (&mut status as *mut libc::c_int).cast(),
                                &mut length,
                            )
                        } != 0
                        {
                            return Err(io::Error::last_os_error());
                        }
                        if length as usize != size_of::<libc::c_int>() {
                            return Err(invalid("Invalid SO_ERROR size"));
                        }
                        if status == 0 {
                            return Ok(Self { fd });
                        }
                        return Err(io::Error::from_raw_os_error(status));
                    }
                    Some(libc::EAGAIN) | Some(libc::ECONNREFUSED) | Some(libc::ENOENT) => {
                        // An AF_UNIX full backlog is EAGAIN without a pending connect.
                        // It has no useful writable-ready edge. A short bounded poll
                        // avoids spinning while waiting for bind/listen or a free slot.
                        let rc = unsafe {
                            libc::poll(std::ptr::null_mut(), 0, milliseconds(deadline)?.min(5))
                        };
                        if rc < 0 {
                            let error = io::Error::last_os_error();
                            if error.kind() != io::ErrorKind::Interrupted {
                                return Err(error);
                            }
                        }
                    }
                    _ => return Err(e),
                }
            }
        }
        pub fn peer_credentials(&self) -> io::Result<Credentials> {
            let mut cred: libc::ucred = unsafe { zeroed() };
            let mut length = size_of::<libc::ucred>() as libc::socklen_t;
            if unsafe {
                libc::getsockopt(
                    self.fd.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PEERCRED,
                    (&mut cred as *mut libc::ucred).cast(),
                    &mut length,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            if length as usize != size_of::<libc::ucred>() || cred.pid <= 0 {
                return Err(invalid("Invalid peer credentials"));
            }
            Ok(Credentials {
                pid: cred.pid,
                uid: cred.uid,
                gid: cred.gid,
            })
        }
        pub fn recv_until(&self, max_bytes: usize, deadline: Instant) -> io::Result<Packet> {
            self.recv_or_eof_until(max_bytes, deadline)
                .and_then(packet_only)
        }
        /// Classifies only this receive; a prior error remains terminal to callers.
        /// No consumer currently uses EOF to clear a session or pending Ticket.
        pub(crate) fn recv_or_eof_until(
            &self,
            max_bytes: usize,
            deadline: Instant,
        ) -> io::Result<Incoming> {
            if max_bytes == 0 || max_bytes > MAX_MESSAGE_BYTES {
                return Err(invalid("Invalid receive bound"));
            }
            loop {
                check_deadline(deadline)?;
                let mut bytes = vec![0_u8; max_bytes + 1];
                let mut control = [0_usize; 512];
                let mut iov = libc::iovec {
                    iov_base: bytes.as_mut_ptr().cast(),
                    iov_len: bytes.len(),
                };
                let mut message: libc::msghdr = unsafe { zeroed() };
                message.msg_iov = &mut iov;
                message.msg_iovlen = 1;
                message.msg_control = control.as_mut_ptr().cast();
                message.msg_controllen = size_of::<[usize; 512]>();
                let rc = unsafe {
                    libc::recvmsg(
                        self.fd.as_raw_fd(),
                        &mut message,
                        libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
                    )
                };
                if rc < 0 {
                    let e = io::Error::last_os_error();
                    match e.kind() {
                        io::ErrorKind::Interrupted => continue,
                        io::ErrorKind::WouldBlock => {
                            wait(self.fd.as_raw_fd(), libc::POLLIN, deadline)?;
                            continue;
                        }
                        _ => return Err(e),
                    }
                }
                let control_len = message.msg_controllen.min(size_of::<[usize; 512]>());
                let control_bytes = unsafe {
                    std::slice::from_raw_parts(control.as_ptr().cast::<u8>(), control_len)
                };
                return incoming_from_received(
                    bytes,
                    rc as usize,
                    max_bytes,
                    control_bytes,
                    message.msg_flags & libc::MSG_TRUNC != 0,
                    message.msg_flags & libc::MSG_CTRUNC != 0,
                    |fd| unsafe {
                        libc::close(fd);
                    },
                );
            }
        }
        pub fn send_until(&self, bytes: &[u8], deadline: Instant) -> io::Result<()> {
            if bytes.is_empty() || bytes.len() > MAX_MESSAGE_BYTES {
                return Err(invalid("Invalid message size"));
            }
            loop {
                check_deadline(deadline)?;
                let rc = unsafe {
                    libc::send(
                        self.fd.as_raw_fd(),
                        bytes.as_ptr().cast(),
                        bytes.len(),
                        libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
                    )
                };
                if rc >= 0 {
                    return if rc as usize == bytes.len() {
                        Ok(())
                    } else {
                        Err(invalid("Non-atomic packet send"))
                    };
                }
                let e = io::Error::last_os_error();
                match e.kind() {
                    io::ErrorKind::Interrupted => continue,
                    io::ErrorKind::WouldBlock => {
                        wait(self.fd.as_raw_fd(), libc::POLLOUT, deadline)?
                    }
                    _ => return Err(e),
                }
            }
        }
    }
}
#[cfg(not(target_os = "linux"))]
mod unsupported {
    use super::*;
    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "Linux sequenced-packet credentials required",
        )
    }
    impl Listener {
        pub fn bind(_nonce: &str) -> io::Result<Self> {
            Err(unsupported())
        }
        pub fn accept_until(&self, _deadline: Instant) -> io::Result<Connection> {
            Err(unsupported())
        }
    }
    impl Connection {
        pub fn expect_eof_until(&self, _deadline: Instant) -> io::Result<()> {
            Err(unsupported())
        }
        pub fn connect_until(_nonce: &str, _deadline: Instant) -> io::Result<Self> {
            Err(unsupported())
        }
        pub fn peer_credentials(&self) -> io::Result<Credentials> {
            Err(unsupported())
        }
        pub fn recv_until(&self, max_bytes: usize, deadline: Instant) -> io::Result<Packet> {
            self.recv_or_eof_until(max_bytes, deadline)
                .and_then(packet_only)
        }
        pub(crate) fn recv_or_eof_until(
            &self,
            _max_bytes: usize,
            _deadline: Instant,
        ) -> io::Result<Incoming> {
            Err(unsupported())
        }
        pub fn send_until(&self, _bytes: &[u8], _deadline: Instant) -> io::Result<()> {
            Err(unsupported())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cmsg(level: i32, kind: i32, payload: &[u8]) -> Vec<u8> {
        let word = std::mem::size_of::<usize>();
        let size = word + 8 + payload.len();
        let mut b = Vec::new();
        b.extend_from_slice(&size.to_ne_bytes());
        b.extend_from_slice(&level.to_ne_bytes());
        b.extend_from_slice(&kind.to_ne_bytes());
        b.extend_from_slice(payload);
        b.resize((size + word - 1) & !(word - 1), 0);
        b
    }
    fn good() -> Vec<u8> {
        let mut b = 12_i32.to_ne_bytes().to_vec();
        b.extend_from_slice(&123_u32.to_ne_bytes());
        b.extend_from_slice(&456_u32.to_ne_bytes());
        cmsg(1, 2, &b)
    }
    #[test]
    fn same_receive_eof_is_typed_but_legacy_api_still_rejects_it() {
        let incoming =
            incoming_from_received(vec![0; 5], 0, 4, &[], false, false, |_| panic!("no rights"))
                .unwrap();
        assert!(matches!(&incoming, Incoming::Eof));
        let error = packet_only(incoming).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "Empty, truncated or oversized packet");
    }
    #[test]
    fn same_receive_preserves_packet_boundary_and_exact_credentials() {
        let incoming =
            incoming_from_received(vec![1, 2, 3, 9, 9], 3, 4, &good(), false, false, |_| {
                panic!("no rights")
            })
            .unwrap();
        let packet = packet_only(incoming).unwrap();
        assert_eq!(packet.bytes, [1, 2, 3]);
        assert_eq!(
            packet.credentials,
            Credentials {
                pid: 12,
                uid: 123,
                gid: 456
            }
        );
        let full = incoming_from_received(
            vec![7; MAX_MESSAGE_BYTES + 1],
            MAX_MESSAGE_BYTES,
            MAX_MESSAGE_BYTES,
            &good(),
            false,
            false,
            |_| {},
        )
        .unwrap();
        assert_eq!(packet_only(full).unwrap().bytes.len(), MAX_MESSAGE_BYTES);
    }
    #[test]
    fn empty_credentialed_message_and_later_eof_are_distinct_results() {
        let first = incoming_from_received(vec![0; 5], 0, 4, &good(), false, false, |_| {});
        assert!(first.is_err());
        let later = incoming_from_received(vec![0; 5], 0, 4, &[], false, false, |_| {}).unwrap();
        assert!(matches!(later, Incoming::Eof));
        // Consumer policy must retain the first failure; this stateless classifier
        // does not prove a session latch or make the later EOF acceptable.
    }
    #[test]
    fn truncation_oversize_and_inconsistent_lengths_never_become_eof() {
        for (received, max, truncated, control_truncated) in [
            (0, 4, true, false),
            (0, 4, false, true),
            (1, 4, true, false),
            (1, 4, false, true),
            (5, 4, false, false),
            (1, 0, false, false),
            (1, MAX_MESSAGE_BYTES + 1, false, false),
        ] {
            assert!(incoming_from_received(
                vec![0; 5],
                received,
                max,
                &good(),
                truncated,
                control_truncated,
                |_| {}
            )
            .is_err());
        }
        assert!(incoming_from_received(vec![], 1, 4, &good(), false, false, |_| {}).is_err());
        for flags in [(true, false), (false, true)] {
            assert!(
                incoming_from_received(vec![0; 5], 0, 4, &[], flags.0, flags.1, |_| {}).is_err()
            );
        }
    }
    #[test]
    fn malformed_or_missing_ancillary_never_becomes_a_valid_packet() {
        for control in [
            vec![],
            vec![0; 4],
            cmsg(1, 99, &[]),
            [good(), good()].concat(),
        ] {
            assert!(
                incoming_from_received(vec![1; 5], 1, 4, &control, false, false, |_| {}).is_err()
            );
        }
    }
    #[test]
    fn delivered_rights_are_closed_even_for_empty_or_truncated_receives() {
        let fds = [9_i32.to_ne_bytes(), 10_i32.to_ne_bytes()].concat();
        let control = [cmsg(1, 99, &[]), cmsg(1, 1, &fds), good()].concat();
        for (received, truncated, control_truncated) in [
            (0, false, false),
            (0, true, false),
            (1, false, true),
            (5, false, false),
        ] {
            let mut closed = Vec::new();
            assert!(incoming_from_received(
                vec![0; 5],
                received,
                4,
                &control,
                truncated,
                control_truncated,
                |fd| closed.push(fd)
            )
            .is_err());
            assert_eq!(closed, [9, 10]);
        }
        let mut truncated_rights = cmsg(1, 1, &fds);
        truncated_rights.truncate(std::mem::size_of::<usize>() + 8 + 4);
        let mut closed = Vec::new();
        assert!(
            incoming_from_received(vec![0; 5], 0, 4, &truncated_rights, false, true, |fd| {
                closed.push(fd)
            })
            .is_err()
        );
        assert_eq!(closed, [9]);
    }
    #[test]
    fn abstract_name_is_fixed_and_nonce_cannot_select_path() {
        let nonce = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            abstract_name(nonce).unwrap(),
            format!("luma-home.artwork.proof.{nonce}").as_bytes()
        );
        for bad in [
            "",
            "ABCDEF0123456789abcdef0123456789ab",
            "0123456789abcdef0123456789abcde/",
            "../socket",
        ] {
            assert!(abstract_name(bad).is_err());
        }
    }
    #[test]
    fn exactly_one_well_formed_credential_is_required() {
        assert_eq!(
            credentials_from_control(&good(), |_| panic!("no rights")).unwrap(),
            Credentials {
                pid: 12,
                uid: 123,
                gid: 456
            }
        );
        assert!(credentials_from_control(&[], |_| {}).is_err());
        let twice = [good(), good()].concat();
        assert!(credentials_from_control(&twice, |_| {}).is_err());
        for malformed in [
            vec![0; 4],
            cmsg(1, 2, &[0; 11]),
            cmsg(1, 2, &[0; 12]),
            cmsg(1, 99, &[0; 12]),
            cmsg(99, 2, &[0; 12]),
        ] {
            assert!(credentials_from_control(&malformed, |_| {}).is_err());
        }
    }
    #[test]
    fn rights_are_closed_even_when_other_ancillary_is_invalid() {
        let mut received = Vec::new();
        let mut fds = 9_i32.to_ne_bytes().to_vec();
        fds.extend_from_slice(&10_i32.to_ne_bytes());
        let bytes = [cmsg(1, 99, &[]), cmsg(1, 1, &fds), good()].concat();
        assert!(credentials_from_control(&bytes, |fd| received.push(fd)).is_err());
        assert_eq!(received, [9, 10]);
        // Truncated rights still close every delivered full descriptor.
        let mut bytes = cmsg(1, 1, &fds);
        bytes.truncate(std::mem::size_of::<usize>() + 8 + 4);
        received.clear();
        assert!(credentials_from_control(&bytes, |fd| received.push(fd)).is_err());
        assert_eq!(received, [9]);
    }
    #[test]
    fn ancillary_lengths_cannot_escape_control_buffer() {
        for size in [0, 1, usize::MAX] {
            let mut b = good();
            b[..std::mem::size_of::<usize>()].copy_from_slice(&size.to_ne_bytes());
            assert!(credentials_from_control(&b, |_| {}).is_err());
        }
        let mut b = good();
        b.push(1);
        assert!(credentials_from_control(&b, |_| {}).is_err());
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_does_not_fall_back_to_stream_or_filesystem() {
        assert_eq!(
            Listener::bind("0123456789abcdef0123456789abcdef")
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            Connection::connect_until("0123456789abcdef0123456789abcdef", Instant::now())
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn typed_receive_has_no_unsupported_platform_fallback() {
        let connection = Connection {};
        assert_eq!(
            connection
                .recv_or_eof_until(4, Instant::now())
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            connection.recv_until(4, Instant::now()).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
    #[cfg(target_os = "linux")]
    mod actual {
        use super::*;
        use std::os::fd::AsRawFd;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Duration;
        fn nonce() -> String {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            format!(
                "{:016x}{:016x}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )
        }
        fn deadline() -> Instant {
            Instant::now() + Duration::from_secs(2)
        }
        fn pair() -> (Connection, Connection) {
            let n = nonce();
            let listener = Listener::bind(&n).unwrap();
            let client = Connection::connect_until(&n, deadline()).unwrap();
            let server = listener.accept_until(deadline()).unwrap();
            (client, server)
        }
        #[test]
        fn actual_typed_receive_and_legacy_eof_behavior_are_separate() {
            let (client, server) = pair();
            server.send_until(b"typed", deadline()).unwrap();
            drop(server);
            let Incoming::Packet(packet) = client.recv_or_eof_until(4096, deadline()).unwrap()
            else {
                panic!("packet required")
            };
            assert_eq!(packet.bytes, b"typed");
            assert_eq!(
                packet.credentials,
                Credentials {
                    pid: std::process::id() as i32,
                    uid: unsafe { libc::geteuid() },
                    gid: unsafe { libc::getegid() }
                }
            );
            assert!(matches!(
                client.recv_or_eof_until(4096, deadline()).unwrap(),
                Incoming::Eof
            ));
            let (client, server) = pair();
            drop(server);
            let error = client.recv_until(4096, deadline()).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(error.to_string(), "Empty, truncated or oversized packet");
        }
        #[test]
        fn actual_zero_byte_credentialed_packet_is_not_clean_eof() {
            let (client, server) = pair();
            assert_eq!(
                unsafe {
                    libc::send(
                        server.fd.as_raw_fd(),
                        std::ptr::null(),
                        0,
                        libc::MSG_NOSIGNAL,
                    )
                },
                0
            );
            drop(server);
            assert!(client.recv_or_eof_until(4096, deadline()).is_err());
        }
        #[test]
        fn actual_eof_can_be_only_a_peer_write_half_close() {
            let (client, server) = pair();
            assert_eq!(
                unsafe { libc::shutdown(server.fd.as_raw_fd(), libc::SHUT_WR) },
                0
            );
            assert!(matches!(
                client.recv_or_eof_until(4096, deadline()).unwrap(),
                Incoming::Eof
            ));
            // The peer descriptor and its receive side are still alive. Typed EOF
            // must never independently qualify helper exit or child cleanup.
            assert_ne!(
                unsafe { libc::fcntl(server.fd.as_raw_fd(), libc::F_GETFD) },
                -1
            );
            client
                .send_until(b"peer-still-receives", deadline())
                .unwrap();
            assert_eq!(
                server.recv_until(4096, deadline()).unwrap().bytes,
                b"peer-still-receives"
            );
        }
        #[test]
        fn eof_after_finished_rejects_empty_or_extra_messages() {
            let (client, server) = pair();
            drop(server);
            assert!(client.expect_eof_until(deadline()).is_ok());
            let (client, server) = pair();
            server.send_until(b"extra", deadline()).unwrap();
            drop(server);
            assert!(client.expect_eof_until(deadline()).is_err());
            let (client, server) = pair();
            assert_eq!(
                unsafe {
                    libc::send(
                        server.fd.as_raw_fd(),
                        std::ptr::null(),
                        0,
                        libc::MSG_NOSIGNAL,
                    )
                },
                0
            );
            drop(server);
            assert!(client.expect_eof_until(deadline()).is_err());
            let (client, _server) = pair();
            assert_eq!(
                client
                    .expect_eof_until(Instant::now() + Duration::from_millis(10))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
        }
        #[test]
        fn actual_packets_preserve_boundaries_and_kernel_credentials() {
            let (c, s) = pair();
            let expected = Credentials {
                pid: std::process::id() as i32,
                uid: unsafe { libc::geteuid() },
                gid: unsafe { libc::getegid() },
            };
            assert_eq!(c.peer_credentials().unwrap(), expected);
            assert_eq!(s.peer_credentials().unwrap(), expected);
            c.send_until(b"one", deadline()).unwrap();
            c.send_until(b"two", deadline()).unwrap();
            let a = s.recv_until(4096, deadline()).unwrap();
            let b = s.recv_until(4096, deadline()).unwrap();
            assert_eq!(a.bytes, b"one");
            assert_eq!(b.bytes, b"two");
            assert_eq!(a.credentials, expected);
            assert_eq!(b.credentials, expected);
            s.send_until(b"reply", deadline()).unwrap();
            assert_eq!(c.recv_until(4096, deadline()).unwrap().bytes, b"reply");
            for fd in [&c.fd, &s.fd] {
                assert_ne!(
                    unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
                    0
                );
                assert_ne!(
                    unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                    0
                );
            }
        }
        #[test]
        fn actual_deadlines_and_invalid_lengths_fail_closed() {
            let n = nonce();
            let l = Listener::bind(&n).unwrap();
            assert_eq!(
                l.accept_until(Instant::now() + Duration::from_millis(10))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
            assert_eq!(
                Connection::connect_until(&nonce(), Instant::now() + Duration::from_millis(10))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
            let (c, s) = pair();
            assert_eq!(
                s.recv_until(4096, Instant::now() + Duration::from_millis(10))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
            assert!(c.send_until(&[], deadline()).is_err());
            assert!(c.send_until(&vec![0; 4097], deadline()).is_err());
            assert!(s.recv_until(4097, deadline()).is_err());
            assert!(s.recv_until(0, deadline()).is_err());
            c.send_until(b"12345", deadline()).unwrap();
            assert!(s.recv_until(4, deadline()).is_err());
        }
        #[test]
        fn actual_oversized_empty_and_peer_close_packets_are_rejected() {
            let (c, s) = pair();
            let bytes = [0_u8; 8192];
            assert_eq!(
                unsafe {
                    libc::send(
                        c.fd.as_raw_fd(),
                        bytes.as_ptr().cast(),
                        bytes.len(),
                        libc::MSG_NOSIGNAL,
                    )
                },
                8192
            );
            assert!(s.recv_until(4096, deadline()).is_err());
            let (c, s) = pair();
            assert_eq!(
                unsafe { libc::send(c.fd.as_raw_fd(), std::ptr::null(), 0, libc::MSG_NOSIGNAL) },
                0
            );
            assert!(s.recv_until(4096, deadline()).is_err());
            drop(c);
            assert!(s.recv_until(4096, deadline()).is_err());
            assert!(s.send_until(b"x", deadline()).is_err()); // MSG_NOSIGNAL prevents process termination.
        }
        #[test]
        fn actual_rights_are_rejected_and_received_descriptors_closed() {
            let (c, s) = pair();
            let mut ends = [-1; 2];
            assert_eq!(
                unsafe { libc::pipe2(ends.as_mut_ptr(), libc::O_CLOEXEC) },
                0
            );
            use std::os::fd::{FromRawFd, OwnedFd};
            let read = unsafe { OwnedFd::from_raw_fd(ends[0]) };
            let _write = unsafe { OwnedFd::from_raw_fd(ends[1]) };
            let target = std::fs::read_link(format!("/proc/self/fd/{}", read.as_raw_fd())).unwrap();
            let references = || {
                std::fs::read_dir("/proc/self/fd")
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter(|entry| std::fs::read_link(entry.path()).ok().as_ref() == Some(&target))
                    .count()
            };
            let before = references();
            let control = cmsg(1, 1, &read.as_raw_fd().to_ne_bytes());
            let mut aligned = [0_usize; 32];
            let raw = unsafe {
                std::slice::from_raw_parts_mut(
                    aligned.as_mut_ptr().cast::<u8>(),
                    std::mem::size_of_val(&aligned),
                )
            };
            raw[..control.len()].copy_from_slice(&control);
            let mut byte = b'x';
            let mut iov = libc::iovec {
                iov_base: (&mut byte as *mut u8).cast(),
                iov_len: 1,
            };
            let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = aligned.as_mut_ptr().cast();
            msg.msg_controllen = control.len();
            assert_eq!(
                unsafe { libc::sendmsg(c.fd.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) },
                1
            );
            assert!(s.recv_until(4096, deadline()).is_err());
            // This pipe inode belongs only to this test. Other parallel test FDs
            // do not affect its reference count; the delivered duplicate is closed.
            assert_eq!(references(), before);
        }
    }
}
