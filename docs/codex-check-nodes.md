# Codex Check Nodes Workflow Instructions

Use the following snippets when triggering the production node health checks.

## Comment for Codex (PR or Issue)

Paste this comment verbatim so Codex runs the workflow with the correct hosts and URLs.

```
Codex: run the “Check IPPAN Nodes” workflow
– hosts: 188.245.97.41,135.181.145.174
– ui_url: http://188.245.97.41:3001
– api_base: https://api.ippan.org
– lb_health: http://127.0.0.1:3000/lb-health
– Verify services active, /health /status /peers = 200, peer_count > 0, versions match
– Upload JSON summaries as artifacts and post a pass/fail summary per host
```

## Issue Body Template (tracking request)

```
**Targets**
- env: production
- hosts:
  - 188.245.97.41
  - 135.181.145.174
```

## Optional: GitHub CLI Trigger

Run the workflow manually from your terminal with the updated defaults.

```bash
gh workflow run "Check IPPAN Nodes" \
  -f hosts="188.245.97.41,135.181.145.174" \
  -f ui_url="http://188.245.97.41:3001" \
  -f api_base="https://api.ippan.org" \
  -f lb_health="http://127.0.0.1:3000/lb-health"
```
