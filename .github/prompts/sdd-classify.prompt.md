---
agent: 'agent'
description: 'Classify this task and apply the lightest safe workflow'
---

Use the `/task-classify` agent skill for this request.

Request/context: ${input:task:Describe the task or relevant scope}

Follow the canonical workflow in `.agents/skills/task-classify/SKILL.md`.
