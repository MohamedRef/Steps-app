# Steps

Steps is my USMLE Step 1 study app. I built it for one learner, me, and it is private.

It is made for short, focused sessions: spaced repetition (FSRS), Step 1 style vignette questions with an explanation for every option, lessons that explain the why in Egyptian Arabic, cases, timed blocks and an Anki export. It works offline on my phone (S24 Ultra), my tablet (Tab S11), my Mac and in any browser, and keeps them in sync.

The current version is **1.0.0-alpha.3**, the third alpha.

## What I study from

The backbone is **First Aid for the USMLE Step 1 2026**. Steps follows the book's own order:

- **General Principles:** Biochemistry, Immunology, Microbiology, Pathology, Pharmacology, Public Health Sciences
- **Organ Systems:** Cardiovascular, Endocrine, Gastrointestinal, Hematology and Oncology, Musculoskeletal and Skin, Neurology and Special Senses, Psychiatry, Renal, Reproductive, Respiratory
- **Rapid Review**, as its own course

That is one lecture for each First Aid subsection, and every item points back to its printed page in the book, so I can open the page and check it.

When I add other sources, they go under the same First Aid topics, and each item says which source it came from, so one topic gathers everything I have on it.

## What is inside

| | |
|---|---|
| Cards | about 5,100 |
| Step 1 style questions (single best answer, A–E) | about 1,700 |
| Clozes | about 1,050 |
| Tables and lists to recall | about 1,500 |
| Lessons in Egyptian Arabic, with drawings and try-it steps | about 360 |
| **All study items** | **about 9,400** |

Every item starts as unverified, and only I mark it approved after I have checked it against the book.

Around the content: study sessions that pick up where I left them, a question bank, timed blocks in the 2026 exam format (up to 20 questions in 30 minutes), an exam planner with catch-up, my NBME and UWSA scores with a readiness check, a lab values sheet, go to a book page, weak spots, the Boards & Beyond videos and UWorld question ids for each lecture with a custom-test maker, a log of my UWorld misses, my own notes, cards and highlights, oral mode with the screen off, quick search and study reminders.

## How it looks

Steps has its own look: the **S** icon that climbs like a stair with a gold dot at the top, the **Readex Pro** font, and an Egyptian Arabic, right to left interface.

- **10 themes:** صبح, بترولي, أسود AMOLED, سهر, سكرَب, سيبيا, نيلي, تباين عالي, مذاكرة and مذاكرة خافت, plus three that follow the device's light and dark.
- **10 styles** for cards and buttons: درجات (the default), خط رفيع, من غير حدود, فواصل, خط جانبي, رمادي فاتح, أبيض وأسود, خطوط, ملوّن and زجاج.

## Sync and sign-in

**Steps Cloud** keeps my progress the same on every device. It runs on Cloudflare's free plan. I sign in with Google, or with a username and password on a device that doesn't have my Gmail (a borrowed laptop, for example). Everything also works offline, and a full backup can be exported from Settings › بياناتك.

## Install the alpha

The alpha is on GitHub: [Steps 1.0.0-alpha.3](https://github.com/MohamedRef/Steps-app/releases/tag/steps-v1.0.0-alpha.3).

- **Android (phone and tablet):** install `steps-1.0.0-alpha.3.apk`. It installs over a preview build signed with the same key and keeps the data.
- **Mac:** unzip `Steps-1.0.0-alpha.3-mac.zip`, move `Steps.app` to Applications, and run this once in Terminal:
  `xattr -dr com.apple.quarantine /Applications/Steps.app`
- **Web:** open <https://steps-web.hamorefaat85.workers.dev>, then "Add to Home Screen" or "Install app" if you like.

After the first install, new versions and new lectures arrive by themselves, and each update says what changed. Settings › التحديثات switches between the alpha, beta and stable channels without reinstalling. Before installing anything over an older build, export a backup first.

## For whoever works on it

Start with **[docs/HANDBOOK.md](docs/HANDBOOK.md)**: the whole project on one page (the rules I work by, how it is built, tested, merged and released, and where things stand right now), with links to every other doc.

— Mohamed Refaat
