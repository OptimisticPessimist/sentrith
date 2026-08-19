<p align="right"><strong>English</strong> ｜ <a href="GAME_INTERACTIVE_3D.ja.md">日本語</a></p>

# Game / Interactive 3D Engineering Profile

Game and interactive-3D work often cannot be verified with unit tests alone.

## Common verification

- Visual Acceptance Criteria
- Runtime / Behavioral Verification
- Scene / Asset Integrity
- Input / Physics Verification
- Platform Matrix
- Performance Budget
- Build / Export Verification

## Engine adapters

```text
Game / Interactive 3D
├─ Unity
│  └─ VRChat
├─ Godot
└─ Unreal Engine
```

The engine is not the methodology. Sentrith selects verification based on what changed.

## Unity

Typical change surfaces:

- C#
- Scenes / Prefabs
- Animator / Animation
- Materials / Shaders
- ScriptableObjects
- serialized references
- build targets

Typical evidence may include edit/play-mode tests, scene/prefab checks, runtime validation, visual acceptance, and platform builds.

## Godot

Typical change surfaces:

- GDScript / C#
- Scene Tree / Nodes
- Signals
- Resources
- Input Map
- shaders
- export presets

A parsed script is not sufficient evidence when Node paths, Signals, or exported runtime behavior changed.

## Unreal Engine

Typical change surfaces:

- Blueprint / C++
- Actor / Component structure
- Level / World
- Material / Niagara
- Animation Blueprint
- Data Assets / Data Tables
- collision / physics
- packaging / target platforms

Blueprint-heavy and C++-heavy projects may require different verification emphasis.

## VRChat

VRChat inherits the Unity profile but has additional SDK, performance, content, and cross-platform constraints.

See:

- [VRChat Profile (English)](VRCHAT.en.md)
- [VRChat Profile (Japanese)](VRCHAT.ja.md)

## Questionnaire examples

Ask the user what kind of change is involved:

- gameplay logic
- scene/level hierarchy
- material/shader/visual
- animation
- physics/collision
- UI/input
- assets/resources
- build/export/packaging
- multiple areas

Then ask which engine and target platforms apply.

The goal is to select the right evidence, not to force a single testing doctrine.
