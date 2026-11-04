# Steps

Mohamed's USMLE Step 1 study app: short retrieval sessions, spaced repetition (FSRS), Step 1 style vignette MCQs with an explanation for every option, lessons that explain the why in Egyptian Arabic, cases, and an Anki export. It runs offline on the S24 Ultra, the Tab S11 and the Mac. It is private and never published.

Steps is built on a copy of Batna, Mohamed's internal-medicine MSc app (`MohamedRef/Batna-app`, 2026-11-04): the same engine with the content and course model changed for First Aid.

The content follows **First Aid for the USMLE Step 1 2026**: 16 courses (Section II General Principles: Biochemistry, Immunology, Microbiology, Pathology, Pharmacology, Public Health Sciences; Section III Organ Systems: Cardiovascular, Endocrine, Gastrointestinal, Hematology and Oncology, Musculoskeletal, Neurology, Psychiatry, Renal, Reproductive, Respiratory), one lecture per First Aid subsection (77 in all). Other sources are merged into the same topics ([docs/SOURCES.md](docs/SOURCES.md)).

| Part | State |
|---|---|
| Content schema `steps.content/v1`, sources, validator | ✓ |
| First Aid course and lecture index | ✓ |
| Content | being written, one workstream per course group ([docs/WORKSTREAMS.md](docs/WORKSTREAMS.md)) |
| App shell, review (FSRS), study sessions, question bank, lessons, sync, updates (from Batna) | ✓, renamed; USMLE features in progress |
| Timed vignette blocks, lab values sheet, Rapid Review mode | planned |
| Sync (Cloudflare Worker) and signed updates | code ready; Steps' own Worker, Google client and update key not set up yet |

How to run it: [docs/RUN.md](docs/RUN.md). How to write content: [docs/CONTENT.md](docs/CONTENT.md) and [docs/LESSONS.md](docs/LESSONS.md); the workflow and its checks: [docs/AUTHORING.md](docs/AUTHORING.md).
