<p align="right"><strong>English</strong> ｜ <a href="README.ja.md">日本語</a></p>

# Engineering Profiles

Profiles add domain-specific verification and technique gates while Sentrith Core stays vendor/tool neutral.

- [Web / Backend](WEB_BACKEND.en.md)
- [AI / ML](AI_ML.en.md)
- [Data Science / Data Engineering](DATA.en.md)
- [Game / Interactive 3D](GAME_INTERACTIVE_3D.en.md)
  - [VRChat](VRCHAT.en.md)

Profiles should describe the problem first and methodology jargon second.

---

## Profiles are not exclusive modes

A profile is not a mode you pick one of. Profiles are **additive overlays**.

A project can enable several, and combinations are the normal case:

```text
API server with RAG        -> Web/Backend + AI/ML
ML with a feature pipeline -> AI/ML + Data
Game backend service       -> Game/3D + Web/Backend
```

There is deliberately **no** "Backend × AI/ML" combination document. Four domains would need fifteen such files, duplicating rules and inflating context.

## Composition rules

Each profile declares only three things:

```text
1. Applicability triggers (which files / kinds of change it covers)
2. Verification dimensions it adds
3. Technique gates (techniques applied only when a condition holds)
```

During a task:

1. Read only the enabled profiles whose **triggers match** the change
2. Apply the **union** of the matched profiles' verification dimensions
3. Collapse duplicate items (regression tests and the like) into one

Profiles add verification rather than dictating implementation structure, so they rarely conflict by construction.

When they appear to conflict, **follow the stricter rule**, using this ordering:

```text
correctness / safety
> explicit user requirement
> repository contract
> review policy
> cost optimization
```

## Where enabled profiles are recorded

`docs/ai/PROFILE.md` holds only an index of the enabled profiles and their triggers.

It contains no technique explanations; those live in the profile documents here.

`PROFILE.md` is loaded on ordinary tasks, so **keeping it short is a requirement** (roughly 100 lines or fewer).

It is initialized by the profile questions in `docs/ai/BOOTSTRAP.md`.
