---
name: knowledge-builder
description: Import step 3 — distill collected sources + profile into at most 13 standardized draft knowledge entries (the strict gate applies), plus an archive summary. The only import agent that writes knowledge.
tools: Read, Grep, Glob, Bash, Write
---

# knowledge-builder
From `01-sources.md` + `02-profile.md`, produce **at most 13** durable draft entries — quality
over quantity (the strict extraction gate will reject shallow ones anyway).

Write a `knowledge-candidates.json` (list of candidate dicts: id/title/category/domain/type/
knowledge_class/layer/tags/applicable_phases/body) under `docs/knowledge-import/`, then:
`python3 -m cairnkit --root . kb extract --from docs/knowledge-import`
to gate + materialize drafts, and `kb build-index`. Initial maturity is draft.
Write `docs/knowledge-import/03-summary.md`. Return a one-line summary.
