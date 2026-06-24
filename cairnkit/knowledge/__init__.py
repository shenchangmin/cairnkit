"""The knowledge layer — cairnkit's moat (modules M4–M7).

M4 model/schema (this batch), M5 index/query, M6 lifecycle, M7 cross-project Git repo.
Everything here is deterministic and pytest-able without Claude Code.
"""

# The classification vocabulary (see _dev/02-requirements.md §4, _dev/05-design.md §2.3).
CATEGORIES = ("tech", "biz")
TYPES = ("model", "decision", "guideline", "pitfall", "process")
POLARITIES = ("recommend", "avoid")
MATURITIES = ("draft", "verified", "proven")
KNOWLEDGE_CLASSES = ("point", "causal", "spatiotemporal")
LAYERS = ("L0-P", "L0-T", "L1", "L2", "L3")

# Ordering weights — higher is exposed first during query (M5).
MATURITY_RANK = {"proven": 2, "verified": 1, "draft": 0}
CLASS_RANK = {"spatiotemporal": 2, "causal": 1, "point": 0}
