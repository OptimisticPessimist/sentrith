<p align="right"><strong>English</strong> ｜ <a href="VRCHAT.ja.md">日本語</a></p>

# VRChat Engineering Profile

VRChat inherits Sentrith's Unity profile but adds its own SDK, performance, and platform constraints.

Sentrith classifies VRChat work by:

- Avatar vs World
- PC / Android / iOS / cross-platform
- visual, animation, physics, Udon, networking, asset, build, or optimization change

Typical verification includes:

- SDK build validation
- avatar performance validation
- Animator / expression integrity
- PhysBone / constraint integrity
- scene / Udon runtime behavior
- allowed component constraints
- platform matrix
- per-platform avatar overrides
- build/export validation
- visual and performance budgets

Sentrith does not hard-code changing VRChat platform limits as timeless rules. Current SDK validation, repository-defined targets, and current official documentation remain the source of truth.

> Correct in Unity is not necessarily correct in VRChat.
