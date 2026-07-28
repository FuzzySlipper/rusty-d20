# Live Testing

Live scenarios are opt-in and gated by `LIVE_RUN=1`. They require `BASE_URL` from the run broker or the local fallback:

```bash
pnpm run serve:local
BASE_URL=<printed-url> LIVE_RUN=1 pnpm run e2e:live
```

Each scenario must write an evidence packet, milestone screenshots when the claim is visual, console and page error dumps, visible text, and explicit non-claims.

Completion evidence report:

```text
Live scenario:
Command:
Backend/profile:
Artifacts:
Screenshots inspected:
Rendered behavior observed:
Evidence packet:
Timeline notes:
Supporting checks:
Non-claims / residual risk:
```
