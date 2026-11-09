# Keeping up with Batna

Steps shares Batna's engine and UI. Batna's app, UI and tool improvements are ported to Steps so the work is never done twice; Batna's content (`content/`) and changelog (`changes/`) never are.

**Last Batna commit ported: `e2e9517` (Batna main, 2026-11-04).** From `0b6e07e` up to it, ported: the web app's service worker and separate content pack (#624), username + password sign-in and lessons resuming on any device (#621, with the password key re-pinned for Steps' own salt `steps-login-v1`), sync property tests (#625) and the refs `-0%` fix (#629). Skipped: calculators (#626, #634, #637), Batna content (#622, #636), and Batna's test and coverage work (#627, #628, #630, #631): Steps rebuilt its own suite and coverage gate, and those PRs' code changes only reshaped code for Batna's tests.

Ported on its own since (each adapted, not patched): Batna #737 (#643), #739/#740 (#645), #741/#743 (#652), #753/#755/#756 (#650), #767 colours (#651), #770 own highlights (#649), #771 reminders (#654), and on 2026-11-09 #746-#751, #758, #760 and #762 (phone touch targets, phone quick search, Cloud test CI). Not ported: #769 (Batna's own icon), #768 (Batna's calendar version numbers), #765 (Batna's Mac preview workflow), calculator parts of #744 and #748.

How to port (the core thread does this):

```bash
git -C ../batna-app fetch origin main
git -C ../batna-app diff <last-ported> origin/main -- . ':!content' ':!changes' \
  | sed 's/BATNA/STEPS/g; s/Batna/Steps/g; s/batna/steps/g; s/لباطنة/لـ Steps/g; s/وباطنة/و Steps/g; s/باطنة/Steps/g' \
  > /tmp/port.patch
git apply --reject --whitespace=nowarn /tmp/port.patch   # then fix any *.rej by hand
```

Then run `just check`, fix tests that expect Batna's courses (`hb`, `res`…), add a `changes/` line for anything Mohamed can see, update the commit above, and open a PR titled "Port Batna main up to <sha>".

Watch for: Batna-only features (MSc calculators, course wording such as credit hours and semesters), which Steps leaves out (its calculators were removed on 2026-11-04), and anything signed or keyed (update keys, Worker names, Google client), which Steps never shares with Batna.
