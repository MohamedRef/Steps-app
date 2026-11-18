# The VQ question bank (sealed)

VQ is Mohamed's own USMLE Step 1 question bank: 1,314 questions, `VQ-0001` to `VQ-1314`, 522 of them with a figure. In Steps each one is an ordinary MCQ filed under the First Aid lecture and topic it tests, so the bank, the blocks, the mistakes log and his ✓ / ✎ / ✗ checks take it like any other question.

The repo and the content packs its releases carry are public, so the questions stay **encrypted in git and in every pack**. They open only on a device where Mohamed has entered his key. Without the key Steps behaves exactly as it does without the bank.

## The key

- 32 random bytes, written as 52 letters and digits in groups of four (`ABCD-EFGH-…`); spaces, dashes and case don't matter when it is typed. `cargo run -p steps-tools -- vq keygen` makes one and prints it with its **key id** (8 hex digits); nothing is written to disk.
- It lives in two places only: Mohamed's password manager, and the GitHub Actions secret `STEPS_VQ_KEY`, which CI uses to check the questions. **Never in git**, an issue, a pull request or a chat.
- The key id names the key in the sealed files and on screen. It tells nothing about the key.
- Changing the key: `vq open` with the old key, delete `content/*/vq/`, `vq seal` with the new key, then update the secret and unlock each device again.

## The files

`content/<course>/vq/LNN.yaml`, one per lecture that has VQ questions, inside the course folder so a content PR stays inside its course:

```yaml
# VQ bank questions filed under this lecture, sealed (docs/VQ.md).
schema: steps.sealed/v1
lecture: path.L03
key: "3f9a1c2b"  # key id: which key sealed them (never the key)
items:
  - { id: path.L03.vq0001, kind: mcq, sealed: "<base64url>" }
```

- `id` and `kind` are in clear, so `steps-tools ids` checks his records' ids without the key. An id is `<lecture>.vqNNNN`, NNNN the VQ number (`path.L03.vq0001` is VQ-0001).
- `sealed` opens to `{ "item": <the item>, "image": "<base64 of the WebP file>", "image_type": "image/webp", "objective": "<the educational objective>" }`; a question without a figure or an objective leaves those fields out.
- Only `steps-tools vq seal` writes these files. They are byte-stable: the same question under the same key gives the same bytes, so an untouched question never shows in a diff.
- Crypto (`crates/core/src/sealed.rs`): XChaCha20-Poly1305 with the item id as associated data, under a subkey of keyed BLAKE2b-256 (`steps vq enc`); the nonce is keyed BLAKE2b (`steps vq nonce`) of `id ‖ 0x00 ‖ plaintext`. A blob is `0x01 ‖ nonce ‖ ciphertext+tag` in base64url. A wrong key, a changed byte or a blob moved to another id doesn't open.
- Nothing else belongs in a `vq/` folder: `validate` refuses any other file there, and a VQ question in an open lecture file.

Each question as an item: `kind: mcq`, a `topic` of its lecture, `stem`, `options` (as many as the question has, 2 to 26), `answer`, `why` (one per option), `src: { book: vq, page: <the VQ number> }`, `status: unverified` and `tags: [vq]`, and optionally its **educational objective** (the one point it teaches, in English as the bank words it), which the app shows with the explanation. VQ questions stay exactly as written (Mohamed, 2026-11-18): any number of options, the key may be the longest, and they are left out of the answer-cue rules. The `vq` source is in `content/sources.yaml` as a `qbank`.

## Fixing or adding a question

With the key in the shell (`export STEPS_VQ_KEY=…`, pasted from the password manager):

1. `cargo run -p steps-tools -- vq open ../vq-open` writes every sealed lecture as plain YAML to `../vq-open/<course>/LNN.yaml` and the figures to `../vq-open/images/VQ-NNNN.webp`. The folder must be outside the repo and new or empty; `vq-open/` is in `.gitignore` in case a copy lands in the repo anyway.
2. Edit the questions there; a question's educational objective sits on its item as `objective:`. To add one, add an item with its id and a figure `images/VQ-NNNN.webp` if it has one. To move a question to another lecture, move it to that lecture's file and change its id's lecture part; its old id goes in `content/ids-retired.txt` (docs/CONTENT-IDS.md).
3. `cargo run -p steps-tools -- vq seal ../vq-open` writes `content/<course>/vq/LNN.yaml` for every lecture there and removes the sealed file of a lecture left without questions. It refuses a folder with problems (an unknown key, an id outside its lecture, a number filed twice, a figure without a question) and writes nothing then. Opening and sealing again changes nothing.
4. `cargo run -p steps-tools -- validate`, then a PR as usual, with its `changes/<topic>.yaml`.
5. Delete `../vq-open` once the PR is merged.

## What validate checks

- **Always, without the key:** each file's schema, a lecture that is listed and written, the file in its lecture's folder, `<lecture>.vqNNNN` ids sorted and used once (also against open items), each VQ number once, `kind: mcq`, and one key id in every file.
- **With `STEPS_VQ_KEY`:** every question opened and checked inside its lecture with the usual item rules (topic, MCQ shape, approvals, unknown keys), plus `src.book: vq`, `src.page` equal to the id's number, the `vq` tag, a figure that is WebP and at most 200 KB, and an objective that isn't empty (leave it out instead). A figure is optional: a stem's "is shown" often means values shown as text, and `no-figure` is an ordinary tag validate doesn't check.
- **Without the key** it prints `VQ: N sealed questions not opened (STEPS_VQ_KEY is not set)`. CI sets the secret and `STEPS_VQ_REQUIRED=1`, which makes unopened questions a failure; with no sealed files nothing fails.
- `packs build` copies the files into the pack as they are, `ids check` / `ids list` include their ids, and `steps-tools counts` leaves them out: the numbers in the README and the handbook are the public content's. `steps-tools verdicts` reads his checks on VQ questions with the key and only names them without it.

## Unlocking a device

Settings › بياناتك › «مفتاح بنك VQ»: paste the key and tap «افتح». Steps checks it against the key id of the questions it carries, keeps it, and opens the library again with them: the card then says «مفتوح: N سؤال», the bank has a «أسئلة VQ» filter, a figure shows above its question's options (a tap opens it full size), the explanation shows its "Educational objective" after the why lines, the page chip says `VQ-0001`, and quick search finds `VQ-0001`, `vq0001` or `vq 1`. «اقفل على الجهاز ده» forgets the key on that device.

- The key is kept like the sign-in session (`crates/app/src/vq/vault.rs`): on the Mac and other desktops in `vq-key.txt` in the data folder, readable only by his user; on Android in the Keystore; in the browser in the site's `localStorage`. Never in the database, so never in a backup, an export or a sync: each device is unlocked once.
- A wrong or missing key opens nothing and shows no error; the card says the key kept there doesn't fit.
- Questions he opened stay in Steps: the Anki export leaves them out, as `steps-tools anki` does.
