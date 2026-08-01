# Supervised worker-model routing evidence

This project routes Orca workers by the repository `AGENTS.md` model table and the user's explicit
instruction. Public model material is supporting context only; it does not override the local cost,
intelligence, taste, or availability rankings.

## Routing contract

| Work shape | Preferred worker | Required coordinator treatment |
|---|---|---|
| Merge-sensitive roots, causal state machines, cross-domain invariants, and difficult debugging | GPT-5.6 Sol | Give one bounded ownership surface and exact acceptance invariants; reject a result that only adds types without closing physical/runtime behavior. |
| Bounded feature leaves with clear dependencies and limited root risk | GPT-5.6 Terra | Keep the file boundary narrow; escalate to Sol when source review finds missed conservation, persistence, or ordering behavior. |
| Catalog inventories, evidence gathering, deterministic data/test matrices, and documentation | Luna when available | Require exact counts, trace rows, and source references; do not let a summary replace an authoritative plan. |
| Clear-spec bulk implementation, mechanical migrations, and repetitive data work | GPT-5.5 | Provide a self-contained issue-like specification; coordinator checks semantics before accepting mechanical completeness. |
| Player-facing UI, art direction, copy, accessibility, and taste review | Opus 4.8 | Supply the current shipped visual references and interaction constraints; implementation still requires browser and accessibility evidence. |

At most three disjoint workers plus the coordinator may be active. Workers do not run Cargo,
formatters, browsers, or image generation. The coordinator owns the single serialized verification
slot and may redo work with a stronger model without asking when a result misses the specification.

## Public-source findings

- OpenAI describes GPT-5.5 as a strong agentic coding model for implementation, refactoring,
  debugging, testing, and long-horizon tool use. That supports using it for clear, self-contained
  engineering tasks, while the local `AGENTS.md` cost/taste table determines its exact project role:
  <https://openai.com/index/introducing-gpt-5-5/>.
- OpenAI's Codex workflow guidance recommends well-scoped, issue-like tasks, persistent repository
  context, and iterative environment improvement. That is why every dispatch names exact files,
  dependencies, invariants, prohibited commands, and completion evidence:
  <https://openai.com/business/guides-and-resources/how-openai-uses-codex/>.
- OpenAI's agent guidance supports delegating well-scoped tasks to multiple agents, but it does not
  replace this project's dependency order, file-ownership limits, or single-heavy-process rule:
  <https://openai.com/index/introducing-codex/>.
- Anthropic presents Opus 4.8 as a long-running coding and agent model. The project nevertheless
  reserves it primarily for user-facing visual, interaction, accessibility, and taste work because
  the local `AGENTS.md` assigns it the strongest available taste score:
  <https://www.anthropic.com/claude/opus>.

No authoritative public documentation for Orca's project-local **Sol**, **Terra**, or **Luna**
routing labels was found. Their meanings therefore come only from the user's instruction, the
available Orca model identifiers, and the repository `AGENTS.md`. Community comparisons are not an
acceptance authority.

## Failure and escalation rule

A worker's `worker_done` message is a review request, not card closure. The coordinator must inspect
the actual source and reject or reassign work when any required behavior is represented only by a
comment, a free-standing helper, a lossy identity conversion, an unbounded collection, an
unpersisted state transition, or a deferred “downstream integration” that belongs to the current
card. Examples already caught by this rule include Hole upgrade cargo disappearing at completion
and material/fixture removal fabricating a different physical identity or quality.

Model choice never relaxes determinism, report secrecy, physical conservation, strict persistence,
the line-1 board order, or the ban on live-AI automated tests.
