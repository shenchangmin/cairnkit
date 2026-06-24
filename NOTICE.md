# Third-party notices & attribution

cairnkit is an original work (MIT). It builds on, ports from, or borrows ideas from the
following open-source projects. We are grateful to their authors.

## Code / structure derived (MIT — attribution retained)

| Project | License | What cairnkit uses |
|---|---|---|
| [aj-geddes/claude-code-bmad-skills](https://github.com/aj-geddes/claude-code-bmad-skills) | MIT | Claude Code **plugin skeleton & file format** (`plugin.json` / `marketplace.json`, `skills/<n>/SKILL.md`, `agents/*.md`, `hooks.json`, `${CLAUDE_PLUGIN_ROOT}` discipline) |
| [bmad-code-org/BMAD-METHOD](https://github.com/bmad-code-org/BMAD-METHOD) | MIT | **Status-frontmatter state-machine pattern** and bounded review/repair loop; role-persona structure |
| [rihebty/flow-kit](https://github.com/rihebty/flow-kit) | MIT | **Stage-gate / Artifact Preflight Gate** pattern, tiered token budget, anti-repeat lessons protocol, document templates |

> Where files are directly derived from the above, the original MIT copyright notice is
> retained in-file. Net-new cairnkit files carry the cairnkit copyright.

## Ideas / concepts borrowed (no code copied)

| Source | What we borrowed (concept only) |
|---|---|
| [MemTensor/MemOS](https://github.com/MemTensor/MemOS) | Maturity-by-execution-feedback taxonomy, strict "is-this-worth-saving" extraction gate, half-life decay formula, provenance/versioned-history modeling — **re-implemented from scratch** as Markdown+Git; none of MemOS's DB/embedding/service code is used |
| *LLM Wiki* pattern (Andrej Karpathy) | The Ingest / Query / **Lint** lifecycle for a knowledge base |

Ideas and architectures are not copyrightable; the above is credited as good practice and
to make cairnkit's lineage transparent.
