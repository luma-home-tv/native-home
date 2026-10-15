# Render during Preview preflight, 0.46 candidate

The [0.45 actual-Preview experiment](preview-artwork-admission-045.md) stopped
before any HBC request or resource dispatch and restored normal 0.37 exactly.
Its five-second context gate expired. The last saved replies were deadline
placeholders, so that evidence cannot establish a permission denial or the
underlying context failure.

The candidate now opens and presents Home before starting the inert collector,
then pumps rendering and events during the existing eight-second hold until the
worker finishes. Joining happens afterwards, matching the established Proof
capability recipe. An observed interruption suppresses further drawing and
resource-worker creation; final event/request drains cover the hold boundary.
Capturing the existing buffer does not reactivate drawing after interruption.
The SDK setup/destruction retain their existing soft lifetime boundary.

One first context sample is retained for diagnosis without extra service calls.
Current before/after samples remain the authority: the new sample cannot admit
a stale context. The existing five-second foreground and eight-second operation
gates, fixed service calls, profiles, marker, pending guard and resource policy
are unchanged. This addresses a source ordering defect; native evidence must
still determine whether it resolves the observed failure.

Host checks pass 443 library tests, five dashboard tests, warnings-denied Clippy
and 57 Python checks. The new context regression preserves an early concrete
failure when later replies become deadline placeholders, with zero HBC calls
and no positive capability. ARM compilation and the next backed-up actual-ID
experiment remain pending. Ordinary interactive artwork activation remains off.

The [transition lifecycle design](artwork-transition-lifecycle.md) remains a
separate production gate. C2's [retained upgrade plan](c2-retained-upgrade-plan.md)
likewise needs its own state/metadata and normal rendering evidence. No result
here changes LG Home, firmware, jail policy, media services or OLED protection.
The [PlxNative framework attribution](../README.md#plxnative-framework-dependency)
continues to apply.
