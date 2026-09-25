# Engineering Decisions

> Durable decisions whose rationale would otherwise be expensive to rediscover.
> This is not a changelog.

## Entry template

### ADR-YYYYMMDD-NN — Short decision title

**Status:** accepted | superseded | deprecated

**Context**

What problem or constraint required a decision?

**Decision**

What was chosen?

**Rationale**

Why was this chosen for this repository?

**Alternatives considered**

- Alternative A — why rejected
- Alternative B — why rejected

**Consequences**

- positive consequence
- tradeoff or constraint
- follow-up obligation

**Affected areas**

- `path/to/file`
- subsystem/API/schema

---

### ADR-001 — Policy and Decision separation

**Status:** accepted (2026-09-25)

**Context:** Model output and subscription availability cannot be trusted as authorization.

**Decision:** Deterministic policy produces allowed execution profiles before any Decision Engine ranks them. The selected ID must be one of those profiles; unsafe or unknown model output falls back visibly to a rule engine or Human Gate. Human override chooses among allowed profiles and cannot bypass security or billing constraints.

**Rationale:** This makes routing preferences replaceable while keeping permissions and charges enforceable in code. Allowing a model to apply its own policy was rejected.

**Consequences:** Every provider, budget, and security signal needs a typed policy result and a test. A provisional rule route has no fabricated score.

**Affected areas:** `tools/sentrith/src/routing.rs`; future decision adapters.

---

### ADR-002 — Modular monolith inside the existing Rust CLI

**Status:** accepted (2026-09-25)

**Context:** The repository already ships one Rust binary and release pipeline.

**Decision:** Add internal modules to `tools/sentrith` and preserve existing commands. Split crates or services only when a demonstrated independent build, reuse, or deployment boundary exists.

**Rationale:** A new workspace or daemon would increase integration and release cost before routing contracts are proven. A separate rewrite was rejected.

**Consequences:** Module boundaries must keep policy, model, provider, validation, context, and storage concerns separate even within one crate.

**Affected areas:** `tools/sentrith`; release scripts and CI.

---

### ADR-003 — SQLite metadata plus compressed content objects

**Status:** accepted; metadata implemented, content objects pending (2026-09-25)

**Context:** Routing metrics need queries, while large raw outputs need deduplication and independent retention.

**Decision:** Store metadata and object references in `.sentrith/sentrith.db`; store large recoverable bytes as SHA-256 addressed zstd objects in `.sentrith/objects/`. Keep `.ai-usage/` intact. Retention is configurable and GC must preserve live references.

**Rationale:** Large SQLite blobs complicate growth and lifecycle management. Plain files without an index complicate audit and GC.

**Consequences:** Storage dependencies need a supply-chain review; writes, symlinks, permissions, integrity, and crash recovery need tests before use. Do not store secrets in logs.

**Affected areas:** future storage/context modules; `.gitignore`.

---

### ADR-004 — Execution Profiles as routing units

**Status:** accepted (2026-09-25)

**Context:** Provider, model, effort, and review combinations have compatibility constraints.

**Decision:** Rank named ExecutionProfile IDs. A provider adapter maps each selected profile to verified current CLI/model settings. Do not let a model independently choose provider and effort.

**Rationale:** Profile candidates are auditable and policy can reject a whole incompatible combination. Product-specific strings in model training knowledge were rejected.

**Consequences:** Profile definitions are versioned configuration; unsupported effort mappings disable a profile rather than silently substituting another.

**Affected areas:** routing/config/provider modules.

---

### ADR-005 — Suggest-first automation rollout

**Status:** accepted (2026-09-25)

**Context:** Agent execution can change code, consume subscriptions, and fail despite a successful exit.

**Decision:** Default to Suggest without agent execution; local decision metadata may be recorded. Add Assist for one verified adapter before Auto; Auto stays opt-in until validation, review, bounded retry, and security behavior have measured evidence.

**Rationale:** A recommendation can be inspected and reversed; execution needs stronger evidence. Default Auto was rejected.

**Consequences:** `route` must declare whether a model or rule engine decided. `run` and success metrics remain separate gates.

**Affected areas:** CLI, config, executor, validator, logging.
