# Issue triage

The neoOMSI issue tracker serves as an **active engineering backlog**, not an unmanaged archive of ideas or unresolved reports.

An open issue represents an actionable defect, verified bug, or planned task actively being addressed or scheduled by maintainers.

## Triage philosophy

High-volume repositories quickly become overwhelmed by duplicate, incomplete, or untriaged reports. Maintainers prioritize engineering time on actionable, reproducible issues.

Closing an issue does not mean the report was invalid; it simply means there is currently insufficient reproducible information or priority to warrant active tracking. Issues can be reopened or resubmitted once reproduction steps or logs become available.

## Label conventions

### Status labels

- `status: untriaged` — New issue, awaiting review.
- `status: needs-info` — Plausible report, but missing steps, logs, or reproduction details.
- `status: verified` — Successfully reproduced or substantiated by clear diagnostic evidence.
- `status: needs-investigation` — Valid problem, but the root cause or correct subsystem fix is unclear.

### Scope labels

- `scope: parity` — Discrepancy with OMSI 2.2.032 behavior.
- `scope: extension` — Feature or enhancement intentionally going beyond OMSI 2.
- `scope: internal` — Build infrastructure, CI, tooling, or refactoring.
- `scope: mod-specific` — Behavior isolated to a third-party add-on.

### Protection label

- `triage: protected` — Explicitly retained through automated maintenance sweeps during ongoing research or design.

## Triage workflow

Maintainers review incoming issues regularly:

1. **Check for completeness:** Does the report specify the neoOMSI build, OMSI 2 path configuration, reproduction steps, and system details?
2. **OMSI 2 baseline comparison:** What does OMSI 2.2.032 do under the exact same inputs and conditions?
3. **Classify:**
   - **Actionable & verified:** Assign `status: verified` and appropriate scope.
   - **Missing information:** Request specifics and tag `status: needs-info`.
   - **Duplicate:** Reference the canonical issue and close.
   - **Out of scope / pure extension:** Label `scope: extension` and defer or close if not aligned with current phase goals.
   - **Incomplete / non-reproducible:** Close with an explanation of what details are required to re-evaluate.

## Mod-specific reports

A bug occurring only on a specific add-on map or vehicle requires this question:

> Does this content expose a general OMSI rule that neoOMSI implements incorrectly?

- If the issue is due to a misconfigured script or syntax error that also fails in OMSI 2, it is outside project scope.
- If OMSI 2 accommodates the mod via a documented or discoverable fallback, document the general rule and label `scope: parity`.

## Feature requests

During the parity-focused phase, features that deliberately diverge from or extend OMSI 2 are secondary to core engine parity. Feature requests may be tagged `scope: extension` and closed or held as discussions to keep the issue tracker focused on engine stabilization.
