//! The persisted monotonic apply floor (reflexer Issue 003 T2 — Plan 009
//! P1): what the operator's machine has APPLIED, across restarts.
//!
//! The bin compares every incoming vessel against this floor BEFORE
//! serving it — the compiled `MIN_ARTIFACT_VERSION` is the release-time
//! baseline, this is the runtime one, and together they close the
//! "rollback across restarts" hole (a validly-signed OLD or FORKED
//! artifact swapped onto the vessel path must refuse, not re-apply).
//!
//! Persistence shape: one JSON object, written atomically (tmp + rename,
//! 0600) after a fully successful apply —
//! `{"key_id":1,"artifact_version":3,"commitment":"<64hex>"}`.
//! A CORRUPT state file refuses LOUDLY (exit 1): reading it as genesis
//! would re-open the rollback hole the floor exists to close. The
//! operator's remedy is to inspect and deliberately delete the file,
//! never to have the host silently forget.

use std::io::Write;
use std::path::Path;

/// What has been applied — the floor's truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFloor {
    pub key_id: u32,
    pub artifact_version: u64,
    pub commitment: [u8; 32],
}

/// The floor's refusals (each force-able by the operator's existing
/// `--vessel-force-downgrade`, logged — the audit trail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyFloorError {
    /// An older artifact than the one applied (replay/rollback).
    Downgrade { vessel: u64, applied: u64 },
    /// The same version under different bytes (a fork of a applied artifact).
    Fork { key_id: u32, artifact_version: u64 },
    /// A different key at a non-advanced version — a rotation must ship a
    /// strictly newer artifact; a "new key, same version" is a takeover
    /// shape, not a rotation.
    UnadvancedKeyRotation { vessel_key_id: u32, applied_key_id: u32, artifact_version: u64 },
    /// The state file exists but does not parse — never read as genesis.
    Corrupt(String),
    /// State I/O failed — the floor cannot be established.
    Io(String),
}

impl std::fmt::Display for ApplyFloorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Downgrade { vessel, applied } => write!(
                f,
                "artifact v{vessel} refused — the applied floor is at v{applied} \
                 (rollback across restarts)"
            ),
            Self::Fork { key_id, artifact_version } => write!(
                f,
                "artifact v{artifact_version} under key-id {key_id} refuses — the applied \
                 floor holds a DIFFERENT commitment at the same version (a fork)"
            ),
            Self::UnadvancedKeyRotation { vessel_key_id, applied_key_id, artifact_version } => write!(
                f,
                "key-id {vessel_key_id} refuses — the applied floor holds key-id \
                 {applied_key_id} at v{artifact_version} (a rotation must ship a strictly \
                 newer artifact)"
            ),
            Self::Corrupt(why) => write!(f, "applied-state file corrupt: {why}"),
            Self::Io(why) => write!(f, "applied-state I/O: {why}"),
        }
    }
}

impl std::error::Error for ApplyFloorError {}

impl ApplyFloor {
    /// The gate: does `(key_id, artifact_version, commitment)` clear the
    /// floor? `None` (nothing applied) always clears.
    pub fn check(
        floor: Option<&ApplyFloor>,
        key_id: u32,
        artifact_version: u64,
        commitment: &[u8; 32],
    ) -> Result<(), ApplyFloorError> {
        let Some(f) = floor else { return Ok(()) };
        if artifact_version < f.artifact_version {
            return Err(ApplyFloorError::Downgrade {
                vessel: artifact_version,
                applied: f.artifact_version,
            });
        }
        if artifact_version == f.artifact_version {
            if commitment != &f.commitment {
                return Err(ApplyFloorError::Fork { key_id, artifact_version });
            }
            if key_id != f.key_id {
                return Err(ApplyFloorError::UnadvancedKeyRotation {
                    vessel_key_id: key_id,
                    applied_key_id: f.key_id,
                    artifact_version,
                });
            }
        }
        Ok(())
    }

    /// Read the floor off `path`. Absent = `Ok(None)` (a fresh host);
    /// present-but-unparsable = `Err(Corrupt)` — loud, never genesis.
    pub fn read(path: &Path) -> Result<Option<ApplyFloor>, ApplyFloorError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ApplyFloorError::Io(format!("{}: {e}", path.display()))),
        };
        #[derive(serde::Deserialize)]
        struct Dto {
            key_id: u32,
            artifact_version: u64,
            commitment: String,
        }
        let dto: Dto = serde_json::from_str(&text)
            .map_err(|e| ApplyFloorError::Corrupt(format!("{}: {e}", path.display())))?;
        let mut commitment = [0u8; 32];
        if dto.commitment.len() != 64
            || !dto.commitment.chars().all(|c| c.is_ascii_hexdigit())
            || dto.commitment
                .as_bytes()
                .chunks(2)
                .enumerate()
                .any(|(i, c)| {
                    u8::from_str_radix(std::str::from_utf8(c).unwrap_or("zz"), 16)
                        .map(|b| commitment[i] = b)
                        .is_err()
                })
        {
            return Err(ApplyFloorError::Corrupt(format!(
                "{}: commitment is not 64-hex",
                path.display()
            )));
        }
        Ok(Some(ApplyFloor {
            key_id: dto.key_id,
            artifact_version: dto.artifact_version,
            commitment,
        }))
    }

    /// Persist the floor atomically (tmp + rename, 0600) — called ONLY
    /// after a fully successful apply.
    pub fn write(&self, path: &Path) -> Result<(), ApplyFloorError> {
        #[derive(serde::Serialize)]
        struct Dto<'a> {
            key_id: u32,
            artifact_version: u64,
            commitment: &'a str,
        }
        let dto = Dto {
            key_id: self.key_id,
            artifact_version: self.artifact_version,
            commitment: &self.commitment.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        };
        let text = serde_json::to_string(&dto)
            .map_err(|e| ApplyFloorError::Io(format!("serialize: {e}")))?;
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
            && std::fs::create_dir_all(dir).is_err()
        {
            // A missing parent for a defaulted path is unusual but not
            // fatal to attempt; the open below reports the real error.
        }
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        #[cfg(unix)]
        let f = {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
        };
        #[cfg(not(unix))]
        let f = std::fs::File::create(&tmp);
        let mut f = f.map_err(|e| ApplyFloorError::Io(format!("{}: {e}", tmp.display())))?;
        f.write_all(text.as_bytes())
            .and_then(|()| f.sync_all())
            .map_err(|e| ApplyFloorError::Io(format!("{}: {e}", tmp.display())))?;
        drop(f);
        std::fs::rename(&tmp, path)
            .map_err(|e| ApplyFloorError::Io(format!("{} -> {}: {e}", tmp.display(), path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shared-temp-path law: a FIXED /tmp path races concurrent
    // processes — every fixture carries the pid.
    fn tmp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("reflexer_apply_{name}_{}.json", std::process::id()))
    }

    fn floor(version: u64, commitment: u8) -> Option<ApplyFloor> {
        Some(ApplyFloor {
            key_id: 1,
            artifact_version: version,
            commitment: [commitment; 32],
        })
    }

    #[test]
    fn nothing_applied_always_clears() {
        assert!(ApplyFloor::check(None, 999, 0, &[0u8; 32]).is_ok());
    }

    #[test]
    fn an_older_artifact_is_a_downgrade() {
        let err = ApplyFloor::check(floor(3, 7).as_ref(), 1, 2, &[9u8; 32]).unwrap_err();
        assert_eq!(
            err,
            ApplyFloorError::Downgrade { vessel: 2, applied: 3 }
        );
    }

    #[test]
    fn same_version_different_bytes_is_a_fork() {
        let err = ApplyFloor::check(floor(3, 7).as_ref(), 1, 3, &[9u8; 32]).unwrap_err();
        assert!(matches!(err, ApplyFloorError::Fork { .. }));
    }

    #[test]
    fn a_new_key_at_a_non_advanced_version_is_a_takeover_shape() {
        let err = ApplyFloor::check(floor(3, 7).as_ref(), 2, 3, &[7u8; 32]).unwrap_err();
        assert!(matches!(err, ApplyFloorError::UnadvancedKeyRotation { .. }));
    }

    #[test]
    fn advancement_and_rotation_with_advance_clear() {
        // Same everything: clear.
        assert!(ApplyFloor::check(floor(3, 7).as_ref(), 1, 3, &[7u8; 32]).is_ok());
        // Newer: clear.
        assert!(ApplyFloor::check(floor(3, 7).as_ref(), 1, 4, &[9u8; 32]).is_ok());
        // Rotation shipping a newer artifact: clear.
        assert!(ApplyFloor::check(floor(3, 7).as_ref(), 2, 4, &[9u8; 32]).is_ok());
    }

    #[test]
    fn round_trip_and_absent_and_corrupt() {
        let p = tmp_path("roundtrip");
        let _ = std::fs::remove_file(&p);
        assert_eq!(ApplyFloor::read(&p).unwrap(), None);
        let fl = ApplyFloor { key_id: 1, artifact_version: 5, commitment: [0xAB; 32] };
        fl.write(&p).expect("write");
        assert_eq!(ApplyFloor::read(&p).unwrap(), Some(fl));
        // Corrupt never reads as genesis.
        std::fs::write(&p, b"{not json").unwrap();
        assert!(matches!(ApplyFloor::read(&p), Err(ApplyFloorError::Corrupt(_))));
        // Bad commitment hex likewise.
        std::fs::write(&p, br#"{"key_id":1,"artifact_version":5,"commitment":"zz"}"#).unwrap();
        assert!(matches!(ApplyFloor::read(&p), Err(ApplyFloorError::Corrupt(_))));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn write_is_atomic_no_tmp_litter() {
        let p = tmp_path("atomic");
        let _ = std::fs::remove_file(&p);
        let fl = ApplyFloor { key_id: 2, artifact_version: 9, commitment: [1u8; 32] };
        fl.write(&p).expect("write");
        let tmp = p.with_extension(format!("tmp{}", std::process::id()));
        assert!(!tmp.exists(), "rename must consume the tmp file");
        let _ = std::fs::remove_file(&p);
    }
}
