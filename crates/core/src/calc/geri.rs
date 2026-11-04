//! The Geriatrics book's assessment scales: Katz ADL, Lawton IADL, GDS-15,
//! MNA short form, Clinical Frailty Scale, CAM, SARC-F, MMSE and the
//! ankle-brachial index.
//!
//! Source: the Geriatrics book (Chapter 7).
//! - Katz Index of Independence in ADL, Lecture 7 p. 111: six activities,
//!   1 point each when independent; 6 = high (independent), 0 = low (very
//!   dependent).
//! - Lawton-Brody IADL, Lecture 7 p. 112: eight domains scored 0 or 1 by the
//!   printed level; 0–8 for women and 0–5 for men "to avoid potential gender
//!   bias". The book does not say which three domains are dropped for men;
//!   the original scale (Lawton & Brody 1969) drops food preparation,
//!   housekeeping and laundry, and so does this calculator.
//! - Geriatric Depression Scale, short form (Sheikh & Yesavage 1986),
//!   Lecture 7 p. 119 (also Lecture 4 pp. 79–80): one point per answer in
//!   the depressive direction; 0–5 normal, above 5 suggests depression.
//! - Mini Nutritional Assessment screening (items A–F), Lecture 7 p. 124:
//!   12–14 normal nutritional status, 8–11 at risk, 0–7 malnourished; 11 or
//!   less goes on to the full assessment (G–R).
//! - Clinical Frailty Scale, Lecture 7 p. 121: levels 1–9 with the book's
//!   names; in dementia the degree of frailty follows the degree of dementia.
//! - Confusion Assessment Method, Lecture 4 pp. 70–71: features 1 (acute
//!   onset and fluctuating course) and 2 (inattention), plus 3 (disorganised
//!   thinking) or 4 (altered level of consciousness).
//! - SARC-F, Lecture 8 pp. 133–134: five items 0–2; a score of 4 or more
//!   best predicts the need for fuller evaluation.
//! - MMSE (Folstein 1975), Lecture 4 p. 63: section scores to 30; 29–30
//!   normal, 26–28 borderline, 18–25 marked dysfunction (may be dementia),
//!   below 17 severe.
//! - Ankle-brachial index: Lecture 1 p. 8 (normal above 0.9–0.95) and the
//!   Diabetes book, Lecture 5 p. 60: the higher ankle pressure (dorsalis
//!   pedis or posterior tibial) over the higher brachial pressure; above 0.9
//!   normal, below 0.4 rest pain and tissue necrosis.

use super::{Area, CalcError, CalcSpec, Field, Guide, Inputs, Outcome, Part, choice, flag, num, show};

/// The points of the option picked for `key`: `options` and `points` are in
/// the same order.
fn scored(
    i: &Inputs,
    key: &'static str,
    options: &[(&'static str, &'static str)],
    points: &[f64],
) -> Result<f64, CalcError> {
    let ids: Vec<&str> = options.iter().map(|(id, _)| *id).collect();
    let picked = i.get_choice(key, &ids)?;
    Ok(ids.iter().position(|id| *id == picked).map_or(0.0, |n| points[n]))
}

fn whole(v: f64) -> String {
    show(v, 0)
}

// ---------------------------------------------------------------- Katz ADL

const BATHING: &[(&str, &str)] = &[
    ("independent", "Bathes self, or needs help with one part only"),
    ("dependent", "Needs help with more than one part, or total bathing"),
];
const DRESSING: &[(&str, &str)] = &[
    ("independent", "Gets clothes and dresses fully (help tying shoes is fine)"),
    ("dependent", "Needs help dressing, or is dressed completely"),
];
const TOILETING: &[(&str, &str)] = &[
    ("independent", "Goes to the toilet, manages clothes and cleans self"),
    ("dependent", "Needs help, or uses a bedpan or commode"),
];
const TRANSFERRING: &[(&str, &str)] = &[
    ("independent", "Moves in and out of bed or chair unassisted (aids are fine)"),
    ("dependent", "Needs help from bed to chair, or a complete transfer"),
];
const CONTINENCE: &[(&str, &str)] = &[
    ("independent", "Complete control of urination and defecation"),
    ("dependent", "Partly or totally incontinent of bowel or bladder"),
];
const FEEDING: &[(&str, &str)] = &[
    ("independent", "Gets food from plate to mouth (preparation by others is fine)"),
    ("dependent", "Needs partial or total help, or parenteral feeding"),
];
const ONE_ZERO: &[f64] = &[1.0, 0.0];

const KATZ_ITEMS: &[(&str, &str)] = &[
    ("bathing", "Bathing"),
    ("dressing", "Dressing"),
    ("toileting", "Toileting"),
    ("transferring", "Transferring"),
    ("continence", "Continence"),
    ("feeding", "Feeding"),
];

pub const KATZ: CalcSpec = CalcSpec {
    id: "katz_adl",
    name: "Katz ADL index",
    area: Area::Elderly,
    fields: &[
        choice("bathing", "Bathing", BATHING),
        choice("dressing", "Dressing", DRESSING),
        choice("toileting", "Toileting", TOILETING),
        choice("transferring", "Transferring", TRANSFERRING),
        choice("continence", "Continence", CONTINENCE),
        choice("feeding", "Feeding", FEEDING),
    ],
    source: "Book Geriatrics Lecture 7, Katz Index of Independence in ADL (p. 111); Katz et al., JAMA 1963;185:914–9",
    compute: katz,
};

fn katz(i: &Inputs) -> Result<Outcome, CalcError> {
    let options = [BATHING, DRESSING, TOILETING, TRANSFERRING, CONTINENCE, FEEDING];
    let mut parts = Vec::new();
    for ((key, label), opts) in KATZ_ITEMS.iter().zip(options) {
        parts.push(Part::new(format!("{label}: independent"), scored(i, key, opts, ONE_ZERO)?));
    }
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total >= 6.0 {
        (
            "katz_adl.independent",
            "6: independent",
            "High: independent in all six basic activities.".to_owned(),
        )
    } else if total <= 0.0 {
        (
            "katz_adl.very_dependent",
            "0: very dependent",
            "Low: dependent in all six basic activities.".to_owned(),
        )
    } else {
        (
            "katz_adl.partial",
            "1–5: partly dependent",
            format!("Dependent in {} of the six basic activities.", whole(6.0 - total)),
        )
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / 6", whole(total)),
        band,
        label,
        meaning,
        page: Some(111),
        parts,
        notes: Vec::new(),
    })
}

// ------------------------------------------------------------- Lawton IADL

const SEX: &[(&str, &str)] = &[("female", "Woman"), ("male", "Man")];
const TELEPHONE: &[(&str, &str)] = &[
    ("own_initiative", "Uses the telephone on own initiative, looks up and dials numbers"),
    ("few_numbers", "Dials a few well-known numbers"),
    ("answers_only", "Answers the telephone but does not dial"),
    ("none", "Does not use the telephone at all"),
];
const SHOPPING: &[(&str, &str)] = &[
    ("all", "Takes care of all shopping independently"),
    ("small", "Shops independently for small purchases only"),
    ("accompanied", "Needs to be accompanied on any shopping trip"),
    ("unable", "Completely unable to shop"),
];
const FOOD: &[(&str, &str)] = &[
    ("independent", "Plans, prepares and serves adequate meals independently"),
    ("if_supplied", "Prepares adequate meals if supplied with ingredients"),
    ("inadequate", "Heats or prepares meals but does not keep an adequate diet"),
    ("needs_served", "Needs meals prepared and served"),
];
const HOUSEKEEPING: &[(&str, &str)] = &[
    ("alone", "Maintains the house alone or with occasional heavy-work help"),
    ("light", "Performs light daily tasks (dishes, bed making)"),
    ("light_poor", "Light daily tasks, but not an acceptable level of cleanliness"),
    ("needs_help", "Needs help with all home maintenance tasks"),
    ("none", "Does not take part in any housekeeping"),
];
const LAUNDRY: &[(&str, &str)] = &[
    ("all", "Does personal laundry completely"),
    ("small_items", "Launders small items only"),
    ("others", "All laundry must be done by others"),
];
const TRANSPORT: &[(&str, &str)] = &[
    ("independent", "Travels independently on public transport or drives own car"),
    ("taxi", "Arranges own travel by taxi, but no other public transport"),
    ("accompanied", "Travels on public transport when accompanied"),
    ("assisted", "Travel limited to taxi or car with someone's help"),
    ("none", "Does not travel at all"),
];
const MEDICATION: &[(&str, &str)] = &[
    ("responsible", "Takes medication in correct doses at correct times"),
    ("if_prepared", "Takes it if prepared in advance in separate doses"),
    ("unable", "Is not capable of dispensing own medication"),
];
const FINANCES: &[(&str, &str)] = &[
    ("independent", "Manages financial matters independently"),
    ("day_to_day", "Manages day-to-day purchases, needs help with banking"),
    ("unable", "Incapable of handling money"),
];

pub const LAWTON: CalcSpec = CalcSpec {
    id: "lawton_iadl",
    name: "Lawton IADL scale",
    area: Area::Elderly,
    fields: &[
        choice("sex", "Sex", SEX),
        choice("telephone", "Ability to use telephone", TELEPHONE),
        choice("shopping", "Shopping", SHOPPING),
        choice("food_preparation", "Food preparation", FOOD)
            .optional()
            .hint("Scored for women only (0–8); men are scored out of 5."),
        choice("housekeeping", "Housekeeping", HOUSEKEEPING)
            .optional()
            .hint("Scored for women only (0–8); men are scored out of 5."),
        choice("laundry", "Laundry", LAUNDRY)
            .optional()
            .hint("Scored for women only (0–8); men are scored out of 5."),
        choice("transport", "Mode of transportation", TRANSPORT),
        choice("medication", "Responsibility for own medications", MEDICATION),
        choice("finances", "Ability to handle finances", FINANCES),
    ],
    source: "Book Geriatrics Lecture 7, Lawton-Brody IADL scale (p. 112); Lawton & Brody, Gerontologist \
             1969;9:179–86, also the source of the domains left out for men (food preparation, housekeeping, \
             laundry)",
    compute: lawton,
};

fn lawton(i: &Inputs) -> Result<Outcome, CalcError> {
    let woman = i.get_choice("sex", &["female", "male"])? == "female";
    let mut parts = vec![
        Part::new("Telephone", scored(i, "telephone", TELEPHONE, &[1.0, 1.0, 1.0, 0.0])?),
        Part::new("Shopping", scored(i, "shopping", SHOPPING, &[1.0, 0.0, 0.0, 0.0])?),
    ];
    if woman {
        parts
            .push(Part::new("Food preparation", scored(i, "food_preparation", FOOD, &[1.0, 0.0, 0.0, 0.0])?));
        parts.push(Part::new(
            "Housekeeping",
            scored(i, "housekeeping", HOUSEKEEPING, &[1.0, 1.0, 1.0, 1.0, 0.0])?,
        ));
        parts.push(Part::new("Laundry", scored(i, "laundry", LAUNDRY, &[1.0, 1.0, 0.0])?));
    }
    parts.push(Part::new("Transportation", scored(i, "transport", TRANSPORT, &[1.0, 1.0, 1.0, 0.0, 0.0])?));
    parts.push(Part::new("Medications", scored(i, "medication", MEDICATION, &[1.0, 0.0, 0.0])?));
    parts.push(Part::new("Finances", scored(i, "finances", FINANCES, &[1.0, 1.0, 0.0])?));
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let max = if woman { 8.0 } else { 5.0 };
    let mut notes = Vec::new();
    if !woman && ["food_preparation", "housekeeping", "laundry"].iter().any(|k| i.0.contains_key(*k)) {
        notes.push("Food preparation, housekeeping and laundry are not scored for men.".to_owned());
    }
    let (band, label, meaning) = if total >= max {
        (
            "lawton_iadl.independent",
            "Highest: independent",
            "High function: independent in every scored domain.",
        )
    } else if total <= 0.0 {
        ("lawton_iadl.dependent", "0: dependent", "Low function: dependent in every scored domain.")
    } else {
        (
            "lawton_iadl.partial",
            "Partly dependent",
            "Needs help with some instrumental activities. Compare with earlier scores: a change matters more \
             than the number.",
        )
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / {}", whole(total), whole(max)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(112),
        parts,
        notes,
    })
}

// ----------------------------------------------------------------- GDS-15

const YES_NO: &[(&str, &str)] = &[("yes", "Yes"), ("no", "No")];

const fn q(key: &'static str, label: &'static str) -> Field {
    choice(key, label, YES_NO)
}

/// Each question and the answer that scores a point.
const GDS_ITEMS: &[(&str, &str)] = &[
    ("satisfied", "no"),
    ("dropped_activities", "yes"),
    ("life_empty", "yes"),
    ("bored", "yes"),
    ("good_spirits", "no"),
    ("afraid", "yes"),
    ("happy", "no"),
    ("helpless", "yes"),
    ("stay_home", "yes"),
    ("memory", "yes"),
    ("wonderful_alive", "no"),
    ("worthless", "yes"),
    ("full_of_energy", "no"),
    ("hopeless", "yes"),
    ("others_better_off", "yes"),
];

pub const GDS: CalcSpec = CalcSpec {
    id: "gds15",
    name: "Geriatric Depression Scale (GDS-15)",
    area: Area::Elderly,
    fields: &[
        q("satisfied", "1. Basically satisfied with your life?"),
        q("dropped_activities", "2. Dropped many of your activities and interests?"),
        q("life_empty", "3. Feel that your life is empty?"),
        q("bored", "4. Often get bored?"),
        q("good_spirits", "5. In good spirits most of the time?"),
        q("afraid", "6. Afraid that something bad is going to happen to you?"),
        q("happy", "7. Feel happy most of the time?"),
        q("helpless", "8. Often feel helpless?"),
        q("stay_home", "9. Prefer to stay at home rather than go out and do new things?"),
        q("memory", "10. More problems with memory than most people?"),
        q("wonderful_alive", "11. Think it is wonderful to be alive?"),
        q("worthless", "12. Feel pretty worthless the way you are now?"),
        q("full_of_energy", "13. Feel full of energy?"),
        q("hopeless", "14. Feel that your situation is hopeless?"),
        q("others_better_off", "15. Think that most people are better off than you are?"),
    ],
    source: "Book Geriatrics Lecture 7, Geriatric Depression Scale short form (p. 119); Lecture 4 (pp. 79–80); \
             Sheikh & Yesavage, Clin Gerontol 1986;5:165–73",
    compute: gds,
};

fn gds(i: &Inputs) -> Result<Outcome, CalcError> {
    let mut parts = Vec::new();
    for (n, (key, scores)) in GDS_ITEMS.iter().enumerate() {
        let answer = i.get_choice(key, &["yes", "no"])?;
        parts.push(Part::met(format!("Q{}: answered {answer}", n + 1), answer == *scores));
    }
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total > 5.0 {
        ("gds15.depression", "Above 5", "Suggests depression: assess further.")
    } else {
        ("gds15.normal", "0–5: normal", "Normal: no depression suggested.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / 15", whole(total)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(119),
        parts,
        notes: Vec::new(),
    })
}

// ------------------------------------------------------------ MNA screening

const MNA_A: &[(&str, &str)] = &[
    ("severe", "Severe decrease in food intake"),
    ("moderate", "Moderate decrease in food intake"),
    ("none", "No decrease in food intake"),
];
const MNA_B: &[(&str, &str)] = &[
    ("over_3kg", "Weight loss greater than 3 kg"),
    ("unknown", "Does not know"),
    ("1_3kg", "Weight loss between 1 and 3 kg"),
    ("none", "No weight loss"),
];
const MNA_C: &[(&str, &str)] = &[
    ("bed_chair", "Bed or chair bound"),
    ("indoors", "Gets out of bed or chair but does not go out"),
    ("goes_out", "Goes out"),
];
const MNA_D: &[(&str, &str)] = &[("yes", "Yes"), ("no", "No")];
const MNA_E: &[(&str, &str)] = &[
    ("severe", "Severe dementia or depression"),
    ("mild_dementia", "Mild dementia"),
    ("none", "No psychological problems"),
];
const MNA_F: &[(&str, &str)] = &[
    ("under_19", "BMI less than 19"),
    ("19_21", "BMI 19 to less than 21"),
    ("21_23", "BMI 21 to less than 23"),
    ("23_plus", "BMI 23 or greater"),
];

pub const MNA_SF: CalcSpec = CalcSpec {
    id: "mna_sf",
    name: "MNA screening (short form)",
    area: Area::Elderly,
    fields: &[
        choice(
            "food_intake",
            "A. Food intake fall over 3 months (appetite, digestion, chewing, swallowing)",
            MNA_A,
        ),
        choice("weight_loss", "B. Weight loss over the last 3 months", MNA_B),
        choice("mobility", "C. Mobility", MNA_C),
        choice("stress", "D. Psychological stress or acute disease in the past 3 months", MNA_D),
        choice("neuropsych", "E. Neuropsychological problems", MNA_E),
        choice("bmi", "F. Body mass index (kg/m²)", MNA_F),
    ],
    source: "Book Geriatrics Lecture 7, Mini Nutritional Assessment, screening items A–F (p. 124); Rubenstein et al., \
             J Gerontol 2001;56A:M366–72; Kaiser et al., J Nutr Health Aging 2009;13:782–8",
    compute: mna_sf,
};

fn mna_sf(i: &Inputs) -> Result<Outcome, CalcError> {
    let parts = vec![
        Part::new("A. Food intake", scored(i, "food_intake", MNA_A, &[0.0, 1.0, 2.0])?),
        Part::new("B. Weight loss", scored(i, "weight_loss", MNA_B, &[0.0, 1.0, 2.0, 3.0])?),
        Part::new("C. Mobility", scored(i, "mobility", MNA_C, &[0.0, 1.0, 2.0])?),
        Part::new("D. Stress or acute disease", scored(i, "stress", MNA_D, &[0.0, 2.0])?),
        Part::new("E. Neuropsychological", scored(i, "neuropsych", MNA_E, &[0.0, 1.0, 2.0])?),
        Part::new("F. BMI", scored(i, "bmi", MNA_F, &[0.0, 1.0, 2.0, 3.0])?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total >= 12.0 {
        ("mna_sf.normal", "12–14: normal", "Normal nutritional status.")
    } else if total >= 8.0 {
        (
            "mna_sf.at_risk",
            "8–11: at risk",
            "At risk of malnutrition: go on to the full assessment (items G–R) for the Malnutrition Indicator Score.",
        )
    } else {
        (
            "mna_sf.malnourished",
            "0–7: malnourished",
            "Malnourished: go on to the full assessment (items G–R) for the Malnutrition Indicator Score.",
        )
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / 14", whole(total)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(124),
        parts,
        notes: Vec::new(),
    })
}

// --------------------------------------------------- Clinical Frailty Scale

const CFS_LEVELS: &[(&str, &str)] = &[
    ("1", "1 Very fit: robust, active, energetic, exercises regularly"),
    ("2", "2 Well: no active disease symptoms, less fit than 1"),
    ("3", "3 Managing well: problems well controlled, not regularly active"),
    ("4", "4 Vulnerable: not dependent, but symptoms limit activities"),
    ("5", "5 Mildly frail: needs help with high-order IADLs"),
    ("6", "6 Moderately frail: help with outside activities, housework, bathing"),
    ("7", "7 Severely frail: completely dependent for personal care, stable"),
    ("8", "8 Very severely frail: completely dependent, approaching end of life"),
    ("9", "9 Terminally ill: life expectancy under 6 months, not otherwise frail"),
];

/// (band, name, meaning) per level, in the book's words.
const CFS_BANDS: [(&str, &str, &str); 9] = [
    ("cfs.1", "Very fit", "Robust, active, energetic and motivated; among the fittest for their age."),
    ("cfs.2", "Well", "No active disease symptoms, but less fit than level 1; active occasionally."),
    (
        "cfs.3",
        "Managing well",
        "Medical problems well controlled, but not regularly active beyond routine walking.",
    ),
    (
        "cfs.4",
        "Vulnerable",
        "Not dependent on others for daily help, but symptoms limit activities (\"slowed up\", tired).",
    ),
    (
        "cfs.5",
        "Mildly frail",
        "More evident slowing; needs help with high-order IADLs (finances, transport, heavy housework, \
         medications).",
    ),
    (
        "cfs.6",
        "Moderately frail",
        "Needs help with all outside activities and with keeping house; often help with bathing and stairs.",
    ),
    (
        "cfs.7",
        "Severely frail",
        "Completely dependent for personal care (physical or cognitive), but stable and not at high risk of \
         dying within about 6 months.",
    ),
    (
        "cfs.8",
        "Very severely frail",
        "Completely dependent and approaching the end of life; could not recover even from a minor illness.",
    ),
    (
        "cfs.9",
        "Terminally ill",
        "Approaching the end of life: life expectancy under 6 months, not otherwise evidently frail.",
    ),
];

pub const CFS: CalcSpec = CalcSpec {
    id: "clinical_frailty",
    name: "Clinical Frailty Scale",
    area: Area::Elderly,
    fields: &[choice("level", "The description that fits best", CFS_LEVELS)
        .hint("In dementia, the degree of frailty follows the degree of dementia.")],
    source: "Book Geriatrics Lecture 7, Frailty assessment, Clinical Frailty Scale (p. 121); Rockwood et al., CMAJ \
             2005;173:489–95",
    compute: cfs,
};

fn cfs(i: &Inputs) -> Result<Outcome, CalcError> {
    let ids: Vec<&str> = CFS_LEVELS.iter().map(|(id, _)| *id).collect();
    let picked = i.get_choice("level", &ids)?;
    let n = ids.iter().position(|id| *id == picked).unwrap_or(0);
    let (band, label, meaning) = CFS_BANDS[n];
    let level = (n + 1) as f64;
    Ok(Outcome {
        score: Some(level),
        display: format!("Level {}: {label}", n + 1),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(121),
        parts: vec![Part::new(label, level)],
        notes: vec![
            "In dementia: mild (forgets details of recent events, repeats questions), moderate (recent memory \
             very impaired, personal care with prompting), severe (cannot do personal care without help)."
                .to_owned(),
        ],
    })
}

// ------------------------------------------------------------------- CAM

pub const CAM: CalcSpec = CalcSpec {
    id: "cam",
    name: "Confusion Assessment Method (CAM)",
    area: Area::Elderly,
    fields: &[
        flag("acute_fluctuating", "1. Acute onset and fluctuating course"),
        flag("inattention", "2. Inattention"),
        flag("disorganised_thinking", "3. Disorganised thinking"),
        flag("altered_consciousness", "4. Altered level of consciousness"),
    ],
    source: "Book Geriatrics Lecture 4, Confusion Assessment Method (pp. 70–71); Inouye et al., Ann Intern Med \
             1990;113:941–8",
    compute: cam,
};

fn cam(i: &Inputs) -> Result<Outcome, CalcError> {
    let f1 = i.get_flag("acute_fluctuating")?;
    let f2 = i.get_flag("inattention")?;
    let f3 = i.get_flag("disorganised_thinking")?;
    let f4 = i.get_flag("altered_consciousness")?;
    let positive = f1 && f2 && (f3 || f4);
    let parts = vec![
        Part::met("1. Acute onset and fluctuating course", f1),
        Part::met("2. Inattention", f2),
        Part::met("3. Disorganised thinking", f3),
        Part::met("4. Altered level of consciousness", f4),
    ];
    let (band, label, meaning) = if positive {
        ("cam.positive", "CAM positive", "Delirium: features 1 and 2, with 3 or 4.")
    } else {
        (
            "cam.negative",
            "CAM negative",
            "Delirium not diagnosed by CAM: it needs features 1 and 2, and either 3 or 4.",
        )
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(71),
        parts,
        notes: Vec::new(),
    })
}

// ---------------------------------------------------------------- SARC-F

const SARC_STRENGTH: &[(&str, &str)] = &[("none", "None"), ("some", "Some"), ("a_lot", "A lot or unable")];
const SARC_WALK: &[(&str, &str)] =
    &[("none", "None"), ("some", "Some"), ("a_lot", "A lot, uses aids, or unable")];
const SARC_RISE: &[(&str, &str)] =
    &[("none", "None"), ("some", "Some"), ("a_lot", "A lot or unable without help")];
const SARC_STAIRS: &[(&str, &str)] = &[("none", "None"), ("some", "Some"), ("a_lot", "A lot or unable")];
const SARC_FALLS: &[(&str, &str)] = &[("none", "None"), ("1_3", "1–3 falls"), ("4_plus", "4 or more falls")];
const ZERO_ONE_TWO: &[f64] = &[0.0, 1.0, 2.0];

pub const SARC_F: CalcSpec = CalcSpec {
    id: "sarc_f",
    name: "SARC-F (sarcopenia screen)",
    area: Area::Elderly,
    fields: &[
        choice("strength", "Strength: difficulty lifting and carrying 10 pounds", SARC_STRENGTH),
        choice("walking", "Assistance in walking: difficulty walking across a room", SARC_WALK),
        choice("rise", "Rise from a chair: difficulty transferring from a chair or bed", SARC_RISE),
        choice("stairs", "Climb stairs: difficulty climbing a flight of 10 stairs", SARC_STAIRS),
        choice("falls", "Falls in the past year", SARC_FALLS).hint(
            "The book prints \"less than 3 falls\" for 1 point; 1–3 falls score 1 in the original SARC-F.",
        ),
    ],
    source: "Book Geriatrics Lecture 8, Sarcopenia, SARC-F questionnaire (pp. 133–134); Malmstrom & Morley, J Am Med \
             Dir Assoc 2013;14:531–2 (falls 1–3 = 1 point); next steps from EWGSOP2, Age Ageing 2019;48:16–31",
    compute: sarc_f,
};

fn sarc_f(i: &Inputs) -> Result<Outcome, CalcError> {
    let parts = vec![
        Part::new("Strength", scored(i, "strength", SARC_STRENGTH, ZERO_ONE_TWO)?),
        Part::new("Assistance in walking", scored(i, "walking", SARC_WALK, ZERO_ONE_TWO)?),
        Part::new("Rise from a chair", scored(i, "rise", SARC_RISE, ZERO_ONE_TWO)?),
        Part::new("Climb stairs", scored(i, "stairs", SARC_STAIRS, ZERO_ONE_TWO)?),
        Part::new("Falls", scored(i, "falls", SARC_FALLS, ZERO_ONE_TWO)?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total >= 4.0 {
        (
            "sarc_f.positive",
            "4 or more: positive",
            "Positive screen: needs fuller evaluation for sarcopenia (muscle strength, mass and performance).",
        )
    } else {
        ("sarc_f.negative", "Below 4: negative", "Negative screen for sarcopenia.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / 10", whole(total)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(134),
        parts,
        notes: Vec::new(),
    })
}

// ------------------------------------------------------------------- MMSE

/// (key, label, maximum)
const MMSE_SECTIONS: &[(&str, &str, f64)] = &[
    ("orientation_time", "Orientation: year, season, date, day, month", 5.0),
    ("orientation_place", "Orientation: country, county, town, hospital, floor", 5.0),
    ("registration", "Registration: name 3 objects", 3.0),
    ("attention", "Attention: serial 7s or WORLD backwards", 5.0),
    ("recall", "Recall the 3 objects", 3.0),
    ("naming", "Language: name a pencil and a watch", 2.0),
    ("repetition", "Language: repeat \"no ifs, ands, or buts\"", 1.0),
    ("command", "Language: 3-stage command", 3.0),
    ("read_write_copy", "Language: read and obey, write a sentence, copy a design", 3.0),
];

pub const MMSE: CalcSpec = CalcSpec {
    id: "mmse",
    name: "Mini-Mental State Examination (MMSE)",
    area: Area::Elderly,
    fields: &[
        num("orientation_time", "Orientation: year, season, date, day, month", "", 0.0, 5.0, 1.0),
        num("orientation_place", "Orientation: country, county, town, hospital, floor", "", 0.0, 5.0, 1.0),
        num("registration", "Registration: name 3 objects", "", 0.0, 3.0, 1.0),
        num("attention", "Attention: serial 7s or WORLD backwards", "", 0.0, 5.0, 1.0),
        num("recall", "Recall the 3 objects", "", 0.0, 3.0, 1.0),
        num("naming", "Language: name a pencil and a watch", "", 0.0, 2.0, 1.0),
        num("repetition", "Language: repeat \"no ifs, ands, or buts\"", "", 0.0, 1.0, 1.0),
        num("command", "Language: 3-stage command", "", 0.0, 3.0, 1.0),
        num("read_write_copy", "Language: read and obey, write a sentence, copy a design", "", 0.0, 3.0, 1.0),
    ],
    source: "Book Geriatrics Lecture 4, Mini-Mental State Examination (p. 63); Folstein et al., J Psychiatr Res \
             1975;12:189–98",
    compute: mmse,
};

fn mmse(i: &Inputs) -> Result<Outcome, CalcError> {
    let mut parts = Vec::new();
    for (key, label, max) in MMSE_SECTIONS {
        let v = i.get_num(key)?;
        if v < 0.0 || v > *max || v.fract() != 0.0 {
            return Err(CalcError::Invalid {
                field: key,
                reason: "a whole number from 0 to the section's maximum",
            });
        }
        parts.push(Part::new(*label, v));
    }
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let mut notes = Vec::new();
    let (band, label, meaning) = if total >= 29.0 {
        ("mmse.normal", "29–30: normal", "Normal.")
    } else if total >= 26.0 {
        ("mmse.borderline", "26–28: borderline", "Borderline cognitive dysfunction.")
    } else if total >= 18.0 {
        (
            "mmse.marked",
            "18–25: marked dysfunction",
            "Marked cognitive dysfunction: may be diagnosed as dementia.",
        )
    } else {
        if total == 17.0 {
            notes.push(
                "The book's table jumps from 18 to \"below 17\"; 17 is read as severe here.".to_owned(),
            );
        }
        ("mmse.severe", "Below 18: severe", "Severe dysfunction: severe dementia.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{} / 30", whole(total)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(63),
        parts,
        notes,
    })
}

// --------------------------------------------------- Ankle-brachial index

pub const ABI: CalcSpec = CalcSpec {
    id: "abi",
    name: "Ankle-brachial index (ABI)",
    area: Area::Elderly,
    fields: &[
        num("dp_mm_hg", "Ankle systolic: dorsalis pedis", "mm Hg", 0.0, 350.0, 1.0)
            .hint("Doppler pressure at the ankle; the higher of the two ankle pressures is used."),
        num("pt_mm_hg", "Ankle systolic: posterior tibial", "mm Hg", 0.0, 350.0, 1.0).optional(),
        num("brachial_right_mm_hg", "Brachial systolic: right arm", "mm Hg", 40.0, 300.0, 1.0)
            .hint("The higher of the two arm pressures is used."),
        num("brachial_left_mm_hg", "Brachial systolic: left arm", "mm Hg", 40.0, 300.0, 1.0).optional(),
    ],
    source: "Book Geriatrics Lecture 1, Subclinical cardiovascular disease (p. 8); Book Diabetes Lecture 5, \
             Ankle brachial pressure index (p. 60); AHA/ACC 2016 PAD guideline (Gerhard-Herman et al., \
             Circulation 2017;135:e726–79) for 0.91–0.99 borderline and above 1.40 non-compressible",
    compute: abi,
};

fn abi(i: &Inputs) -> Result<Outcome, CalcError> {
    let dp = i.get_num("dp_mm_hg")?;
    let pt = i.opt_num("pt_mm_hg")?;
    let right = i.get_pos("brachial_right_mm_hg")?;
    let left = i.opt_num("brachial_left_mm_hg")?;
    if dp < 0.0 {
        return Err(CalcError::Invalid { field: "dp_mm_hg", reason: "must be 0 or more" });
    }
    let ankle = pt.map_or(dp, |p| p.max(dp));
    let arm = left.map_or(right, |l| l.max(right));
    let ratio = ((ankle / arm) * 100.0).round() / 100.0;
    let parts = vec![Part::new("Higher ankle pressure", ankle), Part::new("÷ higher brachial pressure", arm)];
    let mut notes = Vec::new();
    if ratio > 1.4 {
        notes.push(
            "Above 1.40 means stiff, calcified arteries that do not compress (common in diabetes), so the \
             ABI can hide disease. The book does not cover this; the AHA/ACC 2016 guideline calls it non-compressible and uses a toe-brachial index instead."
                .to_owned(),
        );
    }
    if ratio > 0.9 && ratio < 1.0 {
        notes.push(
            "0.91–0.99 is normal by the book but \"borderline\" in the AHA/ACC 2016 guideline: with leg \
             symptoms, do an exercise ABI."
                .to_owned(),
        );
    }
    let (band, label, meaning) = if ratio > 0.9 {
        ("abi.normal", "Above 0.9: normal", "Normal.")
    } else if ratio >= 0.4 {
        (
            "abi.pad",
            "0.9 or below: PAD",
            "Peripheral arterial disease; the lower range goes with claudication.",
        )
    } else {
        (
            "abi.critical",
            "Below 0.4: critical ischaemia",
            "Severe disease: commonly ischaemic rest pain and tissue necrosis.",
        )
    };
    Ok(Outcome {
        score: Some(ratio),
        display: show(ratio, 2),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(8),
        parts,
        notes,
    })
}

// ------------------------------------------------- next steps and the DD

/// Causes to look for when function (ADL or IADL) falls.
const FUNCTION_DD: &[&str] = &[
    "Acute illness or delirium (infection, MI, stroke): a sudden fall in function",
    "Dementia",
    "Depression",
    "Stroke, Parkinson's disease or another neurological disability",
    "Osteoarthritis, fracture, falls or sarcopenia",
    "Poor vision or hearing",
    "Drug side effects (polypharmacy)",
];

/// Causes to look for in a frail older person.
const FRAILTY_DD: &[&str] = &[
    "An acute illness on top: score the baseline from 2 weeks before it",
    "Disability from one disease (stroke, arthritis) rather than global frailty",
    "Dementia or depression",
    "Sarcopenia and malnutrition",
    "Hypothyroidism, anaemia, heart failure, CKD",
    "Polypharmacy",
];

/// Leg pain on walking that is not arterial.
const ABI_DD: &[&str] = &[
    "Lumbar spinal stenosis (neurogenic claudication: eased by bending forward)",
    "Hip or knee osteoarthritis",
    "Nerve root compression (sciatica)",
    "Venous claudication",
    "Symptomatic Baker's cyst",
    "Chronic compartment syndrome",
];

const COGNITION_DD: &[&str] = &[
    "Alzheimer disease",
    "Vascular dementia",
    "Dementia with Lewy bodies (fluctuation, hallucinations, parkinsonism)",
    "Frontotemporal dementia",
    "Delirium (acute onset, inattention): screen with CAM",
    "Depression with cognitive impairment: screen with GDS",
    "Reversible causes: hypothyroidism, B12 deficiency, normal-pressure hydrocephalus, subdural haematoma, drugs",
    "Low education, language barrier, poor vision or hearing (a low score without disease)",
];

const DEMENTIA_WORKUP: &str = "Dementia work-up: informant history, ADL and IADL, CBC, B12 and folate, TSH, calcium, \
                               glucose, renal and liver tests; CT or MRI brain";

const FRAIL_NEXT: &[&str] = &[
    "Comprehensive geriatric assessment: falls, cognition, mood, nutrition, continence",
    "Medication review (deprescribing)",
    "Exercise (resistance and balance) and protein; physiotherapy and occupational therapy",
    "Weigh frailty in treatment decisions: higher risk from surgery, chemotherapy and admission",
];

const END_OF_LIFE_NEXT: &[&str] = &[
    "Advance care planning and goals of care with the patient and family",
    "Weigh benefit against burden for invasive treatment and ICU",
    "Palliative care for symptoms",
];

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "katz_adl.independent",
        Guide {
            next: &[
                "Recheck after any acute illness or admission: a new loss points to acute disease (p. 111)",
            ],
            dd: &[],
        },
    ),
    (
        "katz_adl.partial",
        Guide {
            next: &[
                "Ask how recently it changed: a sudden loss means acute illness to find and treat; a slow loss \
                 needs support (p. 111)",
                "Complete the CGA: MMSE, GDS, MNA, Clinical Frailty Scale, medication review",
                "Physiotherapy, occupational therapy, aids and carers for the lost activities",
            ],
            dd: FUNCTION_DD,
        },
    ),
    (
        "katz_adl.very_dependent",
        Guide {
            next: &[
                "Look for an acute, treatable cause if the loss is recent",
                "Full care needs: carers or a care home, pressure-area and continence care, nutrition",
                "Advance care planning",
            ],
            dd: FUNCTION_DD,
        },
    ),
    (
        "lawton_iadl.independent",
        Guide { next: &["Repeat yearly or after illness: IADLs are lost before basic ADLs"], dd: &[] },
    ),
    (
        "lawton_iadl.partial",
        Guide {
            next: &[
                "IADLs fall before basic ADLs: an early sign of cognitive decline or frailty. Screen with MMSE and \
                 GDS, and check the Katz ADL",
                "Help with the lost tasks: medications in a dosette, help with money, transport",
            ],
            dd: &[
                "Mild cognitive impairment or early dementia",
                "Depression",
                "Frailty",
                "Poor vision or hearing",
                "Musculoskeletal or neurological disability",
                "A task never done (role, culture) rather than a disability",
            ],
        },
    ),
    (
        "lawton_iadl.dependent",
        Guide {
            next: &[
                "Check basic ADLs (Katz) and cognition (MMSE)",
                "Arrange help with every instrumental task; assess safety living alone",
            ],
            dd: FUNCTION_DD,
        },
    ),
    (
        "gds15.normal",
        Guide {
            next: &["A screen, not a diagnosis: if depression is still suspected, take a clinical history"],
            dd: &[],
        },
    ),
    (
        "gds15.depression",
        Guide {
            next: &[
                "Confirm with a clinical interview (DSM-5 major depression); 10 or more is almost always \
                 depression (Sheikh & Yesavage 1986)",
                "Ask about suicidal thoughts",
                "Look for medical causes and drugs; then psychotherapy and an SSRI (avoid tricyclics in the elderly)",
            ],
            dd: &[
                "Major depressive disorder",
                "Bereavement or adjustment disorder",
                "Dementia with apathy, or depression with cognitive impairment",
                "Hypothyroidism",
                "Drug-induced: beta-blockers, steroids, benzodiazepines, levodopa",
                "Parkinson's disease or post-stroke depression",
                "Hypoactive delirium",
                "Chronic pain or alcohol use",
            ],
        },
    ),
    (
        "mna_sf.normal",
        Guide {
            next: &[
                "Rescreen: yearly in the community, every 3 months in care homes, after any acute illness",
            ],
            dd: &[],
        },
    ),
    (
        "mna_sf.at_risk",
        Guide {
            next: &[
                "Complete the full MNA (items G–R, p. 124)",
                "Weigh regularly, dietary advice and oral supplements; rescreen in 3 months",
            ],
            dd: &[
                "Poor intake: teeth, dysphagia, poverty, living alone",
                "Depression or dementia",
                "Drugs causing anorexia, nausea or dry mouth",
                "Chronic disease: heart failure, COPD, CKD, cancer",
            ],
        },
    ),
    (
        "mna_sf.malnourished",
        Guide {
            next: &[
                "Complete the full MNA (items G–R, p. 124) and refer to a dietitian",
                "Treat the cause; oral supplements, then enteral feeding if intake stays poor",
                "Albumin, CBC, B12, folate, vitamin D; watch for refeeding syndrome",
            ],
            dd: &[
                "Cancer",
                "Depression or dementia",
                "Dysphagia, poor teeth, chronic GI disease or malabsorption",
                "Chronic disease: heart failure, COPD, CKD",
                "Hyperthyroidism or uncontrolled diabetes",
                "Poverty, isolation, inability to shop or cook",
                "Drugs causing anorexia",
            ],
        },
    ),
    ("cfs.1", Guide { next: &["No frailty: keep active"], dd: &[] }),
    ("cfs.2", Guide { next: &["No frailty: encourage regular exercise"], dd: &[] }),
    ("cfs.3", Guide { next: &["No frailty: keep chronic disease controlled; encourage activity"], dd: &[] }),
    (
        "cfs.4",
        Guide {
            next: &[
                "Pre-frail: look for treatable causes of the symptoms",
                "Exercise, nutrition and a medication review to prevent frailty",
            ],
            dd: FRAILTY_DD,
        },
    ),
    ("cfs.5", Guide { next: FRAIL_NEXT, dd: FRAILTY_DD }),
    ("cfs.6", Guide { next: FRAIL_NEXT, dd: FRAILTY_DD }),
    ("cfs.7", Guide { next: END_OF_LIFE_NEXT, dd: FRAILTY_DD }),
    ("cfs.8", Guide { next: END_OF_LIFE_NEXT, dd: &[] }),
    ("cfs.9", Guide { next: END_OF_LIFE_NEXT, dd: &[] }),
    (
        "cam.positive",
        Guide {
            next: &[
                "Find the cause: infection, drugs (anticholinergics, sedatives, opioids), metabolic (Na, glucose, \
                 Ca, renal), hypoxia, urinary retention, constipation, pain, alcohol withdrawal",
                "Drug review, bloods, urine, ECG and chest X-ray as indicated",
                "Non-drug care first: orientation, glasses and hearing aids, sleep, family; avoid restraints",
                "Low-dose antipsychotic only for dangerous agitation (not in Parkinson's or Lewy body dementia)",
            ],
            dd: &[
                "Dementia (gradual onset, attention spared early)",
                "Depression (mimics hypoactive delirium)",
                "Acute psychosis",
                "Non-convulsive status epilepticus",
                "Wernicke encephalopathy",
                "Dementia with Lewy bodies (fluctuation)",
            ],
        },
    ),
    (
        "cam.negative",
        Guide {
            next: &[
                "Delirium fluctuates: reassess each shift if it is still suspected (CAM sensitivity 94–100%, \
                 specificity 90–95%, Inouye 1990)",
            ],
            dd: &[],
        },
    ),
    (
        "sarc_f.positive",
        Guide {
            next: &[
                "Confirm with strength (EWGSOP2): grip under 27 kg (men) or 16 kg (women), or 5 chair stands over \
                 15 s",
                "Then muscle mass (DXA or BIA); gait speed 0.8 m/s or less means severe sarcopenia",
                "Resistance exercise and enough protein; screen nutrition (MNA)",
            ],
            dd: &[
                "Sarcopenia",
                "Frailty",
                "Cachexia (cancer, heart failure, CKD, COPD)",
                "Malnutrition",
                "Myopathy: statins, steroids, hypothyroidism, vitamin D deficiency, polymyalgia rheumatica",
                "Neurological weakness: Parkinson's disease, neuropathy, myelopathy",
                "Arthritis pain limiting function",
            ],
        },
    ),
    ("sarc_f.negative", Guide { next: &["Rescreen yearly, or after an acute illness or a fall"], dd: &[] }),
    (
        "mmse.normal",
        Guide {
            next: &[
                "A highly educated person with early dementia can still score normal: if the history worries you, \
                     use a harder test (MoCA)",
            ],
            dd: &[],
        },
    ),
    (
        "mmse.borderline",
        Guide {
            next: &[
                "Take an informant history and check IADLs (Lawton)",
                "Repeat in 6–12 months or use a more sensitive test (MoCA)",
            ],
            dd: COGNITION_DD,
        },
    ),
    (
        "mmse.marked",
        Guide {
            next: &[
                "Exclude delirium (CAM) and depression (GDS) first",
                DEMENTIA_WORKUP,
                "The usual cut-off for cognitive impairment is 23 or less (Folstein 1975)",
            ],
            dd: COGNITION_DD,
        },
    ),
    (
        "mmse.severe",
        Guide {
            next: &[
                "Exclude delirium (CAM) and depression (GDS) first",
                DEMENTIA_WORKUP,
                "Plan care: safety, carers, capacity and advance care planning",
            ],
            dd: COGNITION_DD,
        },
    ),
    (
        "abi.normal",
        Guide {
            next: &[
                "With typical claudication and a normal resting ABI, do an exercise (treadmill) ABI",
                "Above 1.40: arteries do not compress; use a toe-brachial index (abnormal 0.70 or less)",
            ],
            dd: &[],
        },
    ),
    (
        "abi.pad",
        Guide {
            next: &[
                "Treat as atherosclerotic disease: stop smoking, statin, antiplatelet, control BP and diabetes",
                "Supervised exercise for claudication; duplex ultrasound if revascularisation is considered",
                "Look for disease elsewhere: coronary and carotid",
            ],
            dd: ABI_DD,
        },
    ),
    (
        "abi.critical",
        Guide {
            next: &[
                "Urgent vascular referral: chronic limb-threatening ischaemia; imaging for revascularisation",
                "Foot care and wound care; treat infection",
            ],
            dd: &[
                "Acute limb ischaemia (sudden: embolism or thrombosis) needs emergency care",
                "Diabetic neuropathic pain",
                "Vasculitis or thromboangiitis obliterans (Buerger disease)",
                "Cholesterol emboli",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn all_of(keys: &[&str], value: &str) -> Inputs {
        keys.iter().fold(Inputs::new(), |i, k| i.choice(k, value))
    }

    #[test]
    fn every_band_has_a_guide() {
        let bands = [
            "katz_adl.independent",
            "katz_adl.partial",
            "katz_adl.very_dependent",
            "lawton_iadl.independent",
            "lawton_iadl.partial",
            "lawton_iadl.dependent",
            "gds15.normal",
            "gds15.depression",
            "mna_sf.normal",
            "mna_sf.at_risk",
            "mna_sf.malnourished",
            "cfs.1",
            "cfs.2",
            "cfs.3",
            "cfs.4",
            "cfs.5",
            "cfs.6",
            "cfs.7",
            "cfs.8",
            "cfs.9",
            "cam.positive",
            "cam.negative",
            "sarc_f.positive",
            "sarc_f.negative",
            "mmse.normal",
            "mmse.borderline",
            "mmse.marked",
            "mmse.severe",
            "abi.normal",
            "abi.pad",
            "abi.critical",
        ];
        for b in bands {
            assert!(super::super::guide(b).is_some(), "{b}");
        }
    }

    #[test]
    fn katz_counts_independent_activities() {
        let keys: Vec<&str> = KATZ_ITEMS.iter().map(|(k, _)| *k).collect();
        let o = katz(&all_of(&keys, "independent")).unwrap();
        assert_eq!((o.score, o.band), (Some(6.0), "katz_adl.independent"));
        let o = katz(&all_of(&keys, "dependent")).unwrap();
        assert_eq!((o.score, o.band), (Some(0.0), "katz_adl.very_dependent"));
        let o = katz(
            &all_of(&keys, "independent").choice("bathing", "dependent").choice("continence", "dependent"),
        )
        .unwrap();
        assert_eq!((o.score, o.band), (Some(4.0), "katz_adl.partial"));
        assert!(o.meaning.contains("2 of the six"));
        assert!(katz(&all_of(&keys[..5], "independent")).is_err());
    }

    fn lawton_inputs(sex: &str) -> Inputs {
        Inputs::new()
            .choice("sex", sex)
            .choice("telephone", "answers_only") // 1
            .choice("shopping", "small") // 0
            .choice("food_preparation", "if_supplied") // 0
            .choice("housekeeping", "needs_help") // 1
            .choice("laundry", "small_items") // 1
            .choice("transport", "accompanied") // 1
            .choice("medication", "if_prepared") // 0
            .choice("finances", "day_to_day") // 1
    }

    #[test]
    fn lawton_scores_women_of_8_and_men_of_5() {
        let o = lawton(&lawton_inputs("female")).unwrap();
        assert_eq!((o.score, o.display.as_str()), (Some(5.0), "5 / 8"));
        assert_eq!(o.band, "lawton_iadl.partial");
        let o = lawton(&lawton_inputs("male")).unwrap();
        assert_eq!((o.score, o.display.as_str()), (Some(3.0), "3 / 5"));
        assert_eq!(o.notes.len(), 1);
        // Men need only the five domains; women need all eight.
        let men = Inputs::new()
            .choice("sex", "male")
            .choice("telephone", "own_initiative")
            .choice("shopping", "all")
            .choice("transport", "taxi")
            .choice("medication", "responsible")
            .choice("finances", "independent");
        let o = lawton(&men).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "lawton_iadl.independent"));
        assert!(lawton(&men.clone().choice("sex", "female")).is_err());
    }

    #[test]
    fn gds_scores_the_depressive_answers() {
        // Every answer in the healthy direction: 0.
        let healthy = GDS_ITEMS
            .iter()
            .fold(Inputs::new(), |i, (k, dep)| i.choice(k, if *dep == "yes" { "no" } else { "yes" }));
        let o = gds(&healthy).unwrap();
        assert_eq!((o.score, o.band), (Some(0.0), "gds15.normal"));
        // The five reversed items answered "no": 5, still normal (0–5).
        let five = ["satisfied", "good_spirits", "happy", "wonderful_alive", "full_of_energy"]
            .iter()
            .fold(healthy.clone(), |i, k| i.choice(k, "no"));
        assert_eq!(gds(&five).unwrap().band, "gds15.normal");
        // One more: 6, above 5.
        let o = gds(&five.choice("bored", "yes")).unwrap();
        assert_eq!((o.score, o.band), (Some(6.0), "gds15.depression"));
        assert!(expect_ok(&o, "GDS 6, suggests depression"));
        assert!(!expect_ok(&o, "6, normal"));
    }

    fn expect_ok(o: &Outcome, text: &str) -> bool {
        super::super::expect_matches(o, text)
    }

    #[test]
    fn mna_screening_bands() {
        let i = Inputs::new()
            .choice("food_intake", "moderate") // 1
            .choice("weight_loss", "1_3kg") // 2
            .choice("mobility", "goes_out") // 2
            .choice("stress", "no") // 2
            .choice("neuropsych", "mild_dementia") // 1
            .choice("bmi", "19_21"); // 1
        let o = mna_sf(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(9.0), "mna_sf.at_risk"));
        let o = mna_sf(&i.clone().choice("bmi", "23_plus").choice("food_intake", "none")).unwrap();
        assert_eq!((o.score, o.band), (Some(12.0), "mna_sf.normal"));
        let o = mna_sf(&i.choice("stress", "yes").choice("weight_loss", "over_3kg")).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "mna_sf.malnourished"));
    }

    #[test]
    fn frailty_levels_keep_the_book_names() {
        let o = cfs(&Inputs::new().choice("level", "5")).unwrap();
        assert_eq!((o.score, o.band, o.label), (Some(5.0), "cfs.5", "Mildly frail"));
        assert!(expect_ok(&o, "CFS 5, mildly frail"));
        assert!(!expect_ok(&o, "CFS 5, moderately frail"));
        let o = cfs(&Inputs::new().choice("level", "8")).unwrap();
        assert_eq!(o.label, "Very severely frail");
        assert!(super::super::answer_matches(&o, "very severely frail"));
        assert!(!super::super::answer_matches(&o, "severely frail"));
        assert!(cfs(&Inputs::new().choice("level", "10")).is_err());
    }

    #[test]
    fn cam_needs_1_and_2_and_3_or_4() {
        let i = Inputs::new().flag("acute_fluctuating", true).flag("inattention", true);
        assert_eq!(cam(&i).unwrap().band, "cam.negative");
        assert_eq!(cam(&i.clone().flag("altered_consciousness", true)).unwrap().band, "cam.positive");
        assert_eq!(cam(&i.flag("disorganised_thinking", true)).unwrap().band, "cam.positive");
        let no_onset = Inputs::new()
            .flag("inattention", true)
            .flag("disorganised_thinking", true)
            .flag("altered_consciousness", true);
        let o = cam(&no_onset).unwrap();
        assert_eq!(o.band, "cam.negative");
        assert!(expect_ok(&o, "CAM negative"));
        assert!(!expect_ok(&o, "CAM positive"));
    }

    #[test]
    fn sarc_f_four_or_more_is_positive() {
        let i = Inputs::new()
            .choice("strength", "some")
            .choice("walking", "none")
            .choice("rise", "some")
            .choice("stairs", "a_lot")
            .choice("falls", "none");
        let o = sarc_f(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(4.0), "sarc_f.positive"));
        let o = sarc_f(&i.choice("stairs", "some")).unwrap();
        assert_eq!((o.score, o.band), (Some(3.0), "sarc_f.negative"));
    }

    fn mmse_inputs(scores: [f64; 9]) -> Inputs {
        MMSE_SECTIONS.iter().zip(scores).fold(Inputs::new(), |i, ((k, _, _), v)| i.num(k, v))
    }

    #[test]
    fn mmse_bands_follow_the_book() {
        let full = [5.0, 5.0, 3.0, 5.0, 3.0, 2.0, 1.0, 3.0, 3.0];
        let o = mmse(&mmse_inputs(full)).unwrap();
        assert_eq!((o.score, o.band), (Some(30.0), "mmse.normal"));
        let o = mmse(&mmse_inputs([4.0, 4.0, 3.0, 3.0, 1.0, 2.0, 1.0, 3.0, 3.0])).unwrap();
        assert_eq!((o.score, o.band), (Some(24.0), "mmse.marked"));
        let o = mmse(&mmse_inputs([5.0, 5.0, 3.0, 5.0, 1.0, 2.0, 1.0, 3.0, 2.0])).unwrap();
        assert_eq!((o.score, o.band), (Some(27.0), "mmse.borderline"));
        let o = mmse(&mmse_inputs([2.0, 3.0, 3.0, 1.0, 0.0, 2.0, 1.0, 3.0, 2.0])).unwrap();
        assert_eq!((o.score, o.band, o.notes.len()), (Some(17.0), "mmse.severe", 1));
        // A section above its maximum is rejected.
        assert!(mmse(&mmse_inputs([6.0, 5.0, 3.0, 5.0, 3.0, 2.0, 1.0, 3.0, 3.0])).is_err());
        assert!(mmse(&mmse_inputs([5.0, 5.0, 3.0, 5.0, 3.0, 2.0, 1.5, 3.0, 3.0])).is_err());
    }

    #[test]
    fn abi_uses_the_higher_pressures() {
        let i = Inputs::new()
            .num("dp_mm_hg", 90.0)
            .num("pt_mm_hg", 100.0)
            .num("brachial_right_mm_hg", 130.0)
            .num("brachial_left_mm_hg", 140.0);
        let o = abi(&i).unwrap();
        // 100 / 140 = 0.714.
        assert_eq!((o.score, o.band), (Some(0.71), "abi.pad"));
        let o = abi(&Inputs::new().num("dp_mm_hg", 130.0).num("brachial_right_mm_hg", 120.0)).unwrap();
        assert_eq!((o.score, o.band), (Some(1.08), "abi.normal"));
        // Exactly 0.9 is not "above 0.9".
        let o = abi(&Inputs::new().num("dp_mm_hg", 90.0).num("brachial_right_mm_hg", 100.0)).unwrap();
        assert_eq!(o.band, "abi.pad");
        let o = abi(&Inputs::new().num("dp_mm_hg", 40.0).num("brachial_right_mm_hg", 130.0)).unwrap();
        assert_eq!((o.score, o.band), (Some(0.31), "abi.critical"));
        let o = abi(&Inputs::new().num("dp_mm_hg", 200.0).num("brachial_right_mm_hg", 130.0)).unwrap();
        assert_eq!(o.notes.len(), 1);
        assert!(abi(&Inputs::new().num("dp_mm_hg", 90.0).num("brachial_right_mm_hg", 0.0)).is_err());
    }
}
