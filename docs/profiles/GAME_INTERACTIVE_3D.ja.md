<p align="right"><a href="GAME_INTERACTIVE_3D.en.md">English</a> ｜ <strong>日本語</strong></p>

# Game / Interactive 3D Engineering Profile

SentrithのGame / Interactive 3D Profileは、Unit Testだけでは判断できない変更を扱います。

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

Engineそのものを目的にせず、変更内容からverificationを選びます。

### Unity

主な対象:

- C#
- Scene / Prefab
- Animator / Animation
- Material / Shader
- ScriptableObject
- Serialized references
- Build target

### Godot

主な対象:

- GDScript / C#
- Scene Tree / Nodes
- Signals
- Resources
- Input Map
- Shader
- Export presets

### Unreal Engine

主な対象:

- Blueprint / C++
- Actor / Component
- Level / World
- Material / Niagara
- Animation Blueprint
- Data Asset / Data Table
- Collision / Physics
- Packaging / target platform

### VRChat

VRChatはUnity adapterの派生ですが、SDK・platform・performance制約が強いため独立subprofileとして扱います。

詳細: [`VRCHAT.ja.md`](VRCHAT.ja.md)
