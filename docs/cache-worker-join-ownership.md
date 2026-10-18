# Cache ownership after image-worker join

The image worker previously dropped its ordinary durable cache on exit. A
future maintenance supervisor needs that same held cache, rather than a new
cache opened by pathname after shutdown. Queue drain, ordered adoption and a
shutdown receipt do not establish that the worker has joined.

The new private opt-in path ends controller consumption and returns an unjoined
handle. It withdraws the active generation, abandons pending work and deletes
owned textures through the controller backend. A successful actual thread join
can then return the worker's original cache in a private, non-cloneable ownership
wrapper. A panic, failed loader, package-only loader or reused handle supplies
no cache. The generic loader and controller/backend do not cross to the joining
supervisor thread.

Normal `finish()` keeps its existing behaviour: it requests no handoff and the
cache drops on the image worker. No product maintenance caller is enabled.
The change does not add eviction, retirement, stale-slot recovery, publication,
retention-policy exceptions or Root authority.

The 37 focused image-worker tests and strict all-target Clippy pass. Five new
host cases exercise a real held cache and blocked decode, same RootArc identity
versus a reopened cache, default destruction, panic/factory failures, late-result
withdrawal and controller-backend texture deletion before supervisor join.
These use host fixtures and a checking backend; they do not prove native GL,
TV permissions or release-mode panic recovery. The unchanged release profile
uses aborting panics.

This is a source prerequisite for the separate maintenance planner. A future
transaction still needs complete retention policy, fresh logical-path and
namespace checks, exclusive writer-slot handling and final record validation.
Other processes and readers remain outside this worker's ownership proof.
Matching ARM builds, packages, runtime integration and native validation are
pending; compiled performance artifacts and retained TV installations are
unchanged.
