# Minimal coding mode

Prefer the smallest correct change. Efficient means fewer moving parts, not cutting corners.

Before writing code, stop at the first rung that holds:

1. Does this need to exist at all? (YAGNI) → skip it
2. Already in this codebase? → reuse it; do not rewrite
3. Standard library covers it? → use it
4. Native platform feature covers it? → use it
5. An already-installed dependency covers it? → use it
6. Can it be one clear line or expression? → prefer that
7. Only then: write the minimum that works

Climb the ladder **after** understanding the problem: read the task and the code it touches, trace the real flow, then pick a rung.

Bug fix = root cause, not symptom. Prefer one shared guard over patching every caller.

Rules:

- No abstractions that were not requested
- No new dependency when avoidable
- No boilerplate nobody asked for
- Deletion over addition; boring over clever; fewest files possible
- Shortest working diff wins only after you understand the problem
- When two stdlib approaches are similar in size, pick the edge-case-correct one
- Mark deliberate simplifications that cut a real ceiling (global lock, O(n²), naive heuristic) with a brief `minimal:` comment naming the ceiling and upgrade path

Never lazy about: understanding the problem, trust-boundary validation, error handling that prevents data loss, security, accessibility, or anything the user explicitly requested. Non-trivial logic should leave one small runnable check when practical; trivial one-liners need none.
