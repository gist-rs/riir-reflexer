//! reflexer-vessel — the public decision-artifact format (Plan 002 / P3).
//!
//! ONE format, three laws (`.proposals/001` §"What ships here"):
//!
//! 1. **Whole snapshot, never a blend.** The payload is opaque verified
//!    bytes at this layer; the engine layer binds it to
//!    `Genome::to_line`/`from_line`. Applying a vessel swaps the genome
//!    WHOLE — blends do not preserve move rankings.
//! 2. **Two classes, one refusal.** PUBLIC-RELEASE runs anywhere
//!    (extractable, accepted). HOSTED-ONLY: the class bit exists so the
//!    reader refuses it on uncontrolled hardware — an ACCIDENT GUARD and
//!    audit signal, not encryption (there is no ciphertext at this
//!    layer; the private lane's encryption-at-rest is the actual wall).
//!    The real enforcement is DISTRIBUTION: a hosted-only file never
//!    leaves the hosted environment. No path in this repo WRITES class 1,
//!    and the class bit lives INSIDE the signed header — it cannot be
//!    flipped without breaking the signature.
//! 3. **Keys rotate, unknown fails closed.** A signature under an
//!    unknown or revoked key-id never opens the vessel.
//!
//! ## Wire format (v1)
//!
//! ```text
//! off  size  field
//! 0    8     magic            b"RFLEXVSL"
//! 8    4     format_version   u32 LE (this crate: 1)
//! 12   4     flags            u32 LE — bit0 class (0 PUBLIC, 1 HOSTED-ONLY);
//!                                all other bits reserved-0 (nonzero fails closed)
//! 16   4     key_id           u32 LE (pin-table identity of the minting key)
//! 20   8     artifact_version u64 LE (monotonic per lineage)
//! 28   32    parent_commitment blake3 of the parent's signed region (zeros = genesis)
//! 60   8     payload_len      u64 LE
//! 68   64    signature        ed25519-STRICT over bytes[0..68] || payload
//! 132  n     payload          (payload_len bytes; <= MAX_PAYLOAD)
//! ```
//!
//! `commitment(vessel)` = blake3(bytes[0..68] || payload) — the signed
//! region, so the lineage chain and the signature cover the same bytes.
//!
//! ## Security posture (Plan 002 §Security posture — binding)
//!
//! - **Single-read discipline:** callers hand `decode` one buffer (or use
//!   [`open`], which reads once, bounded); hash/verify/apply see the SAME
//!   bytes — no verify-then-re-read TOCTOU window.
//! - **Verify-before-parse:** structure and length caps are checked before
//!   any signature work, and the signature is checked before the payload is
//!   handed out; unverified input never reaches a payload parser.
//! - **Strict signatures:** `VerifyingKey::verify_strict` (canonical;
//!   malleable / small-order keys rejected).
//! - **Monotonic apply:** [`VerifiedVessel::check_monotonic`] refuses
//!   older-than-current artifacts (the replay/downgrade arm) and
//!   [`VerifiedVessel::check_floor`] enforces the compiled-in release
//!   floor ([`MIN_ARTIFACT_VERSION`]); the force path is an operator
//!   action and must log.
//! - **Unknown anything fails closed:** magic, format version, flag bits,
//!   key-id, class, lineage — an unreadable vessel is a refused vessel.
//! - **Per-env release keying** (the `env` module): a vessel is verified
//!   against ONLY the pin set of the env it is served in — a devnet-signed
//!   vessel never satisfies a mainnet pin; an unpinned env refuses loud
//!   (naming the env), never a silent fallback to the env-independent root.
//!
//! Minting/production tooling for the PRIVATE artifacts lives in riir-train
//! (private, forever — no writer for class 1 exists anywhere in this
//! repo). The PUBLIC-RELEASE writer ships HERE ([`writer`] + the `reflexer
//! sign` subcommand): the public format repo owns the public-class writer,
//! so the public can mint vessels for THEIR OWN artifacts on the same
//! carrier — the moat is the artifacts and the improvement loop, not the
//! box.
//!
//! ## Reader capability features (A1: bytes are runtime; capability is
//! compile-time)
//!
//! `vessel_public_read` / `vessel_hosted_read` (both DEFAULT-ON) compile
//! the reader path for their class. A class whose reader is not compiled
//! refuses [`VesselError::ClassNotReadable`] — at `peek` (structural: the
//! class bit is in the signed header) and post-signature at `decode`. The
//! class REFUSALS are not capabilities: `open`'s `HostedOnlyPath` prefix
//! check and `decode`'s authenticated `HostedOnly` refusal stay
//! unconditional.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::fmt;
use std::path::Path;

/// Re-exported so consumers can name the key types WITHOUT taking their
/// own ed25519-dalek dependency (one version authority: this crate owns
/// the format, and its signature key type is part of that format's
/// surface — `vessel::ed25519_dalek::SigningKey`).
pub use ed25519_dalek;

/// File magic — a vessel starts with these 8 bytes.
pub const MAGIC: [u8; 8] = *b"RFLEXVSL";
/// The format version this crate reads and writes.
pub const FORMAT_VERSION: u32 = 1;
/// The HOSTED-ONLY class bit (flags bit 0). Public so tooling can forge
/// refusal fixtures — no writer path for class 1 exists here.
pub const CLASS_BIT_HOSTED: u32 = 1;
/// Signed-header length (everything before the signature).
pub const HEADER_LEN: usize = 68;
/// Signature length (ed25519).
pub const SIG_LEN: usize = 64;
/// magic..payload start.
pub const PREFIX_LEN: usize = HEADER_LEN + SIG_LEN;
/// Payload ceiling for PUBLIC-RELEASE vessels — refuse larger before any
/// allocation-heavy work. A genome line is hundreds of bytes; 1 MiB is
/// generous by orders of magnitude and bounds hostile inputs on the
/// UNTRUSTED read paths (reflexer-wasm in a browser, `open` on any file
/// handed to this crate).
pub const MAX_PAYLOAD: usize = 1 << 20;
/// Payload ceiling for HOSTED-ONLY vessels — the class-aware half of the
/// hostile-input bound. Hosted vessels carry i8 specialist weights
/// (banking77 is ~9.6 MiB; 16 MiB leaves ~1.6x headroom), and they are
/// only ever read on controlled hardware from our own image — the
/// untrusted-path bound above does not move. The class bit lives in the
/// SIGNED header, which [`peek`] parses before any payload allocation,
/// so the right cap is picked before a byte of payload work.
pub const MAX_HOSTED_PAYLOAD: usize = 1 << 24;

// ── class ────────────────────────────────────────────────────────────────

/// The two artifact classes. The class BIT is public (the read side must
/// be able to refuse HOSTED-ONLY on uncontrolled hardware); the payload
/// behind a HOSTED-ONLY vessel is encrypted at rest on the private lane —
/// no decryption exists anywhere in this repo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    PublicRelease,
    HostedOnly,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PublicRelease => "public-release",
            Self::HostedOnly => "hosted-only",
        }
    }
}

// ── header ───────────────────────────────────────────────────────────────

/// The decoded (structural) header. Field order matches the wire table in
/// the crate docs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub format_version: u32,
    pub class: Class,
    pub key_id: u32,
    pub artifact_version: u64,
    pub parent_commitment: [u8; 32],
    pub payload_len: u64,
}

impl Header {
    fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..8].copy_from_slice(&MAGIC);
        b[8..12].copy_from_slice(&self.format_version.to_le_bytes());
        let flags: u32 = if self.class == Class::HostedOnly {
            CLASS_BIT_HOSTED
        } else {
            0
        };
        b[12..16].copy_from_slice(&flags.to_le_bytes());
        b[16..20].copy_from_slice(&self.key_id.to_le_bytes());
        b[20..28].copy_from_slice(&self.artifact_version.to_le_bytes());
        b[28..60].copy_from_slice(&self.parent_commitment);
        b[60..68].copy_from_slice(&self.payload_len.to_le_bytes());
        b
    }

    fn from_bytes(b: &[u8]) -> Result<Self, VesselError> {
        if b[0..8] != MAGIC {
            return Err(VesselError::BadMagic);
        }
        let format_version = u32::from_le_bytes(b[8..12].try_into().expect("len"));
        if format_version != FORMAT_VERSION {
            return Err(VesselError::UnknownFormatVersion(format_version));
        }
        let flags = u32::from_le_bytes(b[12..16].try_into().expect("len"));
        if flags & !1 != 0 {
            return Err(VesselError::UnknownFlags(flags));
        }
        let class = if flags & CLASS_BIT_HOSTED == 0 {
            Class::PublicRelease
        } else {
            Class::HostedOnly
        };
        Ok(Self {
            format_version,
            class,
            key_id: u32::from_le_bytes(b[16..20].try_into().expect("len")),
            artifact_version: u64::from_le_bytes(b[20..28].try_into().expect("len")),
            parent_commitment: b[28..60].try_into().expect("len"),
            payload_len: u64::from_le_bytes(b[60..68].try_into().expect("len")),
        })
    }
}

// ── errors — the fail-closed taxonomy ────────────────────────────────────

/// Every way a vessel refuses to open. One variant per failure CLASS; all
/// of them close the door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VesselError {
    /// Filesystem failure (missing, unreadable).
    Io(String),
    /// File exceeds the size ceiling (checked from metadata BEFORE the
    /// read in [`open`]).
    TooLarge {
        len: u64,
        cap: u64,
    },
    /// Shorter than a header + signature.
    Truncated {
        len: usize,
    },
    BadMagic,
    UnknownFormatVersion(u32),
    UnknownFlags(u32),
    /// Header payload_len disagrees with the actual bytes following the
    /// signature (file grown, shrunk, or the length field tampered).
    PayloadLenMismatch {
        header: u64,
        file: u64,
    },
    /// payload_len exceeds MAX_PAYLOAD — refused before allocation-heavy
    /// work.
    PayloadTooLarge {
        len: u64,
        cap: usize,
    },
    /// Authentic signature, HOSTED-ONLY class — this hardware class never
    /// opens it (fail-closed; no decryption exists here).
    HostedOnly,
    /// The file's header DECLARES the hosted-only class — refused at the
    /// prefix, before any payload byte is read ([`open`]'s bound stays
    /// the public cap). Unlike [`VesselError::HostedOnly`] this is said
    /// BEFORE the signature is verified: it is a structural fact about
    /// the bytes (the class bit is right there in the header), never an
    /// authenticity claim about what the file is.
    HostedOnlyPath,
    /// key-id is not in the pin table.
    UnknownKey(u32),
    /// key-id is revoked — a once-valid minting key that must never open
    /// anything again.
    RevokedKey(u32),
    /// Signature mismatch (any byte of the signed region changed), or a
    /// non-canonical / weak-key signature (verify_strict).
    BadSignature,
    /// No reader is COMPILED for this class (the `vessel_public_read` /
    /// `vessel_hosted_read` capability features) — refused fail-closed.
    /// Said at `peek` as a STRUCTURAL fact (the class bit is in the signed
    /// header, before any payload work) and never spoofed by a forged
    /// file; `decode` reaches it only post-signature.
    ClassNotReadable {
        class: Class,
    },
    /// The requested env has no pin set (`env` module's per-env release
    /// keying) — a configuration gap, refused loudly naming the env. The
    /// env-independent root (`Env::None` in the default table) always
    /// exists; this is the named-env posture refusing to guess.
    EnvUnpinned {
        env: crate::env::Env,
    },
}

impl fmt::Display for VesselError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use VesselError as E;
        match self {
            E::Io(e) => write!(f, "io: {e}"),
            E::TooLarge { len, cap } => write!(f, "file {len} B exceeds the {cap} B ceiling"),
            E::Truncated { len } => {
                write!(f, "truncated: {len} B < header+signature ({PREFIX_LEN} B)")
            }
            E::BadMagic => write!(f, "bad magic (not a vessel)"),
            E::UnknownFormatVersion(v) => {
                write!(f, "unknown format version {v} (reader: {FORMAT_VERSION})")
            }
            E::UnknownFlags(fl) => write!(f, "unknown flag bits {fl:#x} (reserved bits must be 0)"),
            E::PayloadLenMismatch { header, file } => {
                write!(
                    f,
                    "payload_len mismatch: header says {header}, file carries {file}"
                )
            }
            E::PayloadTooLarge { len, cap } => write!(f, "payload {len} B exceeds cap {cap} B"),
            E::HostedOnly => write!(
                f,
                "hosted-only artifact — refused on uncontrolled hardware (fail-closed)"
            ),
            E::HostedOnlyPath => write!(
                f,
                "file declares the hosted-only class — refused before reading the payload \
                    (hosted vessels open only on the hosted lane)"
            ),
            E::UnknownKey(k) => write!(f, "key-id {k} is not pinned (fail-closed)"),
            E::RevokedKey(k) => write!(f, "key-id {k} is REVOKED (fail-closed)"),
            E::BadSignature => write!(f, "signature failed strict verification"),
            E::ClassNotReadable { class } => write!(
                f,
                "no compiled reader for the {} class (capability feature off) — refused fail-closed",
                class.as_str()
            ),
            E::EnvUnpinned { env } => write!(
                f,
                "no pins for env {} (per-env release keying) — refused fail-closed; \
                 pin a minting key for this env before serving it",
                env
            ),
        }
    }
}

impl std::error::Error for VesselError {}

// ── pin table ────────────────────────────────────────────────────────────

/// The trust anchors: which verifying keys may mint vessels, and which
/// key-ids are revoked. Compiled-in tables rotate by shipping a new build
/// (the standard asymmetry: verify keys are public, signing keys are not).
#[derive(Clone, Default)]
pub struct PinTable {
    keys: Vec<(u32, VerifyingKey)>,
    revoked: Vec<u32>,
    /// An operator-specified trust anchor (the `--vessel-pubkey` posture —
    /// the SEAL `SEAL_VESSEL_PUBKEY` precedent): this key is trusted for
    /// ANY key-id. It is an explicit operator act, never a default.
    wildcard: Option<VerifyingKey>,
}

impl PinTable {
    pub const fn empty() -> Self {
        Self {
            keys: Vec::new(),
            revoked: Vec::new(),
            wildcard: None,
        }
    }

    /// One pinned minting key under its key-id.
    pub fn with_key(key_id: u32, key: VerifyingKey) -> Self {
        let mut t = Self::empty();
        t.keys.push((key_id, key));
        t
    }

    /// Mark a key-id revoked — signatures under it fail closed forever.
    pub fn revoke(mut self, key_id: u32) -> Self {
        self.revoked.push(key_id);
        self
    }

    /// The operator override: trust this key regardless of key-id.
    pub fn with_wildcard(mut self, key: VerifyingKey) -> Self {
        self.wildcard = Some(key);
        self
    }

    /// Resolve the verifying key for a key-id, honoring revocation — the
    /// inspection path (`--vessel-print`): key resolution WITHOUT any
    /// verification claim. Same fail-closed taxonomy as `decode`.
    pub fn resolve_key(&self, key_id: u32) -> Result<&VerifyingKey, VesselError> {
        if self.revoked.contains(&key_id) {
            return Err(VesselError::RevokedKey(key_id));
        }
        if let Some((_, k)) = self.keys.iter().find(|(id, _)| *id == key_id) {
            return Ok(k);
        }
        match &self.wildcard {
            Some(k) => Ok(k),
            None => Err(VesselError::UnknownKey(key_id)),
        }
    }

    /// True when this table resolves NO key under ANY key-id (no pins,
    /// no wildcard). The per-env gate ([`crate::env`]) and the workspace
    /// artifact gate read this to tell an unpinned env from a pinned one.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.wildcard.is_none()
    }
}

/// The compiled-in rollback floor (the release-time anti-downgrade
/// baseline): the NEWEST shipped artifact's version at release time.
/// Zero while no artifacts exist. Set in the same change that ships an
/// artifact — the release-lag model's compile-time form. A validly-signed
/// vessel OLDER than this floor refuses to boot (the replay/downgrade
/// arm: an attacker who can place a file on the vessel path cannot hand
/// the operator a pulled-back or known-weak old artifact).
pub const MIN_ARTIFACT_VERSION: u64 = 0;

/// The compiled-in minting pins as RAW KEY BYTES (const-constructible;
/// `VerifyingKey::from_bytes` is not const). Carries the AUTHORITY ROOT
/// key-id 1 — the `gist-rs vessel-sign v1` derivation from the M3 root
/// (Plan 009 P1 / Issue 003; .docs/07 §6 in riir-rethink). A fork that
/// mints its own key and passes it via `--vessel-pubkey` is refused by
/// the release posture (the wildcard is `dev_pins`-gated in the bin).
/// Rotate by shipping key-id 2 and revoking 1 — never by editing this
/// entry in place (a removed pin strands every artifact under it).
pub const DEFAULT_PIN_KEYS: [(u32, [u8; 32]); 1] = [
    (
        1,
        [
            0xd3, 0x90, 0xc0, 0xae, 0x1e, 0xd9, 0x0f, 0xf0, 0xd7, 0x32, 0xfd, 0xf7, 0xcc, 0xe9,
            0x12, 0xa0, 0x07, 0xf7, 0x0f, 0xd0, 0x55, 0x1c, 0x5c, 0x99, 0x9e, 0x76, 0x9c, 0xfa,
            0xb3, 0x73, 0xc9, 0x9f,
        ],
    ),
];

/// The authority posture is compile-time load-bearing: an emptied pin
/// table would silently turn every `default_pins()` consumer into a
/// wildcard-only posture (Issue 003's defect, reintroduced by a tidy-up).
const _: () = assert!(!DEFAULT_PIN_KEYS.is_empty());

/// The compiled-in pin table the bin verifies against (before any
/// operator `--vessel-pubkey` wildcard joins it).
pub fn default_pins() -> PinTable {
    pins_from_bytes(&DEFAULT_PIN_KEYS)
}

/// Build a pin table from raw verifying-key bytes (the const-friendly
/// form `DEFAULT_PIN_KEYS` stores; bad key bytes are SKIPPED — a compiled
/// table is authored, never hostile).
pub fn pins_from_bytes(keys: &[(u32, [u8; 32])]) -> PinTable {
    let mut t = PinTable::empty();
    for (id, bytes) in keys {
        if let Ok(k) = VerifyingKey::from_bytes(bytes) {
            t.keys.push((*id, k));
        }
    }
    t
}

// ── decode ───────────────────────────────────────────────────────────────

/// Structural inspection without pins: header + signature bytes out, no
/// authenticity claim. The `--vessel-print` path — what is this file?
pub fn peek(buf: &[u8]) -> Result<(Header, [u8; SIG_LEN]), VesselError> {
    if buf.len() < PREFIX_LEN {
        return Err(VesselError::Truncated { len: buf.len() });
    }
    let header = Header::from_bytes(&buf[0..HEADER_LEN])?;
    // The reader-capability gate (compile-time, both features default-on):
    // a class whose reader this build did not compile refuses here — a
    // STRUCTURAL fact (the class bit is in the signed header), before any
    // payload byte is read, never an authenticity claim.
    #[cfg(not(feature = "vessel_hosted_read"))]
    if header.class == Class::HostedOnly {
        return Err(VesselError::ClassNotReadable {
            class: header.class,
        });
    }
    #[cfg(not(feature = "vessel_public_read"))]
    if header.class == Class::PublicRelease {
        return Err(VesselError::ClassNotReadable {
            class: header.class,
        });
    }
    // The class-aware cap: the class bit is IN the header just parsed, so
    // the bound is picked before any payload work. A HOSTED-ONLY vessel
    // may carry up to [`MAX_HOSTED_PAYLOAD`] (specialist weights); the
    // PUBLIC-RELEASE bound stays [`MAX_PAYLOAD`] — a size legal for a
    // hosted vessel is still refused here when the header says public.
    let cap = match header.class {
        Class::HostedOnly => MAX_HOSTED_PAYLOAD,
        Class::PublicRelease => MAX_PAYLOAD,
    };
    if header.payload_len > cap as u64 {
        return Err(VesselError::PayloadTooLarge {
            len: header.payload_len,
            cap,
        });
    }
    let actual = (buf.len() - PREFIX_LEN) as u64;
    if header.payload_len != actual {
        return Err(VesselError::PayloadLenMismatch {
            header: header.payload_len,
            file: actual,
        });
    }
    let sig: [u8; SIG_LEN] = buf[HEADER_LEN..PREFIX_LEN].try_into().expect("len");
    Ok((header, sig))
}

fn signed_message(buf: &[u8]) -> Vec<u8> {
    // The signature's message: header || payload (contiguous — the file
    // interleaves the signature between them). Bounded: payload_len was
    // structurally validated against the file and the cap BEFORE this
    // allocation, so unverified input can never force a large one.
    let mut msg = Vec::with_capacity(HEADER_LEN + (buf.len() - PREFIX_LEN));
    msg.extend_from_slice(&buf[0..HEADER_LEN]);
    msg.extend_from_slice(&buf[PREFIX_LEN..]);
    msg
}

/// blake3 of the signed region — the vessel's commitment (the lineage
/// chain's link). Public so minters can chain children.
pub fn commitment_of(header: &Header, payload: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(&header.to_bytes());
    h.update(payload);
    *h.finalize().as_bytes()
}

/// A fully verified vessel. Constructed only by [`decode`] — there is no
/// other way to build one, so a `VerifiedVessel` is proof of authenticity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedVessel {
    header: Header,
    payload: Vec<u8>,
    commitment: [u8; 32],
}

impl VerifiedVessel {
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The verified payload bytes (the genome line for this engine).
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// blake3 of the signed region — pin this alongside results (the
    /// measurement law's per-row artifact pin).
    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }

    pub fn commitment_hex(&self) -> String {
        hex32(&self.commitment)
    }

    /// The rollback-floor gate (the release-time anti-downgrade arm): the
    /// offered artifact must not be older than `min`. Complements
    /// [`VerifiedVessel::check_monotonic`]: the floor is the RELEASE's
    /// baseline (no prior state needed — boot-time), the monotonic gate
    /// binds a prior apply-state (runtime swap / re-apply). The bin runs
    /// BOTH.
    pub fn check_floor(&self, min: u64) -> Result<(), ApplyRefusal> {
        if self.header.artifact_version < min {
            return Err(ApplyRefusal::OlderThanCurrent {
                current: min,
                offered: self.header.artifact_version,
            });
        }
        Ok(())
    }

    /// The monotonic-apply gate (the replay/downgrade arm of the security
    /// posture): refuse an artifact OLDER than the current state; allow an
    /// exact re-apply (idempotent); refuse same-version-different-content
    /// (a lineage fork). The FORCE path is the operator's call and must
    /// log — it lives at the bin layer, never here.
    pub fn check_monotonic(&self, current: &ApplyState) -> Result<(), ApplyRefusal> {
        use ApplyRefusal as R;
        if self.header.artifact_version < current.artifact_version {
            return Err(R::OlderThanCurrent {
                current: current.artifact_version,
                offered: self.header.artifact_version,
            });
        }
        if self.header.artifact_version == current.artifact_version
            && self.commitment != current.commitment
        {
            return Err(R::VersionFork {
                version: current.artifact_version,
            });
        }
        Ok(())
    }
}

/// The refusal taxonomy of the monotonic gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyRefusal {
    /// Replay/downgrade: the offered artifact is older than the current
    /// one.
    OlderThanCurrent { current: u64, offered: u64 },
    /// Same version, different content — a lineage fork, not a successor.
    VersionFork { version: u64 },
}

impl fmt::Display for ApplyRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OlderThanCurrent { current, offered } => write!(
                f,
                "downgrade refused: artifact v{offered} < current v{current} (force is an operator action and logs)"
            ),
            Self::VersionFork { version } => write!(
                f,
                "lineage fork refused: v{version} with a different commitment than current"
            ),
        }
    }
}

/// The apply-side state: what artifact the engine is CURRENTLY serving.
/// The compiled substrate is the genesis state (version 0, zero
/// commitment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyState {
    pub artifact_version: u64,
    pub commitment: [u8; 32],
}

/// The compiled-in substrate as apply-state baseline (v0, genesis).
pub const SUBSTRATE: ApplyState = ApplyState {
    artifact_version: 0,
    commitment: [0u8; 32],
};

/// Verify a whole vessel buffer against the pin table — the single-read
/// entry point. Order of checks (the security-posture law): structure and
/// length caps → pin resolution (fail-closed on unknown/revoked) → STRICT
/// signature → class refusal. A hosted-only refusal is only ever said
/// about an AUTHENTIC vessel (the signature is checked first), so the
/// message cannot be spoofed by a forged file.
pub fn decode(buf: &[u8], pins: &PinTable) -> Result<VerifiedVessel, VesselError> {
    let (header, sig_bytes) = peek(buf)?;
    let key = pins.resolve_key(header.key_id)?;
    let sig = Signature::from_bytes(&sig_bytes);
    key.verify_strict(&signed_message(buf), &sig)
        .map_err(|_| VesselError::BadSignature)?;
    // The reader-capability gate, post-signature (an authentic public
    // vessel still cannot be READ by a build that compiled no public
    // reader). The hosted refusal below is NOT gated: it is the class's
    // own refusal behavior, and by the time it runs the signature is
    // verified — the message cannot be spoofed by a forged file.
    #[cfg(not(feature = "vessel_public_read"))]
    if header.class == Class::PublicRelease {
        return Err(VesselError::ClassNotReadable {
            class: header.class,
        });
    }
    if header.class == Class::HostedOnly {
        return Err(VesselError::HostedOnly);
    }
    let payload = buf[PREFIX_LEN..].to_vec();
    let commitment = commitment_of(&header, &payload);
    Ok(VerifiedVessel {
        header,
        payload,
        commitment,
    })
}

/// Read once, bounded (the single-read law): regular-files only, then a
/// `take(cap+1)` read that can never allocate past the ceiling even if
/// the file grows mid-flight — no stat-then-read window at all. A FIFO or
/// device on the vessel path is refused, not hung on.
///
/// The prefix is read FIRST and the class checked BEFORE the payload
/// read: a file declaring the hosted-only class is refused at 132 bytes
/// ([`VesselError::HostedOnlyPath`], a structural fact, never an
/// authenticity claim — [`decode`]'s authenticated [`VesselError::HostedOnly`
/// refusal keeps its own law), so this path's allocation bound stays the
/// PUBLIC cap even though hosted vessels may be far larger.
pub fn open(path: &Path, pins: &PinTable) -> Result<VerifiedVessel, VesselError> {
    use std::io::Read as _;
    // PRE-open regular-file check: on Unix, File::open on a FIFO blocks
    // until a writer appears — the check must happen BEFORE the open, or
    // a FIFO on the vessel path hangs boot (found by live probe in the
    // posture review, round 2). Symlink note: fs::metadata FOLLOWS
    // symlinks, so a symlink to a regular file is fine (intended).
    if !std::fs::metadata(path)
        .map_err(|e| VesselError::Io(e.to_string()))?
        .is_file()
    {
        return Err(VesselError::Io(format!(
            "{} is not a regular file (vessels are files, not streams)",
            path.display()
        )));
    }
    let mut file = std::fs::File::open(path).map_err(|e| VesselError::Io(e.to_string()))?;
    if !file
        .metadata()
        .map_err(|e| VesselError::Io(e.to_string()))?
        .is_file()
    {
        return Err(VesselError::Io(format!(
            "{} is not a regular file (vessels are files, not streams)",
            path.display()
        )));
    }
    // The prefix first: magic + header + signature — enough to learn the
    // declared class and refuse the hosted shape without reading payload.
    let mut prefix = vec![0u8; PREFIX_LEN];
    file.read_exact(&mut prefix)
        .map_err(|e| VesselError::Io(e.to_string()))?;
    // A structural parse only — errors here are the ordinary truncated /
    // bad-magic class; the signature check happens later in `decode`.
    if prefix[0..MAGIC.len()] != MAGIC {
        return Err(VesselError::BadMagic);
    }
    let header = Header::from_bytes(&prefix[0..HEADER_LEN])?;
    if header.class == Class::HostedOnly {
        return Err(VesselError::HostedOnlyPath);
    }
    let cap = PREFIX_LEN + MAX_PAYLOAD;
    let mut buf = prefix;
    file.take(cap as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| VesselError::Io(e.to_string()))?;
    if buf.len() > cap {
        return Err(VesselError::TooLarge {
            len: buf.len() as u64,
            cap: cap as u64,
        });
    }
    decode(&buf, pins)
}

// ── encode (the PUBLIC class only) ──────────────────────────────────

/// The PUBLIC-RELEASE writer — key handling, the Result-based mint API,
/// and the round-trip/tamper gates. The FIRST writer in this repo; no
/// path here writes class 1 (HOSTED-ONLY minting stays riir-train).
pub mod writer;

pub mod env;
pub use env::{Env, EnvPinTable, UnknownEnv, decode_for_env, open_for_env};

/// Mint a PUBLIC-RELEASE vessel. There is deliberately NO public encoder
/// for HOSTED-ONLY: no path in this repo writes class 1, ever (Plan 002
/// T3). `parent` is the parent vessel's [`VerifiedVessel::commitment`]
/// (zeros for genesis); `artifact_version` must be strictly greater than
/// the parent's — minting-side lineage discipline is the minter's job.
/// Delegates to [`writer::sign_public`] (one implementation; the assert
/// on the cap became its [`VesselError::PayloadTooLarge`] arm).
pub fn encode_public(
    key: &SigningKey,
    key_id: u32,
    artifact_version: u64,
    parent: [u8; 32],
    payload: &[u8],
) -> Vec<u8> {
    writer::sign_public(key, key_id, artifact_version, parent, payload)
        .expect("payload within the public cap")
        .bytes
}

/// The raw encoder — `pub(crate)` so ONLY this crate's own hostile-forgery
/// tests can construct a signed HOSTED-ONLY vessel (proving the reader's
/// refusal); no shipped path writes class 1.
pub(crate) fn encode_raw(key: &SigningKey, header: Header, payload: &[u8]) -> Vec<u8> {
    let header_bytes = header.to_bytes();
    let mut signed_input = Vec::with_capacity(HEADER_LEN + payload.len());
    signed_input.extend_from_slice(&header_bytes);
    signed_input.extend_from_slice(payload);
    let sig = key.sign(&signed_input);
    let mut out = Vec::with_capacity(PREFIX_LEN + payload.len());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&sig.to_bytes());
    out.extend_from_slice(payload);
    out
}

fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        use std::fmt::Write as _;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn test_pins() -> PinTable {
        PinTable::with_key(1, test_key().verifying_key())
    }

    #[cfg(any(feature = "vessel_public_read", feature = "vessel_hosted_read"))]
    fn payload() -> Vec<u8> {
        // a genome-line-shaped payload (content irrelevant at this layer)
        b"tetris-rulebook-v1 en=0xffff d=4 b=8 sh=1.0 dh=0 w=1;2;3".to_vec()
    }

    #[cfg(feature = "vessel_public_read")]
    fn signed_vessel() -> Vec<u8> {
        encode_public(&test_key(), 1, 5, [0u8; 32], &payload())
    }

    /// A header-plus-zero-signature fixture for the PRE-signature refusals
    /// (magic, format version, flags, truncation — all fail before any key
    /// or class work, so no valid signature is needed and this helper stays
    /// ungated across the reader-capability postures).
    fn structural_public_vessel() -> Vec<u8> {
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::PublicRelease,
            key_id: 1,
            artifact_version: 5,
            parent_commitment: [0u8; 32],
            payload_len: 16,
        };
        let mut v = header.to_bytes().to_vec();
        v.extend_from_slice(&[0u8; SIG_LEN]);
        v.extend_from_slice(&[0u8; 16]);
        v
    }

    // ── happy path ────────────────────────────────────────────────────────

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn roundtrip_public_vessel() {
        let v = signed_vessel();
        let got = decode(&v, &test_pins()).expect("decode");
        assert_eq!(got.header().class, Class::PublicRelease);
        assert_eq!(got.header().key_id, 1);
        assert_eq!(got.header().artifact_version, 5);
        assert_eq!(got.header().parent_commitment, [0u8; 32]);
        assert_eq!(got.payload(), payload());
        assert_eq!(got.commitment(), commitment_of(got.header(), got.payload()));
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn commitment_covers_header_and_payload() {
        let v = signed_vessel();
        let got = decode(&v, &test_pins()).expect("decode");
        // independently: blake3 over header||payload
        let mut h = blake3::Hasher::new();
        h.update(&v[0..HEADER_LEN]);
        h.update(&v[PREFIX_LEN..]);
        assert_eq!(got.commitment(), *h.finalize().as_bytes());
        // and it is NOT blake3 of the raw file (the sig sits between)
        let raw = *blake3::hash(&v).as_bytes();
        assert_ne!(got.commitment(), raw);
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn lineage_chaining() {
        let parent = decode(&signed_vessel(), &test_pins()).unwrap();
        let child = encode_public(&test_key(), 1, 6, parent.commitment(), &payload());
        let got = decode(&child, &test_pins()).expect("child decodes");
        assert_eq!(got.header().parent_commitment, parent.commitment());
        assert_ne!(got.commitment(), parent.commitment());
    }

    // ── structural refusals ───────────────────────────────────────────────

    #[test]
    fn bad_magic_refused() {
        let mut v = structural_public_vessel();
        v[0] = b'X';
        assert_eq!(decode(&v, &test_pins()), Err(VesselError::BadMagic));
    }

    #[test]
    fn unknown_format_version_refused() {
        let mut v = structural_public_vessel();
        v[8..12].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            decode(&v, &test_pins()),
            Err(VesselError::UnknownFormatVersion(2))
        );
    }

    #[test]
    fn unknown_flag_bits_refused() {
        let mut v = structural_public_vessel();
        v[12..16].copy_from_slice(&2u32.to_le_bytes()); // bit1 set
        assert_eq!(decode(&v, &test_pins()), Err(VesselError::UnknownFlags(2)));
    }

    #[test]
    fn every_truncation_length_refused() {
        let v = structural_public_vessel();
        for len in 0..PREFIX_LEN {
            let err = decode(&v[..len], &test_pins());
            assert!(
                matches!(err, Err(VesselError::Truncated { .. })),
                "len {len}: {err:?}"
            );
        }
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn payload_len_mismatch_refused_both_ways() {
        let v = signed_vessel();
        // file grown
        let mut grown = v.clone();
        grown.push(0);
        assert!(matches!(
            decode(&grown, &test_pins()),
            Err(VesselError::PayloadLenMismatch { .. })
        ));
        // length field tampered (also breaks the sig, but the structural
        // check fires first — verify-before-parse)
        let mut tampered = v.clone();
        tampered[60..68].copy_from_slice(&((payload().len() + 1) as u64).to_le_bytes());
        assert!(matches!(
            decode(&tampered, &test_pins()),
            Err(VesselError::PayloadLenMismatch { .. })
        ));
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn oversized_payload_refused_before_allocation() {
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::PublicRelease,
            key_id: 1,
            artifact_version: 1,
            parent_commitment: [0; 32],
            payload_len: MAX_PAYLOAD as u64 + 1,
        };
        let mut v = header.to_bytes().to_vec();
        v.extend_from_slice(&[0u8; SIG_LEN]);
        assert!(matches!(
            decode(&v, &test_pins()),
            Err(VesselError::PayloadTooLarge { .. })
        ));
    }

    // ── the class-aware caps (specialist vessels are HOSTED-ONLY and far
    // larger than the genome-era ceiling; the public bound must not move) ──

    /// A forged (unsigned) header is enough for the CAP checks — peek
    /// makes no authenticity claim.
    #[cfg(any(feature = "vessel_public_read", feature = "vessel_hosted_read"))]
    fn forged(class: Class, payload_len: u64) -> Vec<u8> {
        let header = Header {
            format_version: FORMAT_VERSION,
            class,
            key_id: 1,
            artifact_version: 1,
            parent_commitment: [0; 32],
            payload_len,
        };
        let mut v = header.to_bytes().to_vec();
        v.extend_from_slice(&[0u8; SIG_LEN]);
        v
    }

    #[cfg(feature = "vessel_hosted_read")]
    #[test]
    fn hosted_cap_admits_specialist_sized_payloads_at_peek() {
        // ~10 MB (the banking77 winner's class) passes the HOSTED cap.
        // No 10 MB buffer is allocated — payload_len is forged and the
        // PayloadLenMismatch check is what would fire on a real file; a
        // short buffer with a matching forged len would pass peek fully.
        let len = 10 * 1024 * 1024;
        let mut v = forged(Class::HostedOnly, len as u64);
        v.resize(PREFIX_LEN + len, 0);
        let (header, _) = peek(&v).expect("hosted ~10 MB passes the cap at peek");
        assert_eq!(header.class, Class::HostedOnly);
        // …and decode STILL refuses the class (authenticity first — the
        // dummy signature cannot verify).
        assert_eq!(decode(&v, &test_pins()), Err(VesselError::BadSignature));
    }

    #[cfg(feature = "vessel_hosted_read")]
    #[test]
    fn hosted_cap_ceiling_refused() {
        let v = forged(Class::HostedOnly, MAX_HOSTED_PAYLOAD as u64 + 1);
        assert!(matches!(
            peek(&v),
            Err(VesselError::PayloadTooLarge {
                cap: MAX_HOSTED_PAYLOAD,
                ..
            })
        ));
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn public_cap_stays_at_one_mib_even_for_hosted_legal_sizes() {
        // THE test that proves the public path was not loosened: 1 MiB + 1
        // is legal for a hosted vessel but refused the moment the header
        // says public-release.
        let v = forged(Class::PublicRelease, MAX_PAYLOAD as u64 + 1);
        assert!(matches!(
            peek(&v),
            Err(VesselError::PayloadTooLarge {
                cap: MAX_PAYLOAD,
                ..
            })
        ));
        // and exactly AT the public cap, peek still admits it (the cap is
        // inclusive — one real 1 MiB buffer, the only large allocation in
        // this suite).
        let mut at_cap = forged(Class::PublicRelease, MAX_PAYLOAD as u64);
        at_cap.resize(PREFIX_LEN + MAX_PAYLOAD, 0);
        assert!(
            peek(&at_cap).is_ok(),
            "exactly {MAX_PAYLOAD} B is legal public"
        );
    }

    #[test]
    fn open_refuses_declared_hosted_at_the_prefix_without_reading_payload() {
        // A file whose header DECLARES hosted-only: refused at 132 bytes
        // (HostedOnlyPath — a structural fact, pre-auth), even though its
        // payload_len is far over the public bound. No big file is written:
        // the payload is TRUNCATED, which open must never reach.
        // (The header is forged inline — this refusal is unconditional, so
        // the test runs in every reader-capability posture.)
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::HostedOnly,
            key_id: 1,
            artifact_version: 1,
            parent_commitment: [0; 32],
            payload_len: MAX_HOSTED_PAYLOAD as u64,
        };
        let mut v = header.to_bytes().to_vec();
        v.extend_from_slice(&[0u8; SIG_LEN]);
        let dir = std::env::temp_dir().join(format!(
            "reflexer-vessel-hosted-prefix-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("hosted.vessel");
        v.truncate(PREFIX_LEN + 16); // truncated payload — open must refuse BEFORE caring
        std::fs::write(&p, &v).unwrap();
        assert_eq!(
            open(&p, &test_pins()),
            Err(VesselError::HostedOnlyPath),
            "the prefix refusal must fire before any payload read"
        );
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&dir);
    }

    #[cfg(feature = "vessel_hosted_read")]
    #[test]
    fn open_still_refuses_authentic_hosted_only_after_signature() {
        // The pre-existing law, intact: a fully AUTHENTIC hosted vessel
        // (the crate's own test arm writes class 1) that fits the public
        // bound still reaches decode's authenticated refusal — open's
        // prefix check refuses the DECLARED class first, so here the file
        // is decoded directly to pin the auth-order law on the buffer path.
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::HostedOnly,
            key_id: 1,
            artifact_version: 9,
            parent_commitment: [0; 32],
            payload_len: payload().len() as u64,
        };
        let hosted = encode_raw(&test_key(), header, &payload());
        assert_eq!(decode(&hosted, &test_pins()), Err(VesselError::HostedOnly));
    }

    // ── trust refusals ────────────────────────────────────────────────────

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn unknown_key_refused() {
        let v = signed_vessel();
        let other = PinTable::with_key(99, test_key().verifying_key());
        assert_eq!(decode(&v, &other), Err(VesselError::UnknownKey(1)));
        assert_eq!(
            decode(&v, &PinTable::empty()),
            Err(VesselError::UnknownKey(1))
        );
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn revoked_key_refused() {
        let pins = test_pins().revoke(1);
        let v = signed_vessel();
        assert_eq!(decode(&v, &pins), Err(VesselError::RevokedKey(1)));
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn wildcard_operator_pin_verifies_any_key_id() {
        let v = signed_vessel();
        let pins = PinTable::empty().with_wildcard(test_key().verifying_key());
        assert!(decode(&v, &pins).is_ok());
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn any_byte_tamper_closes_the_door() {
        let v = signed_vessel();
        // Tampering ANYWHERE fails closed. Bytes in the magic fail
        // structurally (verify-before-parse); everything else in the
        // header fails structurally-or-at-signature; the signature and
        // payload regions fail AT the signature (the header still parses,
        // the signed bytes changed).
        let expect = |idx: usize, want: Option<VesselError>| {
            let mut t = v.clone();
            t[idx] ^= 0x01;
            let got = decode(&t, &test_pins());
            assert!(got.is_err(), "tamper at {idx} opened the vessel: {got:?}");
            if let Some(want) = want {
                assert_eq!(got, Err(want), "tamper at {idx}: wrong refusal class");
            }
        };
        expect(4, Some(VesselError::BadMagic)); // magic region
        expect(20, None); // header field — structural class varies by byte
        expect(40, None); // parent commitment
        expect(70, Some(VesselError::BadSignature)); // signature region
        expect(100, Some(VesselError::BadSignature));
        expect(135, Some(VesselError::BadSignature)); // payload region
        expect(140, Some(VesselError::BadSignature));
    }

    // ── the two-class law ─────────────────────────────────────────────────

    #[cfg(feature = "vessel_hosted_read")]
    #[test]
    fn hosted_only_refused_and_only_after_authenticity() {
        // forged by the crate's own test arm (pub(crate) raw encoder) —
        // there is no public writer for class 1
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::HostedOnly,
            key_id: 1,
            artifact_version: 9,
            parent_commitment: [0; 32],
            payload_len: payload().len() as u64,
        };
        let hosted = encode_raw(&test_key(), header, &payload());
        // authentic signature, refused on class
        assert_eq!(decode(&hosted, &test_pins()), Err(VesselError::HostedOnly));
        // tampered hosted-only → BadSignature (authenticity checked FIRST,
        // so the hosted-only message cannot be spoofed by a forged file)
        let mut t = hosted.clone();
        let last = t.len() - 1;
        t[last] ^= 1;
        assert_eq!(decode(&t, &test_pins()), Err(VesselError::BadSignature));
        // unpinned hosted-only → UnknownKey (fail-closed before class)
        assert_eq!(
            decode(&hosted, &PinTable::empty()),
            Err(VesselError::UnknownKey(1))
        );
    }

    // ── monotonic apply ───────────────────────────────────────────────────

    #[cfg(feature = "vessel_public_read")]
    fn state(v: u64) -> ApplyState {
        ApplyState {
            artifact_version: v,
            commitment: [v as u8; 32],
        }
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn monotonic_gate_refuses_downgrades_and_forks() {
        let v5 = decode(&signed_vessel(), &test_pins()).unwrap(); // artifact v5
        assert!(v5.check_monotonic(&SUBSTRATE).is_ok(), "v5 > substrate v0");
        assert!(
            v5.check_monotonic(&ApplyState {
                artifact_version: 5,
                commitment: v5.commitment()
            })
            .is_ok(),
            "exact re-apply is idempotent-allowed"
        );
        assert_eq!(
            v5.check_monotonic(&state(6)),
            Err(ApplyRefusal::OlderThanCurrent {
                current: 6,
                offered: 5
            })
        );
        assert_eq!(
            v5.check_monotonic(&ApplyState {
                artifact_version: 5,
                commitment: [9; 32]
            }),
            Err(ApplyRefusal::VersionFork { version: 5 })
        );
    }

    #[cfg(feature = "vessel_public_read")]
    fn vessel_v(version: u64) -> Vec<u8> {
        encode_public(&test_key(), 1, version, [0u8; 32], &payload())
    }

    // ── the rollback floor (the release-time anti-downgrade arm) ──────

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn floor_refuses_old_validly_signed_artifacts() {
        // The reviewer's exact scenario: floor at 2, an authentic v1
        // offered — must refuse. An old-but-VALID artifact is exactly what
        // an attacker with vessel-path write access hands the operator
        // once newer artifacts exist; signature verification alone is
        // happy to open it.
        let v1 = decode(&vessel_v(1), &test_pins()).unwrap();
        assert_eq!(
            v1.check_floor(2),
            Err(ApplyRefusal::OlderThanCurrent {
                current: 2,
                offered: 1
            })
        );
        assert!(
            v1.check_floor(1).is_ok(),
            "floor == version is allowed (the floor IS a shipped artifact)"
        );
        assert!(
            v1.check_floor(0).is_ok(),
            "floor 0 = today's posture (no artifacts shipped)"
        );
        assert_eq!(
            MIN_ARTIFACT_VERSION, 0,
            "no artifacts shipped — the compiled floor must be 0"
        );
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn compiled_pin_table_resolves_without_any_flag() {
        // The wiring the first-artifact change will use: raw key BYTES in a
        // const array → a table that verifies with NO operator flag. The
        // bin's default_pins() path (pins first, wildcard only adds).
        let table = pins_from_bytes(&[(1, test_key().verifying_key().to_bytes())]);
        assert!(
            decode(&vessel_v(1), &table).is_ok(),
            "a compiled pin must verify with no flag"
        );
        assert!(
            decode(&vessel_v(1), &default_pins()).is_err(),
            "the compiled table carries the AUTHORITY ROOT (Issue 003 T1) — a \
             test/fork key claiming key-id 1 fails the signature gate"
        );
        // a wrong-but-parseable compiled key never verifies (from_bytes
        // accepts any decompressable encoding — nearly all 32-byte strings
        // — so the observable law is the signature failing, and a compiled
        // table is authored, never hostile)
        let wrong = pins_from_bytes(&[(
            1,
            SigningKey::from_bytes(&[9u8; 32])
                .verifying_key()
                .to_bytes(),
        )]);
        assert!(matches!(
            decode(&vessel_v(1), &wrong),
            Err(VesselError::BadSignature)
        ));
    }

    // ── fuzz-lite: deterministic mutation sweep ──────────────────────────

    #[test]
    #[cfg(unix)]
    fn fifo_and_directory_paths_refuse_instead_of_hanging() {
        // The round-2 probe: File::open on a FIFO BLOCKS on Unix, so the
        // regular-file check must run BEFORE the open — assert a FIFO
        // path refuses fast (without the pre-open check this test HANGS;
        // the harness timeout is the backstop, not the defense).
        let fifo =
            std::env::temp_dir().join(format!("reflexer-vessel-fifo-{}", std::process::id()));
        let _ = std::fs::remove_file(&fifo);
        let have_mkfifo = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if have_mkfifo {
            let started = std::time::Instant::now();
            let got = open(&fifo, &test_pins());
            let _ = std::fs::remove_file(&fifo);
            assert!(got.is_err(), "a FIFO must refuse, not open");
            assert!(
                started.elapsed() < std::time::Duration::from_secs(2),
                "refusal took {:?} — the pre-open check did not run first",
                started.elapsed()
            );
        }
        // a directory refuses too (metadata().is_file() is false) — the
        // cfg-portable arm of the same law: non-regular files never
        // reach the read
        let dir = std::env::temp_dir();
        assert!(open(&dir, &test_pins()).is_err(), "a directory must refuse");
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn mutated_vessels_never_panic_and_never_open() {
        let v = signed_vessel();
        // seeded LCG — deterministic, zero deps (cargo-fuzz rides before
        // the first public artifact ships; this is the always-on gate)
        let mut seed: u64 = 0x5eed_cafe_f00d_0001;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed >> 33
        };
        for _ in 0..2000 {
            let idx = (next() as usize) % v.len();
            let mask = 1u8 << (next() % 8);
            let mut t = v.clone();
            t[idx] ^= mask;
            if let Ok(opened) = decode(&t, &test_pins()) {
                panic!(
                    "mutation at {idx} (mask {mask:#x}) produced a VALID vessel {opened:?} — \
                     either the format is broken or the RNG collided with a no-op \
                     (impossible: the bit always flips)"
                );
            }
        }
    }

    // ── the reader-capability axis (both features default-on; these arms
    // compile only in the OFF postures and pin the fail-closed law) ──────

    #[cfg(not(feature = "vessel_public_read"))]
    #[test]
    fn public_reader_not_compiled_refuses_fail_closed() {
        // A fully authentic public vessel: refused at peek (structural —
        // the class bit is in the signed header) before any payload work,
        // and at decode post-signature for a direct caller.
        let p = b"genome-line-v1 no-public-reader posture".to_vec();
        let v = encode_public(&test_key(), 1, 1, [0u8; 32], &p);
        assert_eq!(
            peek(&v),
            Err(VesselError::ClassNotReadable {
                class: Class::PublicRelease
            })
        );
        assert_eq!(
            decode(&v, &test_pins()),
            Err(VesselError::ClassNotReadable {
                class: Class::PublicRelease
            })
        );
    }

    #[cfg(not(feature = "vessel_hosted_read"))]
    #[test]
    fn hosted_reader_not_compiled_refuses_fail_closed() {
        // Crafted with the crate's own test arm (no writer for class 1
        // exists): the hosted READ path is not compiled, so peek refuses
        // structurally — while open's UNCONDITIONAL HostedOnlyPath prefix
        // refusal still stands (the structural refusal is not a feature).
        let p = b"genome-line-v1 no-hosted-reader posture".to_vec();
        let header = Header {
            format_version: FORMAT_VERSION,
            class: Class::HostedOnly,
            key_id: 1,
            artifact_version: 1,
            parent_commitment: [0; 32],
            payload_len: p.len() as u64,
        };
        let hosted = encode_raw(&test_key(), header, &p);
        assert_eq!(
            peek(&hosted),
            Err(VesselError::ClassNotReadable {
                class: Class::HostedOnly
            })
        );
        // decode reaches the same structural refusal first (peek runs
        // inside decode); the authenticated HostedOnly refusal is a
        // hosted-reader-capable build's answer, not this one's.
        assert_eq!(
            decode(&hosted, &test_pins()),
            Err(VesselError::ClassNotReadable {
                class: Class::HostedOnly
            })
        );
    }
}
