//! Per-environment release keying — the reader-side fail-closed
//! enforcement point (riir-reflexer Issue 004 / riir-ai Proposal 054 §6 /
//! Plan 623 T2).
//!
//! **The env binds at verification time through the pin table.** v1
//! vessels carry no env field (the wire table is frozen; changing it is a
//! format bump, deliberately not taken in the `v0` draft), so the binding
//! is the pin SET: a devnet-minting key is simply not in the mainnet pin
//! set, and [`decode_for_env`] / [`open_for_env`] resolve pins ONLY for
//! the env the caller names. A vessel that verifies in one env is not
//! thereby valid in another — cross-env trust is structural, not policy.
//!
//! **Unarmed byte-identity (the delegated-verdict condition).** The
//! default [`EnvPinTable`] carries the compiled [`crate::default_pins`]
//! root under [`Env::None`] and NOTHING anywhere else; [`crate::decode`]
//! / [`crate::open`] signatures and behavior are untouched — the new
//! paths are additive entry points a vessel load only reaches when a
//! caller names an env. An unset env slot refuses
//! [`VesselError::EnvUnpinned`] (a config gap is a loud refusal naming
//! the env, never a silent fallback to the root), so a table built with
//! [`EnvPinTable::empty`] can never accidentally serve the root set.

use crate::{PinTable, VesselError, default_pins};
use std::fmt;
use std::path::Path;

/// The serving environment a vessel is verified FOR — the reader-side
/// vocabulary of the manifest schema's `env` field (`none | localnet |
/// devnet | testnet | mainnet`). Discriminants are pinned (`u8::from`)
/// so the vocabulary cannot silently reorder under a tidy-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Env {
    /// Env-independent (public release bins; the compiled trust root).
    None = 0,
    Localnet = 1,
    Devnet = 2,
    Testnet = 3,
    Mainnet = 4,
}

impl Env {
    /// The whole vocabulary, in manifest-schema order.
    pub const ALL: [Env; 5] = [
        Env::None,
        Env::Localnet,
        Env::Devnet,
        Env::Testnet,
        Env::Mainnet,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Env::None => "none",
            Env::Localnet => "localnet",
            Env::Devnet => "devnet",
            Env::Testnet => "testnet",
            Env::Mainnet => "mainnet",
        }
    }
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Env> for u8 {
    fn from(e: Env) -> u8 {
        e as u8
    }
}

/// Parse the manifest vocabulary. Unknown names refuse — never guessed.
impl std::str::FromStr for Env {
    type Err = UnknownEnv;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Env::ALL
            .into_iter()
            .find(|e| e.as_str() == s)
            .ok_or(UnknownEnv(s.to_owned()))
    }
}

/// An `env` name outside the vocabulary (the manifest cross-check's
/// refusal; the message names the offending string).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownEnv(pub String);

impl fmt::Display for UnknownEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown env {:?} (vocabulary: none|localnet|devnet|testnet|mainnet)",
            self.0
        )
    }
}

impl std::error::Error for UnknownEnv {}

/// Per-env trust anchors — one slot per [`Env`].
///
/// [`EnvPinTable::default`] is the COMPILED posture: the [`Env::None`]
/// slot holds the compiled [`default_pins`] root (the env-independent
/// trust root exists by construction — `DEFAULT_PIN_KEYS` is
/// compile-asserted non-empty) and every named env starts UNSET.
/// [`EnvPinTable::empty`] is the all-unset table — every env refuses
/// until pinned.
///
/// Populate a named env only with a measured citation (the
/// `contender_exempt_names.txt` law, applied to trust anchors): a pin
/// row says "this key mints for THIS env", and it is installed in the
/// same change that starts minting for that env.
#[derive(Clone)]
pub struct EnvPinTable {
    slots: [(Env, Option<PinTable>); Env::ALL.len()],
}

impl Default for EnvPinTable {
    /// The compiled posture: root in [`Env::None`], named envs unset.
    fn default() -> Self {
        Self {
            slots: [
                (Env::None, Some(default_pins())),
                (Env::Localnet, None),
                (Env::Devnet, None),
                (Env::Testnet, None),
                (Env::Mainnet, None),
            ],
        }
    }
}

impl EnvPinTable {
    /// Every slot unset — every [`decode_for_env`] refuses until pinned.
    pub fn empty() -> Self {
        Self {
            slots: Env::ALL.map(|e| (e, None)),
        }
    }

    /// Pin `table` for `env` (builder form). Setting [`Env::None`]
    /// replaces the compiled root — an operator act, never a default
    /// (the wildcard law: explicit, never implicit).
    pub fn with_pins(mut self, env: Env, table: PinTable) -> Self {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.0 == env) {
            slot.1 = Some(table);
        }
        self
    }

    /// The pin set for `env` — fail-closed: an unset env is
    /// [`VesselError::EnvUnpinned`], never a silent fallback to the root.
    pub fn pins_for(&self, env: Env) -> Result<&PinTable, VesselError> {
        self.slots
            .iter()
            .find(|s| s.0 == env)
            .map(|s| s.1.as_ref())
            .expect("all five envs are slotted")
            .ok_or(VesselError::EnvUnpinned { env })
    }
}

impl fmt::Debug for EnvPinTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Key MATERIAL is never debug-printed; only slot occupancy.
        let mut m = f.debug_struct("EnvPinTable");
        for (env, slot) in &self.slots {
            let state = match slot {
                Some(t) if t.is_empty() => "empty",
                Some(_) => "pinned",
                None => "unset",
            };
            m.field(env.as_str(), &state);
        }
        m.finish()
    }
}

/// Verify a whole vessel buffer against the pins of the env it is being
/// served in — the per-env enforcement point. The verification walk is
/// [`crate::decode`]'s, byte for byte; only the pin RESOLUTION differs
/// (the named env's set, fail-closed).
pub fn decode_for_env(
    buf: &[u8],
    pins: &EnvPinTable,
    env: Env,
) -> Result<crate::VerifiedVessel, VesselError> {
    crate::decode(buf, pins.pins_for(env)?)
}

/// [`crate::open`] restricted to one env's pins. The file walk is
/// `open`'s, byte for byte; the env resolution happens first (an unset
/// env refuses before the file is touched).
pub fn open_for_env(
    path: &Path,
    pins: &EnvPinTable,
    env: Env,
) -> Result<crate::VerifiedVessel, VesselError> {
    crate::open(path, pins.pins_for(env)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Class, Header, encode_public};
    use ed25519_dalek::SigningKey;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn table(key_id: u32, k: &SigningKey) -> PinTable {
        PinTable::with_key(key_id, k.verifying_key())
    }

    fn signed(seed: u8, k: &SigningKey, key_id: u32) -> Vec<u8> {
        let header = Header {
            format_version: crate::FORMAT_VERSION,
            class: Class::PublicRelease,
            key_id,
            artifact_version: 1,
            parent_commitment: [0; 32],
            payload_len: 4,
        };
        let _ = &header;
        encode_public(k, key_id, 1, [0; 32], &[seed; 4])
    }

    #[test]
    fn env_discriminants_and_names_are_pinned() {
        assert_eq!(u8::from(Env::None), 0);
        assert_eq!(u8::from(Env::Localnet), 1);
        assert_eq!(u8::from(Env::Devnet), 2);
        assert_eq!(u8::from(Env::Testnet), 3);
        assert_eq!(u8::from(Env::Mainnet), 4);
        for e in Env::ALL {
            let parsed: Env = e.as_str().parse().expect("vocabulary parses");
            assert_eq!(parsed, e);
        }
        assert!("staging".parse::<Env>().is_err(), "unknown env refuses");
        assert_eq!(
            "staging".parse::<Env>().unwrap_err().to_string(),
            "unknown env \"staging\" (vocabulary: none|localnet|devnet|testnet|mainnet)"
        );
    }

    #[test]
    fn default_table_root_under_none_every_named_env_refuses_loud() {
        let t = EnvPinTable::default();
        assert!(t.pins_for(Env::None).is_ok());
        for e in [Env::Localnet, Env::Devnet, Env::Testnet, Env::Mainnet] {
            let msg = match t.pins_for(e) {
                Err(err @ VesselError::EnvUnpinned { env }) => {
                    assert_eq!(env, e, "{e} must refuse unset");
                    err.to_string()
                }
                Ok(_) => panic!("{e} must refuse unset"),
                Err(other) => panic!("{e} refused with the wrong class: {other:?}"),
            };
            assert!(msg.contains(e.as_str()), "refusal names the env: {msg}");
        }
    }

    #[test]
    fn empty_table_refuses_everything_including_none() {
        let t = EnvPinTable::empty();
        for e in Env::ALL {
            assert!(matches!(t.pins_for(e), Err(VesselError::EnvUnpinned { .. })));
        }
    }

    #[test]
    fn with_pins_sets_only_the_named_slot() {
        let t = EnvPinTable::default().with_pins(Env::Devnet, table(9, &key(7)));
        assert!(t.pins_for(Env::Devnet).is_ok());
        for e in [Env::Localnet, Env::Testnet, Env::Mainnet] {
            assert!(matches!(t.pins_for(e), Err(VesselError::EnvUnpinned { .. })));
        }
    }

    #[test]
    fn cross_env_trust_is_structural() {
        let dev_key = key(11);
        let main_key = key(12);
        let dev_vessel = signed(1, &dev_key, 9);
        let main_vessel = signed(2, &main_key, 9);

        let devnet_table = EnvPinTable::default().with_pins(Env::Devnet, table(9, &dev_key));
        let mainnet_table = EnvPinTable::default().with_pins(Env::Mainnet, table(9, &main_key));

        // A devnet-minted vessel verifies in devnet and NEVER in mainnet
        // (and vice versa) — the pin set IS the env binding.
        assert!(decode_for_env(&dev_vessel, &devnet_table, Env::Devnet).is_ok());
        assert!(decode_for_env(&main_vessel, &mainnet_table, Env::Mainnet).is_ok());
        // Same key-id, different minting key in the other env: the
        // substitution is caught by the strict signature.
        assert_eq!(
            decode_for_env(&dev_vessel, &mainnet_table, Env::Mainnet).unwrap_err(),
            VesselError::BadSignature
        );
        assert_eq!(
            decode_for_env(&main_vessel, &devnet_table, Env::Devnet).unwrap_err(),
            VesselError::BadSignature
        );
        // And a key-id the target env does not pin at all is UnknownKey.
        let other_env_table = EnvPinTable::default().with_pins(Env::Testnet, table(3, &main_key));
        assert_eq!(
            decode_for_env(&dev_vessel, &other_env_table, Env::Testnet).unwrap_err(),
            VesselError::UnknownKey(9)
        );
    }

    #[test]
    fn unarmed_none_posture_is_byte_identical_to_the_stock_path() {
        // The delegated-verdict condition, as a test: serving through
        // Env::None is EXACTLY the stock decode posture — same accept,
        // same refusal taxonomy — for ANY table the operator supplies.
        let k = key(21);
        let vessel_bytes = signed(3, &k, 1);
        let operator_pins = table(1, &k);

        let stock = crate::decode(&vessel_bytes, &operator_pins).expect("stock decode");
        let via_env = decode_for_env(
            &vessel_bytes,
            &EnvPinTable::empty().with_pins(Env::None, table(1, &k)),
            Env::None,
        )
        .expect("env decode");
        assert_eq!(stock, via_env);
        assert_eq!(stock.commitment(), via_env.commitment());

        // Refusal taxonomy: a key-id the table does not pin refuses
        // identically both ways.
        let wrong = table(2, &k); // vessel carries key-id 1
        assert_eq!(
            crate::decode(&vessel_bytes, &wrong).unwrap_err(),
            VesselError::UnknownKey(1)
        );
        let env_err = decode_for_env(
            &vessel_bytes,
            &EnvPinTable::empty().with_pins(Env::None, table(2, &k)),
            Env::None,
        )
        .unwrap_err();
        assert!(matches!(env_err, VesselError::UnknownKey(1)), "got: {env_err:?}");
    }

    #[test]
    fn unset_env_refuses_before_the_file_is_touched() {
        let k = key(31);
        let vessel_bytes = signed(4, &k, 1);
        let tmp = std::env::temp_dir().join(format!(
            "reflexer_vessel_env_unpinned_{}.rflexvsl",
            std::process::id()
        ));
        std::fs::write(&tmp, &vessel_bytes).expect("write fixture");
        let err = open_for_env(&tmp, &EnvPinTable::default(), Env::Devnet).unwrap_err();
        assert!(matches!(err, VesselError::EnvUnpinned { env: Env::Devnet }));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn open_for_env_none_matches_open_with_the_same_table() {
        let k = key(41);
        let vessel_bytes = signed(5, &k, 1);
        let tmp = std::env::temp_dir().join(format!(
            "reflexer_vessel_env_open_{}.rflexvsl",
            std::process::id()
        ));
        std::fs::write(&tmp, &vessel_bytes).expect("write fixture");
        let pins = table(1, &k);
        let stock = crate::open(&tmp, &pins).expect("stock open");
        let via_env = open_for_env(&tmp, &EnvPinTable::empty().with_pins(Env::None, table(1, &k)), Env::None)
            .expect("env open");
        assert_eq!(stock, via_env);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn debug_never_carries_key_material() {
        let k = key(51);
        let t = EnvPinTable::default().with_pins(Env::Mainnet, table(1, &k));
        let rendered = format!("{t:?}");
        let hex: String = k
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(!rendered.contains(&hex), "key bytes leaked: {rendered}");
        assert!(rendered.contains("pinned"));
        assert!(rendered.contains("unset"));
    }
}
