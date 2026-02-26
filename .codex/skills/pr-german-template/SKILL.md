---
name: pr-german-template
description: Create a PR body with German feature explanation, test evidence, and screenshot links.
tools: Bash, Read, Grep
---

# PR German Template

## PR Body Sections
1. Summary (English short technical overview)
2. Deutsche Erklaerung (what the feature does for users)
3. Changes included
4. Validation commands and results
5. Screenshots (0x0 links or repo paths)
6. Risks or follow-ups

## German Section Template
```md
## Deutsche Erklaerung
Dieses Feature fuegt <feature> hinzu. Dadurch koennen Spieler <effect>. 
Die Aenderung verbessert <benefit> und bleibt bewusst klein im Umfang.
```

## Final Gate
Do not finalize until PR URL exists and evidence links are present.
