# flowlive/1, bozza dello schema

Bozza per lo studio `README.md` §2.7; la PR L1 la porta in `docs/schemi/flowlive-1.json`.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://freeesta.github.io/mzlab/schemi/flowlive-1.json",
  "title": "flowlive/1 (draft): one event of the live Flow scheduler",
  "description": "Draft of the study docs/studi/flow-live/README.md §2.7. PR L1 moves the final version to docs/schemi/flowlive-1.json.",
  "type": "object",
  "required": ["format", "seq", "rev", "kind", "instance", "state"],
  "additionalProperties": false,
  "properties": {
    "format": { "const": "flowlive/1" },
    "seq": { "type": "integer", "minimum": 0, "description": "Grows by one with every event of a scheduler." },
    "rev": { "type": "integer", "minimum": 0, "description": "Revision of the plan the event belongs to; an event of an older revision is dropped by the page." },
    "kind": { "enum": ["job", "node", "plan"] },
    "instance": { "type": "string", "description": "Instance id of the plan; empty for kind plan." },
    "item": { "type": ["string", "null"], "description": "Name of the plan input the job is driven by (never an index); null for a step that is not per_item and for kind node." },
    "state": { "enum": ["waiting", "ready", "running", "done", "reused", "stale", "cancelled", "error", "refused", "blocked", "skipped", "paused"] },
    "frac": { "type": ["number", "null"], "minimum": 0, "maximum": 1, "description": "Share done; null = not known (indeterminate)." },
    "parte": { "type": ["integer", "null"], "minimum": 0, "description": "kind node: jobs done." },
    "di": { "type": ["integer", "null"], "minimum": 0, "description": "kind node: jobs in all." },
    "ms": { "type": ["number", "null"], "minimum": 0, "description": "Time of the finished jobs of this run, as the executor measured it." },
    "mb": { "type": ["number", "null"], "minimum": 0, "description": "Memory the job asks for or measured." },
    "via": { "enum": ["browser", "computer", null], "description": "Where the step can run." },
    "key": { "type": ["string", "null"], "pattern": "^[0-9a-f]{64}$", "description": "plan_key of the job (SHA-256)." },
    "blocked_by": { "type": ["string", "null"], "description": "state blocked: what it waits for (permission.memory, platform.computer, plan.problem)." },
    "error": {
      "type": ["object", "null"],
      "required": ["error_key"],
      "properties": { "error_key": { "type": "string" }, "params": { "type": "object" } }
    }
  }
}
```
