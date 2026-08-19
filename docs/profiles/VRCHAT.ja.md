<p align="right"><a href="VRCHAT.en.md">English</a> ｜ <strong>日本語</strong></p>

# VRChat Engineering Profile

VRChatはUnity上で動くためUnity Profileを継承しますが、通常のUnityプロジェクトとは異なる制約を持つため、Sentrithでは独立したSubprofileとして扱います。

## 最初の分類

### Content type

- Avatar
- World
- Both / shared tooling

### Target platform

- Windows / PC
- Android / Quest / mobile
- iOS
- Cross-platform
- Unknown

### Change area

- C# / editor tooling
- Udon / UdonSharp
- Avatar Animator / Expressions
- PhysBones / Constraints
- Prefab / Scene
- Material / Shader
- Mesh / Rig / BlendShapes
- Audio / Video
- Networking / synchronization
- Build / Publish / SDK configuration
- Optimization
- Multiple

## Avatar verification

変更内容に応じて確認する候補:

- Avatar Performance Rank
- download / uncompressed size constraints
- material / texture / mesh cost
- PhysBone / Constraint usage
- Animator Controller integrity
- expression parameters / menus
- humanoid rig / bones
- BlendShape integrity
- missing references
- per-platform avatar override
- PC / Android / iOS feature differences
- upload/build validation

Sentrithは「Performance RankがGoodでなければ失敗」のような固定ルールを標準にはしません。
目標Rankやplatform budgetはプロジェクト側のAcceptance Criteriaとして保持します。

## World verification

変更内容に応じて確認する候補:

- Scene integrity
- Udon/UdonSharp compilation
- allowed component constraints
- networking/synchronization behavior
- ClientSim / runtime behavior where useful
- mobile shaders / rendering constraints
- build size / platform limits
- input differences between desktop / VR / mobile
- audio/video behavior
- Build & Test / target-device validation
- cross-platform feature parity or documented differences

## Cross-platform rule

PC版が動くだけではcross-platform変更の完了条件になりません。

Target Matrix例:

```text
                 PC   Android   iOS
Avatar Build      ✓      ✓       ✓
Visual            ✓      ✓       ✓
Expressions       ✓      ✓       ✓
PhysBones         ✓      ✓       ✓
Performance       ✓      ✓       ✓
```

Worldなら:

```text
                 PC   Android   iOS
Build             ✓      ✓       ✓
Udon behavior     ✓      ✓       ✓
Input             ✓      ✓       ✓
Visual            ✓      ✓       ✓
Performance       ✓      ✓       ✓
```

必要なplatformだけ選びます。

## Performance Budget

固定値をSentrith本体へ焼き込みません。

VRChat SDK/公式制限は変更され得るため:

1. current SDK validation
2. repository-defined target
3. current official platform limits

をsource of truthとして使います。

プロジェクト側では例えば:

```text
Target:
- Avatar Performance Rank: Good
- Android compatible: required
- iOS compatible: optional
- Material slots: project-defined budget
```

のように保持できます。

## Questionnaire

ユーザーにVRChat専門用語を大量入力させないため、選択式で分岐します。

### Q1. 何を作っていますか？

A. Avatar  
B. World  
C. Editor / build tool  
D. 複数  
E. 分からない

### Q2. 対象platformは？

A. PCのみ  
B. PC + Android / Quest  
C. PC + Android + iOS  
D. Mobileのみ  
E. 分からない

### Avatar branch

主に何を変更しますか？

A. 見た目 / Material / Texture  
B. Mesh / Rig / BlendShape  
C. Animator / Expressions  
D. PhysBones / Constraints  
E. Optimization  
F. Build / SDK  
G. 複数

### World branch

主に何を変更しますか？

A. Scene / Environment  
B. Udon / UdonSharp  
C. Networking / Sync  
D. UI / Input  
E. Audio / Video  
F. Optimization  
G. Build / SDK  
H. 複数

### Q4. 見た目の正しさは重要ですか？

A. ほぼ不要  
B. 一部重要  
C. 非常に重要

### Q5. platform間で機能差を許容しますか？

A. 同等機能が必要  
B. 見た目差のみ許容  
C. 一部機能差を許容  
D. platform別設計でよい  
E. 分からない

この回答からSentrithがVerification Profileを生成します。

## Example Engineering Profile

```text
Profile: VRChat Avatar / Cross-platform

Core
✓ Evidence-driven
✓ Visual Acceptance
✓ Project Memory

Unity
✓ Serialized reference integrity
✓ Animator integrity

VRChat
✓ SDK build validation
✓ Avatar performance validation
✓ PC / Android platform matrix
✓ per-platform override awareness

Optional
○ iOS validation
○ visual regression
○ performance budget regression
```

## Principle

> Unityで正しいだけでは、VRChatで正しいとは限らない。

VRChat SDKとtarget platformで実際に成立することをverification evidenceとして扱います。
