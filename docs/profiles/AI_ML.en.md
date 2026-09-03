<p align="right"><strong>English</strong> ｜ <a href="AI_ML.ja.md">日本語</a></p>

# AI / ML Engineering Profile

This profile covers changes where **the code can run correctly while quality degrades**.

```text
tests are green
≠
model output quality held
```

## Applies when

```text
prompts, system prompts, few-shot examples
model/provider switches or parameter changes
RAG chunking, embeddings, retrieval
fine-tuning, training data, preprocessing
evaluation code, thresholds, scoring
paths affecting inference cost or latency
```

## Verification dimensions added

- **Baseline comparison** — a change without a prior score cannot be called an improvement
- **Golden eval** — a fixed set of representative inputs and expected properties; the primary regression signal
- **Provenance** — record prompt, model id, dataset version, and parameters so results are reproducible
- **Failure-mode regressions** — keep previously broken inputs in the eval set
- **Cost / latency budget** — check that quality gains did not trade away cost

## Technique gates

| Technique | Apply when | Skip when |
|---|---|---|
| **Golden eval set** | Output quality is a requirement and the area changes repeatedly | One-off experiment |
| **LLM-as-judge** | Human evaluation is the bottleneck and the judge itself can be validated | The criterion is mechanical — a deterministic check is cheaper |
| **Statistical review** | Differences are small enough to be confused with noise | The difference is clearly large |
| **Ablation** | Several changes landed together and attribution is unclear | Single change |
| **Data provenance tracking** | Training/eval data changes over time | Fixed public dataset |

## Definition of correct

State at least:

- the baseline being compared against (when and under what configuration it was measured)
- the input set used for evaluation and why it was chosen
- the acceptable regression envelope (improving every metric at once is rare)
- what is needed to reproduce the result (model id, parameters, dataset version)

## Cost note

Automated evaluation calls models, which **collides directly with Sentrith's credit policy**.

`docs/development/CREDIT_POLICY.md` still applies:

- run evaluation when a change can affect quality, not on every turn
- keep a small golden set for routine use; reserve large sets for pre-release
- do not use an LLM judge where a deterministic check decides the same thing

Model calls for quality verification are not "extra calls for routine work" — but they are still **budgeted**.
