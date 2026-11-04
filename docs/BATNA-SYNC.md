# Keeping up with Batna

Steps shares Batna's engine and UI. Batna's app, UI and tool improvements are ported to Steps so the work is never done twice; Batna's content (`content/`) and changelog (`changes/`) never are.

**Last Batna commit ported: `c7927f5` (Batna main, 2026-11-04).**

How to port (the core thread does this):

```bash
git -C ../batna-app fetch origin main
git -C ../batna-app diff <last-ported> origin/main -- . ':!content' ':!changes' \
  | sed 's/BATNA/STEPS/g; s/Batna/Steps/g; s/batna/steps/g; s/لباطنة/لـ Steps/g; s/وباطنة/و Steps/g; s/باطنة/Steps/g' \
  > /tmp/port.patch
git apply --reject --whitespace=nowarn /tmp/port.patch   # then fix any *.rej by hand
```

Then run `just check`, fix tests that expect Batna's courses (`hb`, `res`…), add a `changes/` line for anything Mohamed can see, update the commit above, and open a PR titled "Port Batna main up to <sha>".

Watch for: Batna-only features (its MSc calculators, course wording such as credit hours and semesters), which Steps may adapt rather than copy, and anything signed or keyed (update keys, Worker names, Google client), which Steps never shares with Batna.
