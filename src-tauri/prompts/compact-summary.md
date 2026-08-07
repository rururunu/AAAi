Your task is to create a detailed summary of the conversation so far, so a coding agent can resume the work without losing context. The agent keeps your summary alongside the user's own turns (kept verbatim) and the most recent tail of the conversation; your job is to fold the assistant/tool work in between into a briefing it can act on directly.

Before writing the summary, work through the conversation chronologically in an `<analysis>` block: for each stretch of messages, identify the user's explicit requests, the approach taken, key decisions and their rationale, specific file/code details (paths, signatures, line locations, exact edits), errors hit and how they were resolved, and any place the user corrected your direction. Double-check the analysis for completeness before writing the summary — it is disposable scratch work that will not reach the agent's context, so use it to get the facts right rather than to sound polished.

Then write the summary itself inside a `<summary>` block, using exactly these headings, and omitting a heading only if it truly has no content:

## Standing facts & constraints
Everything the user stated that still governs the work — names, paths, IDs, versions, tokens, preferences, and hard "never do X" rules — in their own words. Be exhaustive; this is the durable contract, so prefer over- to under-including.

## Goal
The user's request and intent, stated precisely enough that the agent would not need to re-ask what "done" means.

## Decisions & rationale
Key choices made so far and why, so they are not re-litigated or silently reversed later.

## Files & code
Files read or modified, with the specific facts that matter: signatures, line locations, data shapes, and the exact edits applied. Be concrete — this is what lets the agent act without re-reading everything from scratch.

## Commands & outcomes
Commands run (builds, tests, git, generators) and their relevant results — what passed, what failed, and the exact error text that matters for diagnosing it.

## Errors & fixes
Problems hit and how they were resolved (or not), so the same dead ends are not repeated. Pay special attention to any case where the user said "not like that" or corrected an earlier approach.

## Pending & next step
What is still in progress or unstarted, and the single most concrete next action to take. If there is a next step, quote the most recent relevant instruction verbatim so the resumed task does not drift from what was actually asked.

Here is the shape your output should take:

<example>
<analysis>
[Chronological pass over the conversation: what was asked, what was tried, what changed, what broke and how it was fixed, and any place the user redirected the approach.]
</analysis>

<summary>
## Standing facts & constraints
- [Fact 1, in the user's own terms]
- [Fact 2]

## Goal
[Precise restatement of what the user wants]

## Decisions & rationale
- [Decision 1 — why]

## Files & code
- `path/to/file.ts`
  - [why it matters]
  - [exact edit or signature, if load-bearing]

## Commands & outcomes
- `command run` → [result / error text]

## Errors & fixes
- [Error] → [fix, or "unresolved" if still open]

## Pending & next step
- [What remains]
- Next: [concrete action, quoting the user's own words for the last instruction if one exists]
</summary>
</example>

Rules: be terse inside each section — bullet points and fragments, not prose paragraphs. Preserve identifiers, paths, and numbers exactly as they appeared; do not round, rename, or paraphrase them. Do not invent anything not present in the messages — if something is genuinely unknown, omit it rather than guessing, since a plausible-sounding guess is worse than a gap the agent can ask about.

If additional summarization instructions appear elsewhere in context (for example a `## Compact Instructions` block), follow them in addition to the structure above rather than instead of it.
