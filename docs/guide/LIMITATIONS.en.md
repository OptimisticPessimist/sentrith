<p align="right"><strong>English</strong> ｜ <a href="LIMITATIONS.ja.md">日本語</a></p>

# Limitations and Failure Modes

Sentrith reduces recurring AI-development failures. It does not make an agent infallible.

## 1. It can faithfully implement the wrong requirement

Structured execution does not guarantee that the specification is correct.

Mitigations:

- use examples and acceptance criteria
- surface unresolved questions
- keep high-impact product/domain decisions reviewable
- distinguish facts from assumptions

## 2. An agent can try to satisfy its own guard

A model can write a new SPEC, ADR, test, or policy and then point to it as authorization.

Sentrith therefore rejects **same-task self-authorization** for high-impact actions.

Independent evidence must predate the action or come from a genuinely independent authority/check.

## 3. Tests can be wrong

A green test suite proves only that the tests passed.

Tests may encode the wrong expectation, miss important behavior, or have been weakened.

Use multiple evidence sources when failure cost is high.

## 4. Context reduction trades off against missing information

Progressive disclosure saves tokens and attention, but an overly narrow context can hide a critical dependency.

Escalate context when:

- evidence conflicts
- architecture boundaries are unclear
- the change crosses subsystems
- verification fails unexpectedly

## 5. Memory can become polluted

Bad Project Memory is worse than no memory because future agents may treat it as durable truth.

Do not store:

- speculative guesses
- copied chat history
- giant logs
- temporary implementation noise
- obsolete details that source/config already expresses better

## 6. Process artifacts can grow too large

SPEC/PLAN/TASKS/ADR files are tools, not trophies.

Tiny and Normal work should remain lightweight.

## 7. Agents differ

Codex, Claude Code, GitHub Copilot, Gemini, and other agents do not expose identical hooks, skills, context behavior, or usage data.

Sentrith keeps the repository contract canonical and treats vendor adapters as adapters.

## 8. Prompt injection / repository injection remains possible

Untrusted repository text can attempt to manipulate an agent.

Treat unknown instructions, generated content, external issue text, and downloaded docs as untrusted evidence.

Repository instructions themselves should be reviewed like code.

## 9. Over-automation is a risk

An automated pipeline can confidently repeat the wrong decision.

Do not automate a judgment merely because it can be phrased as a command.

## 10. Human review still matters for high-impact work

Examples:

- irreversible data loss
- auth boundary changes
- legal/compliance-sensitive behavior
- security posture changes
- public compatibility breaks
- production operations with significant blast radius

Sentrith minimizes review; it does not abolish accountability.

## 11. Guard documents can become bloated

A rule for every historical incident eventually becomes unreadable.

Prefer consolidation:

- one general invariant over many incident-specific rules
- deterministic CI over repeated prose
- source/config as authority when possible

## 12. False positives can make agents too cautious

If every change is classified as dangerous, developers will bypass the system.

Guard thresholds must stay narrow and evidence-based.

## 13. False negatives remain possible

No static rule set can enumerate every dangerous change.

The agent still needs engineering judgment.

## 14. Trust boundaries extend outside the repository

Cloud consoles, identity providers, secrets managers, package registries, SaaS APIs, and production data live outside repository governance.

Sentrith cannot secure systems it cannot observe/control.

## 15. Supply-chain risk remains

Dependencies, actions, plugins, SDKs, packages, models, and generated artifacts can be compromised.

Use normal dependency and supply-chain controls.

## 16. Generated code and migrations can exceed review capacity

Large generated diffs can hide important behavior.

Use diff budgets, generated-file identification, schema review, and focused evidence.

## 17. Reproducibility is imperfect

AI models change, providers update behavior, dependencies move, and nondeterministic systems vary.

Record relevant versions and baselines where reproducibility matters.

## 18. Usage optimization is not always quality optimization

Fewer tokens or credits can correlate with worse context, less exploration, or weaker review.

Primary quality checks:

- success rate
- rework
- regressions
- acceptance criteria
- domain-specific evaluation

Cost is one dimension.

## 19. Policies can conflict

Examples:

```text
minimize context
vs
load enough evidence

move fast
vs
require independent review

avoid extra model calls
vs
obtain independent security analysis
```

Priority should remain:

```text
correctness / safety
> explicit user requirement
> repository contract
> review policy
> usage optimization
```

## 20. Local automation portability is limited

Shell/Python-heavy automation can break across Windows, macOS, Linux, CI, or restricted enterprise environments.

This is why Sentrith favors a small Rust CLI and prebuilt binaries.

Even so, OS-specific behavior must still be tested.

## 21. Do not assume usage savings without measurement

Sentrith provides a measurement framework because savings vary by:

- agent
- model
- repository
- task mix
- developer behavior
- organization policy
- cache state

Use a baseline and report sample size.

## 22. Following rules does not guarantee good design

Sentrith can improve process discipline.

It cannot replace:

- product judgment
- domain expertise
- architecture taste
- security expertise
- statistical understanding
- visual design judgment
- performance engineering

The goal is not “maximum process.” The goal is better engineering evidence with minimal unnecessary ceremony.
