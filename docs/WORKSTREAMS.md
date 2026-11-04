# Workstreams

Each workstream is one project thread. A content workstream writes only its own courses (`content/<course>/`, `content/lessons/<course>/`, `content/book-refs/<course>/`) and appends glossary entries, so they never conflict. `chunk` in `content/index.yaml` names the workstream of each lecture.

| Chunk | Workstream | Courses (First Aid pages) |
|---|---|---|
| 1 | Core: engine, schema, tools, shared files, merges of new sources | all |
| 2 | Biochemistry | bio (32–92) |
| 3 | Immunology and Pathology | imm (94–120), path (202–226) |
| 4 | Microbiology | mic (122–200) |
| 5 | Pharmacology, Public Health and Psychiatry | pharm (228–254), phs (256–278), psych (570–594) |
| 6 | Cardiovascular | cvs (284–328) |
| 7 | Endocrine | endo (330–362) |
| 8 | Gastrointestinal | gi (364–408) |
| 9 | Hematology and Oncology | heme (410–448) |
| 10 | Musculoskeletal, Skin, and Connective Tissue | msk (450–498) |
| 11 | Neurology and Special Senses | neuro (500–568) |
| 12 | Renal and Respiratory | renal (596–628), resp (678–706) |
| 13 | Reproductive | repro (630–676) |
| 14 | USMLE app features: timed vignette blocks, lab values sheet, Rapid Review mode, course screens without MSc wording | crates/app |

Order inside a content workstream: one lecture per PR, in book order; items first, then its lessons.
