# Public icon reader foundation

[Draft PR9](https://github.com/luma-home-tv/native-home/pull/9), functional source
`092a3047e18add8a303e7130a8d3e7e043f0753d`, adds a standalone file-reading
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

Strict host C11 and ASan/UBSan each pass eight test groups. Independent source
review and six additional host test groups pass, followed by a fresh coordinator
replay. Additional cases exercise transient directory/file modes, directory
replacement, hard links, quoted filenames, malformed packets and blocked
output with an inherited blocked alarm mask. The blocked-output process exits
124 with a partial frame, which remains unsuccessful. The PID follow-up checks
the actual child PID, failed/short/interrupted lifecycle writes and default
SIGPIPE. The signal distinction is corrected in the published README: a closed
pipe can terminate the process rather than return the ordinary error status.

One bounded stderr write reports the actual reader PID after Root checks and
before path I/O. A failed or short write never enters the path walk. The PID
line alone provides neither Root authority nor closure: a future controller
must bind the call, exact executable, actual terminal result and literal PID
absence. A wrapper PID cannot substitute for reader identity. No live raw7 or
namespace observation follows from the host test.

The actual SDK build uses ARMv7 soft-float, PIE and 64-bit file offsets. Its
17,832-byte ELF has SHA-256
`a8ec073d2c401d18924fbbc02a19e788c4e090f444a36b304f7ade8a5bf2f44f`,
interpreter `/lib/ld-linux.so.3`, one direct dependency (`libc.so.6`) and no
RPATH. The frozen C source is
`96c28e30bce3b17e9b7f3b2641b56090a5338d0414df1ae652c5ab41eb68371d`;
68 compiler/header inputs and the 44 host evidence bindings match before and
after the build. This is local compilation and ELF inspection, not independent
recompilation, transitive loading or target execution proof.
The earlier `e7017b3`/`1872697e` source and `853f083f` ELF remain preserved as
historical evidence. Future PID-based qualification uses the new artifact.

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
