# Calculators

Generated from `crates/core/src/calc` (run `cargo test -p steps-core -- --ignored print_reference`). A `calc` item names one of these in `calculator:` and gives its `inputs` by key (a key may drop its unit suffix: `alt` for `alt_u_l`). `just validate` computes every item, so a typo, a missing input or an `expect` that disagrees fails the check. `expect` may be the band id (`child_pugh.b`) or text that contains the result (`Child-Pugh B (8 points)`, `Stage C`).

## `child_pugh`: Child-Pugh

Source: Book Lecture 1, Table 2 (p. 14); INR bands: Pugh 1973 as used by AASLD

| input | kind | |
|---|---|---|
| `encephalopathy` | `none`, `grade_1_2`, `grade_3_4` | Encephalopathy |
| `ascites` | `absent`, `mild_moderate`, `severe` | Ascites |
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |
| `albumin_g_dl` | number, g/dL | Albumin |
| `pt_prolonged_s` | number, s | Prothrombin time, seconds prolonged; optional |
| `inr` | number | INR; optional |

## `meld`: MELD

Source: Formula: UNOS/OPTN MELD (Kamath 2001). The book (Lecture 1, p. 14) names MELD without a formula.

| input | kind | |
|---|---|---|
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |
| `inr` | number | INR |
| `creatinine_mg_dl` | number, mg/dL | Creatinine |
| `dialysis` | true / false (default false) | Dialysis twice in the past week |

## `meld_na`: MELD-Na

Source: Formula: UNOS/OPTN MELD-Na 2016 (Kim 2008). The book (Lecture 1, p. 14) names MELD-Na without a formula.

| input | kind | |
|---|---|---|
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |
| `inr` | number | INR |
| `creatinine_mg_dl` | number, mg/dL | Creatinine |
| `dialysis` | true / false (default false) | Dialysis twice in the past week |
| `sodium_mmol_l` | number, mmol/L | Sodium |

## `albi`: ALBI grade

Source: Johnson 2015; book Lecture 18, p. 363

| input | kind | |
|---|---|---|
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |
| `albumin_g_dl` | number, g/dL | Albumin |

## `fib4`: FIB-4

Source: Sterling 2006; book Lecture 6, p. 102

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `ast_u_l` | number, U/L | AST |
| `alt_u_l` | number, U/L | ALT |
| `platelets_10e9_l` | number, ×10⁹/L | Platelets |

## `apri`: APRI

Source: Book Lecture 2, p. 34; Wai 2003; WHO HCV guidelines 2016

| input | kind | |
|---|---|---|
| `ast_u_l` | number, U/L | AST |
| `ast_uln_u_l` | number, U/L | AST upper limit of normal |
| `platelets_10e9_l` | number, ×10⁹/L | Platelets |

## `fli`: Fatty liver index (FLI)

Source: Bedogni 2006 (formula and cut-offs; the book names the FLI only, Lecture 6, p. 101)

| input | kind | |
|---|---|---|
| `tg_mg_dl` | number, mg/dL | Triglycerides |
| `bmi` | number, kg/m² | BMI |
| `waist_cm` | number, cm | Waist circumference |
| `ggt_u_l` | number, U/L | GGT |

## `baveno_vi`: Baveno VI (skip variceal screening)

Source: Baveno VI 2015; book Lecture 2, p. 35 (EASL 2021)

| input | kind | |
|---|---|---|
| `lsm_kpa` | number, kPa | Liver stiffness (TE) |
| `platelets_10e9_l` | number, ×10⁹/L | Platelets |

## `r_ratio`: R ratio (DILI pattern)

Source: Book Lecture 8, p. 129; EASL DILI 2019; Kullak-Ublick 2017

| input | kind | |
|---|---|---|
| `alt_u_l` | number, U/L | ALT |
| `alt_uln_u_l` | number, U/L | ALT upper limit of normal |
| `alp_u_l` | number, U/L | ALP |
| `alp_uln_u_l` | number, U/L | ALP upper limit of normal |

## `maddrey`: Maddrey discriminant function

Source: Maddrey 1978, Carithers 1989; AASLD 2019

| input | kind | |
|---|---|---|
| `pt_s` | number, s | Prothrombin time (patient) |
| `pt_control_s` | number, s | Prothrombin time (control) |
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |

## `glasgow_ah`: Glasgow alcoholic hepatitis score

Source: Forrest 2005 (Gut)

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `wbc_10e9_l` | number, ×10⁹/L | White cells |
| `urea_mmol_l` | number, mmol/L | Urea |
| `inr` | number | INR or PT ratio |
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |

## `saag`: SAAG

Source: Book Lecture 9, p. 150–151

| input | kind | |
|---|---|---|
| `serum_albumin_g_dl` | number, g/dL | Serum albumin |
| `ascitic_albumin_g_dl` | number, g/dL | Ascitic albumin |
| `ascitic_protein_g_dl` | number, g/dL | Ascitic total protein; optional |

## `ascitic_pmn`: Ascitic PMN / SBP

Source: Book Lecture 9, p. 167–168

| input | kind | |
|---|---|---|
| `pmn_per_mm3` | number, /mm³ | Ascitic PMN (neutrophils) |
| `culture` | `pending`, `negative`, `monomicrobial`, `polymicrobial` | Ascitic culture |
| `protein_g_dl` | number, g/dL | Ascitic total protein; optional |
| `glucose_mg_dl` | number, mg/dL | Ascitic glucose; optional |
| `ldh_above_serum` | true / false (default false) | Ascitic LDH above serum LDH |

## `sos_criteria`: Seattle / Baltimore criteria (SOS)

Source: McDonald 1984, Jones 1987, as the book merges them, Lecture 4, p. 81

| input | kind | |
|---|---|---|
| `weight_gain` | true / false (default false) | Weight gain |
| `ascites` | true / false (default false) | Ascites |
| `tender_hepatomegaly` | true / false (default false) | Tender hepatomegaly |
| `jaundice` | true / false (default false) | Jaundice |
| `days_since_transplant` | number, days | Days since the transplant |

## `cha2ds2_vasc`: CHA₂DS₂-VASc

Source: Lip 2010, ESC 2020; book Lecture 4, p. 87 (thrombotic risk for endoscopy, ASGE/ACG 2022)

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `sex` | `male`, `female` | Sex |
| `heart_failure` | true / false (default false) | Heart failure |
| `hypertension` | true / false (default false) | Hypertension |
| `diabetes` | true / false (default false) | Diabetes |
| `stroke_tia` | true / false (default false) | Previous stroke, TIA or thromboembolism |
| `vascular` | true / false (default false) | Vascular disease (MI, PAD, aortic plaque) |

## `bclc`: BCLC staging

Source: Book Lecture 18, p. 362 and 364–373 (BCLC 2018)

| input | kind | |
|---|---|---|
| `performance_status` | number | ECOG performance status |
| `child_pugh_points` | number | Child-Pugh points |
| `ascites` | true / false (default false) | Ascites present |
| `nodules` | number | Number of nodules |
| `largest_cm` | number, cm | Largest nodule |
| `vascular_invasion` | true / false (default false) | Macrovascular invasion |
| `extrahepatic` | true / false (default false) | Extrahepatic spread |

## `milan`: Milan criteria

Source: Mazzaferro 1996; book Lecture 18, p. 368 (adds PS ≤ 1) and p. 373

| input | kind | |
|---|---|---|
| `nodules` | number | Number of lesions |
| `largest_cm` | number, cm | Largest lesion |
| `vascular_invasion` | true / false (default false) | Gross vascular invasion |
| `metastasis` | true / false (default false) | Nodal or distant metastasis |
| `performance_status` | number | ECOG performance status |

## `okuda`: Okuda stage

Source: Okuda 1985 (the book names it, Lecture 18, p. 361 and 355, without the criteria)

| input | kind | |
|---|---|---|
| `tumour_over_half` | true / false (default false) | Tumour > 50% of the liver |
| `ascites` | true / false (default false) | Ascites |
| `albumin_g_dl` | number, g/dL | Albumin |
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin |

## `page_b`: PAGE-B

Source: Papatheodoridis 2016; book Lecture 18, p. 351–352 (platelet points from the paper, see note)

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `sex` | `male`, `female` | Sex |
| `platelets_10e9_l` | number, ×10⁹/L | Platelets |

## `kings_college`: King's College criteria

Source: Book Lecture 10, p. 187 (O'Grady 1989; Bernal 2002)

| input | kind | |
|---|---|---|
| `cause` | `acetaminophen`, `wilson_budd_chiari`, `indeterminate_or_drug`, `other` | Cause of ALF |
| `inr` | number | INR |
| `ph` | number | Arterial pH (after resuscitation); optional |
| `lactate_mmol_l` | number, mmol/L | Lactate; optional |
| `creatinine_mg_dl` | number, mg/dL | Creatinine; optional |
| `he_stage` | number | Encephalopathy stage; optional |
| `bilirubin_mg_dl` | number, mg/dL | Bilirubin; optional |
| `age_years` | number, years | Age; optional |
| `jaundice_to_he_days` | number, days | Days from jaundice to encephalopathy; optional |
| `over_24h` | true / false (default false) | More than 24 h since ingestion |

## `clichy`: Clichy criteria

Source: Book Lecture 10, p. 187 (Bernuau 1986)

| input | kind | |
|---|---|---|
| `he_stage` | number | Encephalopathy stage |
| `factor_v_pct` | number, % of normal | Factor V |
| `age_years` | number, years | Age |

## `ranson`: Ranson's criteria

Source: Ranson 1974/1979; book Lecture 15, p. 288 (mortality bands)

| input | kind | |
|---|---|---|
| `gallstone` | true / false (default false) | Gallstone pancreatitis |
| `age_years` | number, years | Age |
| `wbc_per_mm3` | number, /mm³ | White cells |
| `glucose_mg_dl` | number, mg/dL | Glucose |
| `ldh_iu_l` | number, IU/L | LDH |
| `ast_iu_l` | number, IU/L | AST |
| `hct_fall_points` | number, points | Haematocrit fall in 48 h; optional |
| `bun_rise_mg_dl` | number, mg/dL | BUN rise in 48 h; optional |
| `calcium_mg_dl` | number, mg/dL | Calcium at 48 h; optional |
| `pao2_mm_hg` | number, mmHg | PaO₂ at 48 h; optional |
| `base_deficit_meq_l` | number, mEq/L | Base deficit at 48 h; optional |
| `fluid_sequestration_l` | number, L | Fluid sequestration in 48 h; optional |

## `bisap`: BISAP

Source: Wu 2008; book Lecture 15, p. 288

| input | kind | |
|---|---|---|
| `bun_mg_dl` | number, mg/dL | BUN |
| `impaired_mental_status` | true / false (default false) | Impaired mental status |
| `sirs` | true / false (default false) | SIRS (2 or more criteria) |
| `age_years` | number, years | Age |
| `pleural_effusion` | true / false (default false) | Pleural effusion |

## `ctsi`: CT severity index (Balthazar)

Source: Balthazar 1990; book Lecture 15, table p. 282 and p. 283

| input | kind | |
|---|---|---|
| `ct_grade` | `a`, `b`, `c`, `d`, `e` | Balthazar CT grade |
| `necrosis` | `none`, `lt30`, `30_50`, `gt50` | Pancreatic necrosis |

## `atlanta_2012`: Revised Atlanta severity (2012)

Source: Banks 2013 (Atlanta 2012); book Lecture 15, table p. 285

| input | kind | |
|---|---|---|
| `organ_failure` | `none`, `transient`, `persistent` | Organ failure |
| `complications` | true / false (default false) | Local or systemic complications |

## `glasgow_blatchford`: Glasgow-Blatchford score

Source: Blatchford 2000; book Gastroenterology Lecture 11, p. 309

| input | kind | |
|---|---|---|
| `bun_mg_dl` | number, mg/dL | BUN |
| `sex` | `male`, `female` | Sex |
| `hb_g_dl` | number, g/dL | Haemoglobin |
| `sbp_mmhg` | number, mmHg | Systolic blood pressure |
| `hr_bpm` | number, beats/min | Heart rate |
| `melena` | true / false (default false) | Melena |
| `syncope` | true / false (default false) | Syncope |
| `hepatic_disease` | true / false (default false) | Hepatic disease |
| `cardiac_failure` | true / false (default false) | Cardiac failure |

## `shock_index`: Shock index

Source: Book Gastroenterology Lecture 7, p. 227

| input | kind | |
|---|---|---|
| `hr_bpm` | number, beats/min | Heart rate |
| `sbp_mmhg` | number, mmHg | Systolic blood pressure |

## `oakland`: Oakland score (lower GI bleeding)

Source: Oakland 2017; book Gastroenterology Lecture 7, Table 1 (pp. 228–229)

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `sex` | `male`, `female` | Sex |
| `previous_lgib` | true / false (default false) | Previous admission with lower GI bleeding |
| `dre_blood` | true / false (default false) | Blood on digital rectal examination |
| `hr_bpm` | number, beats/min | Heart rate |
| `sbp_mmhg` | number, mmHg | Systolic blood pressure |
| `hb_g_dl` | number, g/dL | Haemoglobin |

## `uc_activity`: UC activity (Truelove and Witts)

Source: Book Gastroenterology Lecture 6, Table 4 (p. 222), titled there "Mayo score"; the grading is Truelove and Witts 1955

| input | kind | |
|---|---|---|
| `stools_per_day` | number | Bowel movements a day |
| `blood` | `small`, `between`, `visible` | Blood in stools |
| `fever` | true / false (default false) | Temperature above 37.8 °C |
| `pulse_90` | true / false (default false) | Pulse 90/min or more |
| `anaemia` | true / false (default false) | Anaemia |
| `esr_mm_h` | number, mm/h | ESR; optional |

## `kyoto_gastritis`: Kyoto classification score (gastritis)

Source: Toyoshima 2020; book Gastroenterology Lecture 1, Table 2 (p. 16) and p. 21

| input | kind | |
|---|---|---|
| `atrophy` | `none`, `mild`, `severe` | Mucosal atrophy (Kimura-Takemoto) |
| `intestinal_metaplasia` | `none`, `mild`, `severe` | Intestinal metaplasia |
| `enlarged_folds` | true / false (default false) | Enlarged folds (5 mm or wider) |
| `nodularity` | true / false (default false) | Nodularity (small nodules in the antrum) |
| `diffuse_redness` | `none`, `mild`, `severe` | Diffuse redness |

## `rome4_ibs`: Rome IV criteria for IBS

Source: Book Gastroenterology Lecture 3, pp. 125–126 (Rome IV, 2016)

| input | kind | |
|---|---|---|
| `pain_weekly` | true / false (default false) | Recurrent abdominal pain, on average 1 day a week or more, in the last 3 months |
| `onset_6_months` | true / false (default false) | Symptoms began 6 months or more before diagnosis |
| `defecation` | true / false (default false) | Pain related to defecation |
| `frequency` | true / false (default false) | Pain with a change in stool frequency |
| `form` | true / false (default false) | Pain with a change in stool form |
| `hard_25` | true / false (default false) | Bristol 1–2 in more than 25% of abnormal bowel movements |
| `loose_25` | true / false (default false) | Bristol 6–7 in more than 25% of abnormal bowel movements |

## `anion_gap`: Anion gap

Source: Book Nephrology Lecture 1, pp. 5, 9, 14

| input | kind | |
|---|---|---|
| `na_meq_l` | number, mEq/L | Sodium |
| `cl_meq_l` | number, mEq/L | Chloride |
| `hco3_meq_l` | number, mEq/L | Bicarbonate (HCO3) |
| `albumin_g_dl` | number, g/dL | Albumin; optional |

## `delta_ratio`: Delta ratio (gap : gap)

Source: Book Nephrology Lecture 1, pp. 12, 14

| input | kind | |
|---|---|---|
| `anion_gap_meq_l` | number, mEq/L | Anion gap |
| `hco3_meq_l` | number, mEq/L | Bicarbonate (HCO3) |

## `expected_compensation`: Expected compensation

Source: Book Nephrology Lecture 1, pp. 13–14

| input | kind | |
|---|---|---|
| `disorder` | `metabolic_acidosis`, `metabolic_alkalosis`, `respiratory_acidosis_acute`, `respiratory_acidosis_chronic`, `respiratory_alkalosis_acute`, `respiratory_alkalosis_chronic` | Primary disorder |
| `hco3_meq_l` | number, mEq/L | Bicarbonate (HCO3) |
| `pco2_mm_hg` | number, mmHg | pCO2 |

## `osmolal_gap`: Osmolal gap

Source: Book Nephrology Lecture 1, p. 20

| input | kind | |
|---|---|---|
| `measured_osm` | number, mOsm/kg | Measured osmolality |
| `na_meq_l` | number, mEq/L | Sodium |
| `glucose_mg_dl` | number, mg/dL | Glucose |
| `bun_mg_dl` | number, mg/dL | BUN |

## `kdigo_aki`: KDIGO AKI stage

Source: KDIGO 2012; book Nephrology Lecture 5, p. 100

| input | kind | |
|---|---|---|
| `baseline_cr_mg_dl` | number, mg/dL | Baseline creatinine |
| `current_cr_mg_dl` | number, mg/dL | Current creatinine |
| `urine_output` | `normal`, `lt05_6h`, `lt05_12h`, `lt03_24h_or_anuria` | Urine output |
| `rrt` | true / false (default false) | Renal replacement therapy started |

## `fena`: FENa (fractional excretion of sodium)

Source: Book Nephrology Lecture 5, p. 109

| input | kind | |
|---|---|---|
| `urine_na_meq_l` | number, mEq/L | Urine sodium |
| `plasma_na_meq_l` | number, mEq/L | Plasma sodium |
| `urine_cr_mg_dl` | number, mg/dL | Urine creatinine |
| `plasma_cr_mg_dl` | number, mg/dL | Plasma creatinine |

## `feurea`: FEurea (fractional excretion of urea)

Source: Book Nephrology Lecture 5, p. 109

| input | kind | |
|---|---|---|
| `urine_urea_mg_dl` | number, mg/dL | Urine urea |
| `plasma_urea_mg_dl` | number, mg/dL | Plasma urea |
| `urine_cr_mg_dl` | number, mg/dL | Urine creatinine |
| `plasma_cr_mg_dl` | number, mg/dL | Plasma creatinine |

## `bun_creatinine`: BUN : creatinine ratio

Source: Book Nephrology Lecture 5, p. 109 (prerenal vs ATN table); no validated cut-off study, so read it with the FENa

| input | kind | |
|---|---|---|
| `bun_mg_dl` | number, mg/dL | BUN |
| `creatinine_mg_dl` | number, mg/dL | Serum creatinine |

## `ckd_epi_2021`: eGFR (CKD-EPI 2021, race-free)

Source: Book Nephrology Lecture 2, p. 29 (worked examples); equation from Inker et al., NEJM 2021 (the book doesn't print it)

| input | kind | |
|---|---|---|
| `creatinine_mg_dl` | number, mg/dL | Serum creatinine |
| `age_years` | number, years | Age |
| `sex` | `male`, `female` | Sex |

## `kdigo_ckd`: KDIGO CKD stage (G and A)

Source: KDIGO 2012 CKD guideline (Kidney Int Suppl 2013); book Nephrology Lecture 2, pp. 24, 31–32

| input | kind | |
|---|---|---|
| `egfr` | number, mL/min/1.73 m² | eGFR |
| `acr_mg_g` | number, mg/g | Urine albumin : creatinine ratio (ACR) |

## `aa_gradient`: A-a gradient

Source: Book Nephrology Lecture 1, ABG table (p. 14): alveolar gas equation, expected gradient age/4 + 4 (room air)

| input | kind | |
|---|---|---|
| `fio2` | number, fraction | FiO2 |
| `pao2_mm_hg` | number, mmHg | PaO2 |
| `pco2_mm_hg` | number, mmHg | PaCO2 |
| `age_years` | number, years | Age |

## `free_water_deficit`: Free water deficit (hypernatraemia)

Source: Book Nephrology Lecture 6, pp. 133–135 (replacement volume and pace); deficit = TBW × (Na/140 − 1) and the TBW fractions from Adrogué & Madias, NEJM 2000 (the book doesn't print them)

| input | kind | |
|---|---|---|
| `na_meq_l` | number, mEq/L | Serum sodium |
| `weight_kg` | number, kg | Weight |
| `sex` | `male`, `female` | Sex |
| `elderly` | true / false (default false) | Elderly (65 or over) |
| `fluid` | `water`, `saline_0_2`, `saline_0_45` | Replacement fluid |

## `plasmic`: PLASMIC score (TTP)

Source: Bendapudi et al., Lancet Haematol 2017 (derivation and validation); book Nephrology Lecture 14, pp. 466, 479–480

| input | kind | |
|---|---|---|
| `platelets_10e9_l` | number, × 10⁹/L | Platelets |
| `hemolysis` | true / false (default false) | Haemolysis |
| `mcv_fl` | number, fL | MCV |
| `inr` | number | INR |
| `creatinine_mg_dl` | number, mg/dL | Serum creatinine |
| `active_cancer` | true / false (default false) | Active cancer |
| `transplant` | true / false (default false) | Solid-organ or stem-cell transplant |

## `cockcroft_gault`: Cockcroft-Gault creatinine clearance

Source: Book Geriatrics Lecture 2, p. 30; Lecture 3, p. 42

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `weight_kg` | number, kg | Weight |
| `creatinine_mg_dl` | number, mg/dL | Serum creatinine |
| `sex` | `male`, `female` | Sex |

## `katz_adl`: Katz ADL index

Source: Book Geriatrics Lecture 7, Katz Index of Independence in ADL (p. 111); Katz et al., JAMA 1963;185:914–9

| input | kind | |
|---|---|---|
| `bathing` | `independent`, `dependent` | Bathing |
| `dressing` | `independent`, `dependent` | Dressing |
| `toileting` | `independent`, `dependent` | Toileting |
| `transferring` | `independent`, `dependent` | Transferring |
| `continence` | `independent`, `dependent` | Continence |
| `feeding` | `independent`, `dependent` | Feeding |

## `lawton_iadl`: Lawton IADL scale

Source: Book Geriatrics Lecture 7, Lawton-Brody IADL scale (p. 112); Lawton & Brody, Gerontologist 1969;9:179–86, also the source of the domains left out for men (food preparation, housekeeping, laundry)

| input | kind | |
|---|---|---|
| `sex` | `female`, `male` | Sex |
| `telephone` | `own_initiative`, `few_numbers`, `answers_only`, `none` | Ability to use telephone |
| `shopping` | `all`, `small`, `accompanied`, `unable` | Shopping |
| `food_preparation` | `independent`, `if_supplied`, `inadequate`, `needs_served` | Food preparation; optional |
| `housekeeping` | `alone`, `light`, `light_poor`, `needs_help`, `none` | Housekeeping; optional |
| `laundry` | `all`, `small_items`, `others` | Laundry; optional |
| `transport` | `independent`, `taxi`, `accompanied`, `assisted`, `none` | Mode of transportation |
| `medication` | `responsible`, `if_prepared`, `unable` | Responsibility for own medications |
| `finances` | `independent`, `day_to_day`, `unable` | Ability to handle finances |

## `gds15`: Geriatric Depression Scale (GDS-15)

Source: Book Geriatrics Lecture 7, Geriatric Depression Scale short form (p. 119); Lecture 4 (pp. 79–80); Sheikh & Yesavage, Clin Gerontol 1986;5:165–73

| input | kind | |
|---|---|---|
| `satisfied` | `yes`, `no` | 1. Basically satisfied with your life? |
| `dropped_activities` | `yes`, `no` | 2. Dropped many of your activities and interests? |
| `life_empty` | `yes`, `no` | 3. Feel that your life is empty? |
| `bored` | `yes`, `no` | 4. Often get bored? |
| `good_spirits` | `yes`, `no` | 5. In good spirits most of the time? |
| `afraid` | `yes`, `no` | 6. Afraid that something bad is going to happen to you? |
| `happy` | `yes`, `no` | 7. Feel happy most of the time? |
| `helpless` | `yes`, `no` | 8. Often feel helpless? |
| `stay_home` | `yes`, `no` | 9. Prefer to stay at home rather than go out and do new things? |
| `memory` | `yes`, `no` | 10. More problems with memory than most people? |
| `wonderful_alive` | `yes`, `no` | 11. Think it is wonderful to be alive? |
| `worthless` | `yes`, `no` | 12. Feel pretty worthless the way you are now? |
| `full_of_energy` | `yes`, `no` | 13. Feel full of energy? |
| `hopeless` | `yes`, `no` | 14. Feel that your situation is hopeless? |
| `others_better_off` | `yes`, `no` | 15. Think that most people are better off than you are? |

## `mna_sf`: MNA screening (short form)

Source: Book Geriatrics Lecture 7, Mini Nutritional Assessment, screening items A–F (p. 124); Rubenstein et al., J Gerontol 2001;56A:M366–72; Kaiser et al., J Nutr Health Aging 2009;13:782–8

| input | kind | |
|---|---|---|
| `food_intake` | `severe`, `moderate`, `none` | A. Food intake fall over 3 months (appetite, digestion, chewing, swallowing) |
| `weight_loss` | `over_3kg`, `unknown`, `1_3kg`, `none` | B. Weight loss over the last 3 months |
| `mobility` | `bed_chair`, `indoors`, `goes_out` | C. Mobility |
| `stress` | `yes`, `no` | D. Psychological stress or acute disease in the past 3 months |
| `neuropsych` | `severe`, `mild_dementia`, `none` | E. Neuropsychological problems |
| `bmi` | `under_19`, `19_21`, `21_23`, `23_plus` | F. Body mass index (kg/m²) |

## `clinical_frailty`: Clinical Frailty Scale

Source: Book Geriatrics Lecture 7, Frailty assessment, Clinical Frailty Scale (p. 121); Rockwood et al., CMAJ 2005;173:489–95

| input | kind | |
|---|---|---|
| `level` | `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`, `9` | The description that fits best |

## `cam`: Confusion Assessment Method (CAM)

Source: Book Geriatrics Lecture 4, Confusion Assessment Method (pp. 70–71); Inouye et al., Ann Intern Med 1990;113:941–8

| input | kind | |
|---|---|---|
| `acute_fluctuating` | true / false (default false) | 1. Acute onset and fluctuating course |
| `inattention` | true / false (default false) | 2. Inattention |
| `disorganised_thinking` | true / false (default false) | 3. Disorganised thinking |
| `altered_consciousness` | true / false (default false) | 4. Altered level of consciousness |

## `sarc_f`: SARC-F (sarcopenia screen)

Source: Book Geriatrics Lecture 8, Sarcopenia, SARC-F questionnaire (pp. 133–134); Malmstrom & Morley, J Am Med Dir Assoc 2013;14:531–2 (falls 1–3 = 1 point); next steps from EWGSOP2, Age Ageing 2019;48:16–31

| input | kind | |
|---|---|---|
| `strength` | `none`, `some`, `a_lot` | Strength: difficulty lifting and carrying 10 pounds |
| `walking` | `none`, `some`, `a_lot` | Assistance in walking: difficulty walking across a room |
| `rise` | `none`, `some`, `a_lot` | Rise from a chair: difficulty transferring from a chair or bed |
| `stairs` | `none`, `some`, `a_lot` | Climb stairs: difficulty climbing a flight of 10 stairs |
| `falls` | `none`, `1_3`, `4_plus` | Falls in the past year |

## `mmse`: Mini-Mental State Examination (MMSE)

Source: Book Geriatrics Lecture 4, Mini-Mental State Examination (p. 63); Folstein et al., J Psychiatr Res 1975;12:189–98

| input | kind | |
|---|---|---|
| `orientation_time` | number | Orientation: year, season, date, day, month |
| `orientation_place` | number | Orientation: country, county, town, hospital, floor |
| `registration` | number | Registration: name 3 objects |
| `attention` | number | Attention: serial 7s or WORLD backwards |
| `recall` | number | Recall the 3 objects |
| `naming` | number | Language: name a pencil and a watch |
| `repetition` | number | Language: repeat "no ifs, ands, or buts" |
| `command` | number | Language: 3-stage command |
| `read_write_copy` | number | Language: read and obey, write a sentence, copy a design |

## `abi`: Ankle-brachial index (ABI)

Source: Book Geriatrics Lecture 1, Subclinical cardiovascular disease (p. 8); Book Diabetes Lecture 5, Ankle brachial pressure index (p. 60); AHA/ACC 2016 PAD guideline (Gerhard-Herman et al., Circulation 2017;135:e726–79) for 0.91–0.99 borderline and above 1.40 non-compressible

| input | kind | |
|---|---|---|
| `dp_mm_hg` | number, mm Hg | Ankle systolic: dorsalis pedis |
| `pt_mm_hg` | number, mm Hg | Ankle systolic: posterior tibial; optional |
| `brachial_right_mm_hg` | number, mm Hg | Brachial systolic: right arm |
| `brachial_left_mm_hg` | number, mm Hg | Brachial systolic: left arm; optional |

## `acr_eular_2010_ra`: ACR/EULAR 2010 RA criteria

Source: Book Rheumatology Lecture 16, p. 246

| input | kind | |
|---|---|---|
| `large_joints` | number | Medium-large joints involved |
| `small_joints` | number | Small joints involved |
| `serology` | `negative`, `low`, `high` | RF / ACPA |
| `synovitis_6_weeks` | true / false (default false) | Synovitis 6 weeks or longer |
| `abnormal_crp_esr` | true / false (default false) | Abnormal CRP or ESR |

## `caspar`: CASPAR (psoriatic arthritis)

Source: Taylor et al., Arthritis Rheum 2006 (CASPAR); book Rheumatology Lecture 20, p. 334

| input | kind | |
|---|---|---|
| `inflammatory_disease` | true / false (default false) | Inflammatory articular disease (joint, spine or entheses) |
| `psoriasis` | `current`, `past`, `family`, `none` | Psoriasis |
| `nail_lesions` | true / false (default false) | Nail lesions: onycholysis, pitting, hyperkeratosis |
| `dactylitis` | true / false (default false) | Dactylitis, present or past (documented by a rheumatologist) |
| `rf_negative` | true / false (default false) | Rheumatoid factor negative (any method except latex) |
| `juxta_articular_bone` | true / false (default false) | Juxta-articular new bone formation on X-ray (not osteophytes) |

## `asas_axial_spa`: ASAS axial SpA criteria

Source: Rudwaleit et al., Ann Rheum Dis 2009 (ASAS); book Rheumatology Lecture 17, pp. 287–288 (entry line from the paper)

| input | kind | |
|---|---|---|
| `back_pain_3_months_onset_45` | true / false (default false) | Back pain 3 months or longer, starting before age 45 |
| `sacroiliitis_imaging` | true / false (default false) | Sacroiliitis on imaging |
| `hla_b27` | true / false (default false) | HLA-B27 positive |
| `ibp` | true / false (default false) | Inflammatory back pain |
| `arthritis` | true / false (default false) | Arthritis |
| `enthesitis` | true / false (default false) | Enthesitis (heel) |
| `uveitis` | true / false (default false) | Uveitis |
| `dactylitis` | true / false (default false) | Dactylitis |
| `psoriasis` | true / false (default false) | Psoriasis |
| `ibd` | true / false (default false) | Crohn's disease or ulcerative colitis |
| `nsaid_response` | true / false (default false) | Good response to NSAIDs |
| `family_history` | true / false (default false) | Family history of SpA |
| `elevated_crp` | true / false (default false) | Elevated CRP |

## `asas_peripheral_spa`: ASAS peripheral SpA criteria

Source: Rudwaleit et al., Ann Rheum Dis 2011 (ASAS); book Rheumatology Lecture 17, p. 288

| input | kind | |
|---|---|---|
| `arthritis` | true / false (default false) | Arthritis |
| `enthesitis` | true / false (default false) | Enthesitis |
| `dactylitis` | true / false (default false) | Dactylitis |
| `psoriasis` | true / false (default false) | Psoriasis |
| `ibd` | true / false (default false) | Inflammatory bowel disease |
| `preceding_infection` | true / false (default false) | Preceding infection |
| `hla_b27` | true / false (default false) | HLA-B27 positive |
| `uveitis` | true / false (default false) | Uveitis |
| `sacroiliitis_imaging` | true / false (default false) | Sacroiliitis on imaging (X-ray or MRI) |
| `ibp_past` | true / false (default false) | Inflammatory back pain in the past |
| `family_history` | true / false (default false) | Family history of SpA |

## `modified_new_york`: Modified New York criteria (AS)

Source: van der Linden et al., Arthritis Rheum 1984; book Rheumatology Lecture 17, pp. 286–288

| input | kind | |
|---|---|---|
| `inflammatory_back_pain` | true / false (default false) | Low back pain and stiffness 3 months or more, better with exercise, not rest |
| `lumbar_limitation` | true / false (default false) | Limited lumbar motion in sagittal and frontal planes |
| `chest_expansion` | true / false (default false) | Limited chest expansion (for age and sex) |
| `si_grade_right` | number | Sacroiliitis grade, right |
| `si_grade_left` | number | Sacroiliitis grade, left |

## `sle_acr_1997`: SLE 1997 ACR criteria

Source: Hochberg, Arthritis Rheum 1997 (ACR, revising Tan 1982); book Rheumatology Lecture 19, p. 322

| input | kind | |
|---|---|---|
| `malar_rash` | true / false (default false) | Malar rash |
| `discoid_rash` | true / false (default false) | Discoid rash |
| `photosensitivity` | true / false (default false) | Photosensitivity |
| `oral_ulcers` | true / false (default false) | Oral ulcers |
| `arthritis` | true / false (default false) | Arthritis |
| `serositis` | true / false (default false) | Serositis |
| `renal` | true / false (default false) | Renal disorder |
| `neurological` | true / false (default false) | Neurological disorder |
| `haematological` | true / false (default false) | Haematological disorder |
| `immunological` | true / false (default false) | Immunological disorder |
| `ana` | true / false (default false) | ANA positive |

## `sle_eular_acr_2019`: SLE 2019 EULAR/ACR criteria

Source: Aringer et al., Ann Rheum Dis / Arthritis Rheumatol 2019 (EULAR/ACR); book Nephrology Lecture 14, p. 448

| input | kind | |
|---|---|---|
| `ana_1_80` | true / false (default false) | ANA 1:80 or more, ever |
| `fever` | true / false (default false) | Fever |
| `cutaneous` | `none`, `alopecia`, `oral_ulcers`, `subacute_discoid`, `acute_cutaneous` | Skin and mucosa; optional |
| `arthritis` | true / false (default false) | Synovitis or tenderness in 2 or more joints |
| `neurologic` | `none`, `delirium`, `psychosis`, `seizure` | Neurological; optional |
| `serosal` | `none`, `effusion`, `pericarditis` | Serosal; optional |
| `haematologic` | `none`, `leukopenia`, `thrombocytopenia`, `haemolysis` | Haematological; optional |
| `renal` | `none`, `proteinuria`, `class_2_5`, `class_3_4` | Renal; optional |
| `antiphospholipid` | true / false (default false) | Antiphospholipid antibodies |
| `complement` | `none`, `c3_or_c4`, `c3_and_c4` | Complement; optional |
| `specific_antibody` | true / false (default false) | Anti-dsDNA or anti-Sm antibody |

## `gpa_2022`: GPA 2022 ACR/EULAR criteria

Source: Robson et al., Ann Rheum Dis 2022 (ACR/EULAR); book Rheumatology Lecture 8, p. 147

| input | kind | |
|---|---|---|
| `nasal` | true / false (default false) | Nasal bloody discharge, ulcers, crusting, congestion, blockage or septal perforation (+3) |
| `cartilage` | true / false (default false) | Cartilage: ear or nose chondritis, hoarseness, stridor, endobronchial, saddle nose (+2) |
| `hearing_loss` | true / false (default false) | Conductive or sensorineural hearing loss (+1) |
| `c_anca_pr3` | true / false (default false) | c-ANCA or anti-PR3 positive (+5) |
| `lung_nodules` | true / false (default false) | Pulmonary nodules, mass or cavitation on chest imaging (+2) |
| `granuloma` | true / false (default false) | Granuloma, extravascular granulomatous inflammation or giant cells on biopsy (+2) |
| `sinus_imaging` | true / false (default false) | Nasal/paranasal sinus inflammation, consolidation or effusion, or mastoiditis on imaging (+1) |
| `pauci_immune_gn` | true / false (default false) | Pauci-immune glomerulonephritis on biopsy (+1) |
| `p_anca_mpo` | true / false (default false) | p-ANCA or anti-MPO positive (−1) |
| `eosinophils` | true / false (default false) | Blood eosinophils ≥ 1 × 10⁹/L (−4) |

## `mpa_2022`: MPA 2022 ACR/EULAR criteria

Source: Suppiah et al., Ann Rheum Dis 2022 (ACR/EULAR); book Rheumatology Lecture 8, p. 147

| input | kind | |
|---|---|---|
| `nasal` | true / false (default false) | Nasal bloody discharge, ulcers, crusting, congestion, blockage or septal perforation (−3) |
| `p_anca_mpo` | true / false (default false) | p-ANCA or anti-MPO positive (+6) |
| `lung_fibrosis` | true / false (default false) | Fibrosis or interstitial lung disease on chest imaging (+3) |
| `pauci_immune_gn` | true / false (default false) | Pauci-immune glomerulonephritis on biopsy (+3) |
| `c_anca_pr3` | true / false (default false) | c-ANCA or anti-PR3 positive (−1) |
| `eosinophils` | true / false (default false) | Blood eosinophils ≥ 1 × 10⁹/L (−4) |

## `corrected_sodium`: Corrected sodium (hyperglycaemia)

Source: Book Diabetology Lecture 15, p. 189

| input | kind | |
|---|---|---|
| `na_meq_l` | number, mEq/L | Sodium |
| `glucose_mg_dl` | number, mg/dL | Glucose |

## `effective_osmolality`: Serum osmolality (HHS)

Source: Book Diabetology Lecture 15, pp. 189, 192

| input | kind | |
|---|---|---|
| `na_meq_l` | number, mEq/L | Sodium |
| `glucose_mg_dl` | number, mg/dL | Glucose |
| `urea_mmol_l` | number, mmol/L | Urea; optional |

## `dka_severity`: DKA severity

Source: Book Diabetology Lecture 15, Table 2 (p. 188), from the ADA 2024 consensus

| input | kind | |
|---|---|---|
| `ph` | number | Venous or arterial pH |
| `hco3_mmol_l` | number, mmol/L | Bicarbonate |
| `bhb_mmol_l` | number, mmol/L | β-hydroxybutyrate; optional |
| `urine_ketones` | true / false (default false) | Urine ketones 2+ or more |
| `glucose_mg_dl` | number, mg/dL | Glucose; optional |
| `known_diabetes` | true / false (default false) | Known diabetes |
| `mental` | `alert`, `drowsy`, `stupor` | Mental status |

## `premixed_insulin`: Premixed insulin starting dose

Source: Book Diabetology Lecture 10, p. 128

| input | kind | |
|---|---|---|
| `weight_kg` | number, kg | Body weight |

## `bmi`: Body mass index (BMI)

Source: Book Diabetology Lecture 2, pp. 18–22 (ADA); class names WHO

| input | kind | |
|---|---|---|
| `weight_kg` | number, kg | Weight |
| `height_cm` | number, cm | Height |

## `ipss_mf`: IPSS (primary myelofibrosis)

Source: Cervantes 2009; book Hematology Lecture 9, p. 151

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `wbc_10e9_l` | number, × 10⁹/L | WBC |
| `hb_g_dl` | number, g/dL | Haemoglobin |
| `blasts_pct` | number, % | Peripheral blasts |
| `constitutional` | true / false (default false) | Constitutional symptoms |

## `rpi`: Reticulocyte production index (RPI)

Source: Hillman & Finch, Red Cell Manual (formula and maturation times, not printed in the book); book Hematology Lecture 2, p. 14 (RI < 2% / > 2%)

| input | kind | |
|---|---|---|
| `retic_pct` | number, % | Reticulocytes |
| `hct_pct` | number, % | Haematocrit |
| `rbc_10e12_l` | number, × 10⁶/µL | RBC count; optional |

## `rai_cll`: Rai stage (CLL)

Source: Rai 1975, modified Rai 1987, iwCLL 2018; book Hematology Lecture 8, Table 1, p. 129

| input | kind | |
|---|---|---|
| `lymphadenopathy` | true / false (default false) | Lymphadenopathy |
| `organomegaly` | true / false (default false) | Hepatomegaly or splenomegaly |
| `hb_g_dl` | number, g/dL | Haemoglobin |
| `platelets_10e9_l` | number, × 10³/µL | Platelets |

## `binet_cll`: Binet stage (CLL)

Source: Binet 1981, iwCLL 2018; book Hematology Lecture 8, Table 1, p. 129

| input | kind | |
|---|---|---|
| `areas` | number, of 5 | Enlarged lymphoid areas |
| `hb_g_dl` | number, g/dL | Haemoglobin |
| `platelets_10e9_l` | number, × 10³/µL | Platelets |

## `ann_arbor`: Ann Arbor stage (lymphoma)

Source: Carbone 1971, Lugano 2014; book Hematology Lecture 12, pp. 192–193

| input | kind | |
|---|---|---|
| `spread` | `single_group`, `one_side`, `both_sides`, `extralymphatic` | Disease extent |
| `night_sweats` | true / false (default false) | Night sweats |
| `fever` | true / false (default false) | Fever |
| `weight_loss` | true / false (default false) | Weight loss > 10% in 6 months |

## `ipi_nhl`: International Prognostic Index (NHL)

Source: Shipp 1993, NEJM (points and groups, not in the book); book Hematology Lecture 12, p. 193

| input | kind | |
|---|---|---|
| `age_years` | number, years | Age |
| `ldh_high` | true / false (default false) | LDH above normal |
| `ecog_2_plus` | true / false (default false) | ECOG performance status 2 or more |
| `stage_3_4` | true / false (default false) | Ann Arbor stage III or IV |
| `extranodal_2_plus` | true / false (default false) | More than one extranodal site |

## `who_pv`: WHO 2016 criteria: polycythaemia vera

Source: WHO 2016 (Arber 2016); book Hematology Lecture 10, p. 163

| input | kind | |
|---|---|---|
| `sex` | `male`, `female` | Sex |
| `hb_g_dl` | number, g/dL | Haemoglobin |
| `hct_pct` | number, % | Haematocrit; optional |
| `rcm_increased` | true / false (default false) | Increased red cell mass |
| `bm_panmyelosis` | true / false (default false) | Marrow: hypercellular with trilineage growth (panmyelosis) |
| `jak2` | true / false (default false) | JAK2 V617F or JAK2 exon 12 mutation |
| `low_epo` | true / false (default false) | Subnormal serum erythropoietin |

## `who_et`: WHO 2016 criteria: essential thrombocythaemia

Source: WHO 2016 (Arber 2016); book Hematology Lecture 9, p. 135

| input | kind | |
|---|---|---|
| `platelets_10e9_l` | number, × 10⁹/L | Platelets |
| `bm_megakaryocytes` | true / false (default false) | Marrow: mainly enlarged, mature, hyperlobulated megakaryocytes |
| `not_other_myeloid` | true / false (default false) | Not meeting WHO criteria for CML, PV, PMF, MDS or another myeloid neoplasm |
| `driver_mutation` | true / false (default false) | JAK2, CALR or MPL mutation |
| `clonal_or_not_reactive` | true / false (default false) | Another clonal marker, or no evidence of reactive thrombocytosis |

## `who_pmf`: WHO 2016 criteria: overt primary myelofibrosis

Source: WHO 2016 (Arber 2016); book Hematology Lecture 9, pp. 145–146

| input | kind | |
|---|---|---|
| `bm_megakaryocytes_fibrosis` | true / false (default false) | Marrow: megakaryocytic proliferation and atypia with grade 2–3 fibrosis |
| `not_other_myeloid` | true / false (default false) | Not meeting WHO criteria for ET, PV, CML, MDS or another myeloid neoplasm |
| `clonal` | true / false (default false) | JAK2, CALR or MPL mutation, or another clonal marker, or no reactive myelofibrosis |
| `anaemia` | true / false (default false) | Anaemia not due to another condition |
| `leukocytosis` | true / false (default false) | Leukocytosis > 11 × 10⁹/L |
| `splenomegaly` | true / false (default false) | Palpable splenomegaly |
| `ldh_high` | true / false (default false) | LDH above the upper limit of normal |
| `leukoerythroblastosis` | true / false (default false) | Leukoerythroblastosis |

## `incidence`: Incidence rate

Source: Research L02: measures of disease frequency, slide 25

| input | kind | |
|---|---|---|
| `new_cases` | number | New cases during the period |
| `population_at_risk` | number | Population at risk (mid-period) |
| `per` | `100`, `1000`, `10000`, `100000` | Expressed |

## `prevalence`: Prevalence

Source: Research L02: measures of disease frequency, slide 31

| input | kind | |
|---|---|---|
| `existing_cases` | number | Existing (old + new) cases |
| `population` | number | Total population (mid-point or mid-period) |
| `per` | `100`, `1000`, `10000`, `100000` | Expressed |

## `prevalence_id`: Prevalence = Incidence × Duration

Source: Research L02: measures of disease frequency, slide 37

| input | kind | |
|---|---|---|
| `incidence` | number, per 1,000 per year | Incidence |
| `duration_years` | number, years | Mean duration of disease |

## `attack_rate`: Attack rate

Source: Research L02: measures of disease frequency, slides 26–29

| input | kind | |
|---|---|---|
| `cases` | number | Cases during the outbreak |
| `at_risk` | number | Population at risk |

## `sar`: Secondary attack rate

Source: Research L02: measures of disease frequency, slides 46–49

| input | kind | |
|---|---|---|
| `secondary_cases` | number | Secondary cases (within one incubation period) |
| `contacts` | number | Contacts (household members, all) |
| `primary_cases` | number | Primary cases among them |

## `cfr`: Case fatality rate

Source: Research L02: measures of disease frequency, slide 60

| input | kind | |
|---|---|---|
| `deaths` | number | Deaths from the disease |
| `cases` | number | People with the disease |

## `pmr`: Proportionate mortality ratio

Source: Research L02: measures of disease frequency, slide 59

| input | kind | |
|---|---|---|
| `cause_deaths` | number | Deaths from the cause |
| `all_deaths` | number | Deaths from all causes |

## `imr`: Infant mortality rate

Source: Research L02: measures of disease frequency, slide 63

| input | kind | |
|---|---|---|
| `infant_deaths` | number | Deaths under one year |
| `live_births` | number | Live births, same year |

## `nmr`: Neonatal mortality rate

Source: Research L02: measures of disease frequency, slide 64

| input | kind | |
|---|---|---|
| `neonatal_deaths` | number | Deaths in the first 28 days |
| `live_births` | number | Live births, same year |

## `relative_risk`: Relative risk (cohort)

Source: Research L05: analytic studies, slides 50–53

| input | kind | |
|---|---|---|
| `a` | number | a |
| `b` | number | b |
| `c` | number | c |
| `d` | number | d |

## `odds_ratio`: Odds ratio (case-control)

Source: Research L05: analytic studies, slides 32–34

| input | kind | |
|---|---|---|
| `a` | number | a |
| `b` | number | b |
| `c` | number | c |
| `d` | number | d |

## `attributable_risk`: Attributable risk

Source: Research L05: analytic studies, slide 51

| input | kind | |
|---|---|---|
| `incidence_exposed` | number, per 1,000 | Incidence among exposed (Ie) |
| `incidence_unexposed` | number, per 1,000 | Incidence among non-exposed (Io) |

## `nnt`: NNT / NNH

Source: Research L06–L07: experimental studies (NNT = 1/ARR)

| input | kind | |
|---|---|---|
| `control_event_rate` | number, % | Event rate, control group (CER) |
| `experimental_event_rate` | number, % | Event rate, treated group (EER) |

## `diagnostic_accuracy`: Diagnostic accuracy (2×2)

Source: Research L14: measures of diagnostic accuracy

| input | kind | |
|---|---|---|
| `tp` | number | True positives (a) |
| `fp` | number | False positives (b) |
| `fn` | number | False negatives (c) |
| `tn` | number | True negatives (d) |
| `measure` | `sensitivity`, `specificity`, `ppv`, `npv`, `accuracy`, `lr_positive`, `lr_negative` | Answer with |

## `z_score`: Z-score

Source: Research L16: normal distribution and Z-score

| input | kind | |
|---|---|---|
| `value` | number | Observation (x) |
| `mean` | number | Mean (μ) |
| `sd` | number | Standard deviation (σ) |
