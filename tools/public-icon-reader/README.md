# Public app icon reader (not deployed)

`reader.c` is a standalone, one-request Root-admin CLI. It is not a service,
setuid tool, ordinary-app fallback or cache writer. Neither the snapshot collector
nor a TV transport calls it. Deployment, helper/ELF identity, staging, device lease,
request eligibility, namespace qualification and complete removal need a separate
reviewed controller. The unsafe legacy shell icon read in `collect-snapshot.py`
is not an alternative to this primitive and must not be enabled in that controller.

The production command takes exactly three arguments:

```
public-icon-reader developer_apps com.example.app images/icon.png
```

Closed root kinds are `lg_system` (`/usr/palm/applications`), `encrypted_apps`
(`/media/cryptofs/apps/usr/palm/applications`) and `developer_apps`
(`/media/developer/apps/usr/palm/applications`). The existing ordinary `src/app_artwork.rs` reader and pinned framework remain
unchanged; this admin-only boundary does not activate privileged normal artwork.

No root path, UID, timeout,
service mode or test-root option exists. Real/effective UID and GID must all be 0.
App IDs are 1–180 ASCII characters, start alphanumeric and otherwise contain only
alphanumerics, dot, underscore or hyphen. Relative filenames are at most 512
printable ASCII bytes, at most 8 components of 1–128 bytes, and end in PNG/JPG/JPEG
(case insensitive). Empty, dot, dot-dot, absolute, backslash, control and URI
segments refuse. Non-ASCII filenames are currently unavailable.

The walk starts at a held literal `/` FD. Every root, app and relative directory
is opened through its held parent using `openat(O_NOFOLLOW|O_DIRECTORY)`;
all FDs remain open until response emission. Each directory must be Root:Root,
plain mode with no special bits and have `S_IXOTH`. The final object must be a
Root:Root regular, single-link file, plain mode with `S_IROTH`, and 1–1,048,576
bytes. Public 0777 objects are permitted. Private 0700 directories and 0600 files
refuse even under a writable public parent. These are DAC-bit requirements;
no ACL or ordinary-native effective-access assertion follows.

The complete body is read into one bounded allocation, followed by an EOF check.
Every held and named directory, and the final held/named file, must still match
the original device/inode/type/mode/owner/links/size/mtime/ctime nanosecond sample
before any stdout is written. PNG/JPEG signatures are checked; the reader does
not decode images or prove valid dimensions/pixels. It does not accept arbitrary
paths, follow symlinks, read account files through links, call a shell, modify
permissions, write a TV file, contact a network endpoint or start another process.

## Response and refusal

Stdout is exactly:

1. Four ASCII bytes `LPI1`.
2. A four-byte unsigned big-endian metadata length (1–4095).
3. Exactly that many UTF-8 JSON bytes, without a separator.
4. Exactly `bytes` raw body bytes, then EOF.

Metadata has exactly these fields: `schema` (integer 1), `root_kind`, `app_id`,
`relative`, `bytes`, `device`, `inode`, `uid`, `gid`, `mode`, `nlink`,
`mtime_seconds`, `mtime_nanos`, `ctime_seconds`, `ctime_nanos`. Mode includes the
regular-file type bits. The host computes SHA-256; no crypto dependency is added.
`protocol.py` validates complete framing, exact requested key, typed metadata,
public file policy, signature and body length. It performs no transport or
execution and cannot establish helper identity or Root authority.

Validation refusals exit 1 with **zero stdout bytes** and a fixed stderr message.
A single original process-wide 5-second `SIGALRM` begins before argument/path I/O,
is never renewed, explicitly unblocks an inherited alarm mask, and exits 124. Reads, writes and polls do not retry `EINTR`.
An output failure or timeout during emission can leave a partial frame: the host
must reject nonzero status, incomplete data or any trailing bytes. A later native
controller must also bound the host child and qualify its known closure.

Byte limits bound the body allocation and path/metadata processing; at most 24
directory FDs are held. Repeated short reads remain within the single original
alarm. This is not a total-memory, storage-latency or uninterruptible-kernel-I/O
guarantee. Sampled identity agreement is not continuous observation, defence
against hostile Root, catalogue/source-version freshness or proof that a file
cannot change after the final sample. No current build, TV namespace, transitive
loading, artwork hit, permission or rendering qualification follows from host tests.

## Host verification

Use an explicitly fresh private build directory outside the source checkout:

```
python3 -B tools/public-icon-reader/test_public_icon_reader.py --build-dir "$PRIVATE_BUILD_DIR"
```

`--sanitize` also enables host AddressSanitizer and UndefinedBehaviorSanitizer.
Strict C11 warnings are errors. `test_reader.c` includes the same core with
compile-time-only fixture hooks and a test owner parameter; the production CLI
is absent from that translation unit. There is no production test flag or
environment-variable bypass. Real filesystem tests cover symlink/parent/root
replacement, same-body new inode, growth/rewrite, hard links, FIFOs, private modes,
wrong ownership policy, size limits, malformed keys and timeout before emission.
All test images/IDs are synthetic. No SDK, ARM compiler or TV is used.
