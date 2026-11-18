//! Sealed questions: Mohamed's own question bank, VQ (docs/VQ.md), stays
//! encrypted in git and in every content pack, and opens only on a device
//! where he entered his key.
//!
//! - **The key** is 32 random bytes (`steps-tools vq keygen`). It is written
//!   as RFC 4648 base32 without padding, in groups of four joined by `-`;
//!   reading it ignores spaces, dashes and case. Its id ([`Key::id`], 8 hex
//!   digits) says which key sealed a file and tells nothing about the key.
//! - **Subkeys** are keyed BLAKE2b-256 of the labels `steps vq enc` and
//!   `steps vq nonce`; the key id is the first 4 bytes of the same over
//!   `steps vq key id`.
//! - **A blob** is `0x01 ‖ nonce ‖ ciphertext+tag` in base64url without
//!   padding. XChaCha20-Poly1305 seals the plaintext with the item id as
//!   associated data, so a blob moved to another id doesn't open. The
//!   24-byte nonce is keyed BLAKE2b (nonce subkey) of `id ‖ 0x00 ‖
//!   plaintext`: the same item under the same key always gives the same
//!   bytes (SIV style), so an untouched question never shows in a diff.
//!
//! A sealed item opens to [`Opened`]: the item itself and, when the question
//! has them, its figure and its educational objective. Pure: no randomness
//! and no IO.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use blake2::Blake2bMac;
use blake2::digest::Mac as _;
use blake2::digest::consts::{U24, U32};
use chacha20poly1305::aead::{Aead as _, Payload};
use chacha20poly1305::{KeyInit as _, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};

use crate::content::Item;

/// `content/<course>/vq/LNN.yaml`.
pub const SCHEMA: &str = "steps.sealed/v1";

/// The first byte of every blob: the layout above.
const VERSION: u8 = 1;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;

/// The source VQ questions cite (`src.book`), a `qbank` source whose
/// `page` counts questions.
pub const SOURCE: &str = "vq";
/// The tag every VQ question carries (the bank's «أسئلة VQ» filter).
pub const TAG: &str = "vq";
/// The only figure type, and the largest figure a question may carry.
pub const IMAGE_TYPE: &str = "image/webp";
pub const MAX_IMAGE_BYTES: usize = 200 * 1024;

/// One lecture's sealed questions, as the file holds them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedLecture {
    pub schema: String,
    /// `path.L03`: the First Aid lecture the questions are filed under.
    pub lecture: String,
    /// Which key sealed them ([`Key::id`]); never the key.
    pub key: String,
    /// Sorted by id.
    pub items: Vec<SealedItem>,
}

/// A question in the file: its id and kind in clear, so `steps-tools ids`
/// works without the key, and the rest sealed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedItem {
    /// `path.L03.vq0001` is VQ-0001.
    pub id: String,
    pub kind: String,
    /// The blob: an [`Opened`] as JSON, sealed.
    pub sealed: String,
}

/// What a sealed item opens to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opened {
    pub item: Item,
    /// The question's figure: the file's bytes in standard base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// The figure's media type ([`IMAGE_TYPE`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_type: Option<String>,
    /// The question's educational objective: the one point it teaches, as
    /// the bank words it. Shown with the explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SealError {
    #[error("not a VQ key: it is 52 letters and digits (A–Z, 2–7) in groups of four")]
    KeyText,
    #[error("not a sealed blob")]
    Blob,
    #[error("doesn't open with this key (another key, a changed byte, or another question's blob)")]
    Open,
    #[error("opened, but holds no question: {0}")]
    Json(String),
    #[error("opened to {found}, but the file names it {expected}")]
    Id { expected: String, found: String },
    #[error("opened to a {found}, but the file says {expected}")]
    Kind { expected: String, found: String },
}

/// A VQ key. Its `Debug` shows only the key id, so a key never lands in a
/// log; [`Key::text`] is the one way to write it out.
#[derive(Clone, PartialEq, Eq)]
pub struct Key([u8; 32]);

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key(id {})", self.id())
    }
}

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
/// 256 bits in 5-bit letters, the last with 4 bits to spare.
const KEY_LETTERS: usize = 52;

impl Key {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Reads the text form; spaces, dashes and case don't matter.
    pub fn parse(text: &str) -> Result<Self, SealError> {
        let letters: Vec<u8> = text
            .bytes()
            .filter(|b| !b.is_ascii_whitespace() && *b != b'-')
            .map(|b| b.to_ascii_uppercase())
            .collect();
        if letters.len() != KEY_LETTERS {
            return Err(SealError::KeyText);
        }
        let (mut out, mut acc, mut bits) = (Vec::with_capacity(32), 0u32, 0u32);
        for b in letters {
            let v = BASE32.iter().position(|&c| c == b).ok_or(SealError::KeyText)? as u32;
            acc = (acc << 5) | v;
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
        // The spare bits are zero, so each key has exactly one text form.
        if acc != 0 {
            return Err(SealError::KeyText);
        }
        out.try_into().map(Self).map_err(|_| SealError::KeyText)
    }

    /// `ABCD-EFGH-…`: base32, uppercase, in groups of four.
    pub fn text(&self) -> String {
        let (mut letters, mut acc, mut bits) = (Vec::with_capacity(KEY_LETTERS), 0u32, 0u32);
        for &b in &self.0 {
            acc = (acc << 8) | u32::from(b);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                letters.push(BASE32[((acc >> bits) & 31) as usize]);
            }
            acc &= (1 << bits) - 1;
        }
        if bits > 0 {
            letters.push(BASE32[((acc << (5 - bits)) & 31) as usize]);
        }
        letters.chunks(4).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join("-")
    }

    /// 8 hex digits naming the key in files and on screens.
    pub fn id(&self) -> String {
        self.subkey(b"steps vq key id")[..4].iter().map(|b| format!("{b:02x}")).collect()
    }

    fn subkey(&self, label: &[u8]) -> [u8; 32] {
        let mut mac = Blake2bMac::<U32>::new_from_slice(&self.0).expect("a 32-byte key fits BLAKE2b");
        mac.update(label);
        mac.finalize().into_bytes().into()
    }
}

/// Seals `plaintext` for the item `id`.
pub fn seal(key: &Key, id: &str, plaintext: &[u8]) -> String {
    let nonce = nonce(key, id, plaintext);
    let cipher = XChaCha20Poly1305::new(&key.subkey(b"steps vq enc").into());
    let sealed = cipher
        .encrypt(&XNonce::from(nonce), Payload { msg: plaintext, aad: id.as_bytes() })
        .expect("a question is far below the cipher's length limit");
    let mut blob = Vec::with_capacity(1 + NONCE_LEN + sealed.len());
    blob.push(VERSION);
    blob.extend_from_slice(&nonce);
    blob.extend(sealed);
    URL_SAFE_NO_PAD.encode(blob)
}

/// Opens a blob [`seal`] made for the item `id`.
pub fn open(key: &Key, id: &str, blob: &str) -> Result<Vec<u8>, SealError> {
    let bytes = URL_SAFE_NO_PAD.decode(blob.trim()).map_err(|_| SealError::Blob)?;
    if bytes.len() < 1 + NONCE_LEN + TAG_LEN || bytes[0] != VERSION {
        return Err(SealError::Blob);
    }
    let (nonce, sealed) = bytes[1..].split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| SealError::Blob)?;
    let cipher = XChaCha20Poly1305::new(&key.subkey(b"steps vq enc").into());
    let plain = cipher
        .decrypt(&XNonce::from(nonce), Payload { msg: sealed, aad: id.as_bytes() })
        .map_err(|_| SealError::Open)?;
    // Only `seal` makes this nonce for this plaintext.
    if self::nonce(key, id, &plain) != nonce {
        return Err(SealError::Open);
    }
    Ok(plain)
}

fn nonce(key: &Key, id: &str, plaintext: &[u8]) -> [u8; NONCE_LEN] {
    let mut mac = Blake2bMac::<U24>::new_from_slice(&key.subkey(b"steps vq nonce"))
        .expect("a 32-byte key fits BLAKE2b");
    mac.update(id.as_bytes());
    mac.update(&[0]);
    mac.update(plaintext);
    mac.finalize().into_bytes().into()
}

/// Seals a question for the file: its id and kind stay in clear.
pub fn seal_item(key: &Key, opened: &Opened) -> SealedItem {
    let json = serde_json::to_vec(opened).expect("content types always serialize");
    SealedItem {
        id: opened.item.id.clone(),
        kind: opened.item.body.kind().to_string(),
        sealed: seal(key, &opened.item.id, &json),
    }
}

/// Opens a question from the file. It must be the question the file names:
/// the same id and kind.
pub fn open_item(key: &Key, sealed: &SealedItem) -> Result<Opened, SealError> {
    read_opened(sealed, &open(key, &sealed.id, &sealed.sealed)?)
}

/// The question in `plain`, a blob of `sealed` already opened.
pub fn read_opened(sealed: &SealedItem, plain: &[u8]) -> Result<Opened, SealError> {
    let opened: Opened = serde_json::from_slice(plain).map_err(|e| SealError::Json(e.to_string()))?;
    if opened.item.id != sealed.id {
        return Err(SealError::Id { expected: sealed.id.clone(), found: opened.item.id });
    }
    if opened.item.body.kind() != sealed.kind {
        return Err(SealError::Kind {
            expected: sealed.kind.clone(),
            found: opened.item.body.kind().to_string(),
        });
    }
    Ok(opened)
}

impl SealedLecture {
    /// The file for `lecture`, its questions sorted by id.
    pub fn new(lecture: &str, key: &Key, mut items: Vec<SealedItem>) -> Self {
        items.sort_by(|a, b| a.id.cmp(&b.id));
        Self { schema: SCHEMA.into(), lecture: lecture.into(), key: key.id(), items }
    }
}

impl Opened {
    /// The figure's bytes, when it has one that decodes.
    pub fn image_bytes(&self) -> Option<Vec<u8>> {
        STANDARD.decode(self.image.as_deref()?).ok()
    }

    /// The question with a figure from `bytes` (a WebP file).
    pub fn with_image(item: Item, bytes: Option<&[u8]>) -> Self {
        Self {
            item,
            image: bytes.map(|b| STANDARD.encode(b)),
            image_type: bytes.map(|_| IMAGE_TYPE.to_string()),
            objective: None,
        }
    }

    /// The same question with its educational objective.
    pub fn with_objective(self, objective: Option<String>) -> Self {
        Self { objective, ..self }
    }
}

/// `path.L03.vq0001` → 1: the VQ number of a question id, if it has the
/// `<lecture>.vqNNNN` shape (four digits, from 0001).
pub fn number(id: &str) -> Option<u32> {
    let (lecture, _) = id.rsplit_once('.')?;
    under(id, lecture)
}

/// The VQ number when `id` is `<lecture>.vqNNNN` under exactly `lecture`.
pub fn under(id: &str, lecture: &str) -> Option<u32> {
    let digits = id.strip_prefix(lecture)?.strip_prefix(".vq")?;
    let n: u32 = digits.parse().ok()?;
    (digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit()) && n > 0).then_some(n)
}

/// `path.L03`, 1 → `path.L03.vq0001`.
pub fn id_for(lecture: &str, number: u32) -> String {
    format!("{lecture}.vq{number:04}")
}

/// `VQ`, 1 → `VQ-0001`: how a question-bank item is named on screens (its
/// source's `short`, then its number).
pub fn label(short: &str, page: u32) -> String {
    format!("{short}-{page:04}")
}

/// A WebP file: `RIFF`, a size, `WEBP`.
pub fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

#[cfg(test)]
mod tests;
