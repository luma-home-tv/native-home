# Public icon reader foundation

[Draft PR9](https://github.com/luma-home-tv/native-home/pull/9), source
`e7017b36ca93af2699a53222237e5ead70329daa`, adds a standalone file-reading
primitive. It does not change the dashboard, framework, snapshot collector or
either retained installation.

The legacy shell snapshot copy checks paths before reopening them. A real host
fixture demonstrated that a changed directory can redirect that read. The new
C CLI keeps every directory open from literal `/` through the selected public
application root, app directory and relative path. It uses no-follow opens,
Root ownership, public search/read permission bits and held/named metadata
checks before emitting the fully buffered body. Public `0777` objects are
permitted; private directories/files, links and special permission bits refuse.
Its three roots and path/body limits are fixed. The companion Python module
checks complete response framing and the explicit requested key.

Strict host C11 and ASan/UBSan each pass seven test groups. Independent source
review and six additional host test groups pass, followed by a fresh coordinator
replay. Additional cases exercise transient directory/file modes, directory
replacement, hard links, quoted filenames, malformed packets and blocked
output with an inherited blocked alarm mask. The blocked-output process exits
124 with a partial frame, which remains unsuccessful.

The actual SDK build uses ARMv7 soft-float, PIE and 64-bit file offsets. Its
13,604-byte ELF has SHA-256
`853f083fcc0c61df5d34e80195bd7ac7bd87bfaccf0c9c9271002c69e840cef0`,
interpreter `/lib/ld-linux.so.3`, one direct dependency (`libc.so.6`) and no
RPATH. The frozen C source is
`1872697ed74b5bf3435fff7cf84de15831eea3baa5031ffb99ab958167844dc4`;
68 compiler/header inputs and the 46 host evidence bindings match before and
after the build. This is local compilation and ELF inspection, not independent
recompilation, transitive loading or target execution proof.

The reader checks signatures rather than decoding images. Its sampled metadata
checks do not prove continuous or post-output immutability, catalogue/version
freshness or ordinary native access. The five-second alarm and byte bounds do
not guarantee total memory or uninterruptible kernel I/O latency. A response
alone grants no Root authority; a future controller must verify the exact
executable, request, actual child result, complete framing and known closure.

No TV contact or deployment occurred. Normal Root artwork remains off. The next
scope is a separately reviewed headless read of one known packaged icon, with a
fresh device baseline, lease, private staging and complete removal. Broader icon
collection, cache publication and normal rendering remain separate work.
