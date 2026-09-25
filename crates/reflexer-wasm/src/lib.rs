//! The reflexer engine behind a four-export raw ABI, built for
//! `wasm32-wasip1` so ONE module serves both hosts: the browser (the arena's
//! "Reflexer · wasm local" board) and the Cloudflare Worker
//! (`cloudflare/reflexer-worker`). WASI is used only for the clock and
//! stderr; `cloudflare/reflexer-worker/wasi.mjs` is the zero-dependency
//! shim both hosts load.
//!
//! ABI (all lengths in bytes, strings UTF-8 JSON):
//! - `rx_alloc(len) -> ptr` — a buffer the host writes a request into;
//! - `rx_free(ptr, len)` — release a buffer from `rx_alloc` or a result;
//! - `rx_handle(ptr, len) -> u64` — one request line → one envelope line
//!   (`reflexer::serve::envelope_line`, byte-identical to the bin's stdout),
//!   returned packed as `(out_ptr << 32) | out_len`; the INPUT buffer is
//!   consumed (freed) by the call;
//! - `rx_info() -> u64` — `{name, version, proto, genome}`, packed the same.
//!
//! A panic traps (wasm is panic=abort): the host drops the instance and
//! answers `internal` itself — never a half-written envelope.

use reflexer::engine::Engine;
use reflexer::proto::{GENOME_ID, PROTO};
use std::sync::OnceLock;

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(Engine::champion)
}

/// One request line → one envelope line (the safe core behind `rx_handle`).
pub fn handle(line: &str) -> String {
    reflexer::serve::envelope_line(engine(), line.trim(), 1)
}

/// `{name, version, proto, genome}` (the safe core behind `rx_info`).
pub fn info() -> String {
    format!(
        r#"{{"name":"reflexer","version":"{}","proto":{PROTO},"genome":"{GENOME_ID}"}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

/// The raw host ABI. wasm32 ONLY: a result is packed as
/// `(ptr << 32) | len`, which is lossless for 32-bit pointers and would
/// truncate a 64-bit one — so the exports do not exist on native targets.
#[cfg(target_arch = "wasm32")]
mod abi {
    /// Leak `bytes` to the host as a packed `(ptr << 32) | len`.
    fn hand_out(bytes: Vec<u8>) -> u64 {
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let ptr = Box::into_raw(boxed) as *mut u8 as usize;
        ((ptr as u64) << 32) | len as u64
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rx_alloc(len: usize) -> *mut u8 {
        let mut buf = Vec::<u8>::with_capacity(len);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }

    /// # Safety
    /// `ptr`/`len` must come from `rx_alloc(len)` or a packed `rx_handle` /
    /// `rx_info` result, and be freed once.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn rx_free(ptr: *mut u8, len: usize) {
        if !ptr.is_null() {
            // SAFETY: the caller contract — ptr came from this module's
            // allocator with capacity `len`.
            drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
        }
    }

    /// # Safety
    /// `ptr` must come from `rx_alloc(len)` with `len` bytes initialised;
    /// the buffer is consumed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn rx_handle(ptr: *mut u8, len: usize) -> u64 {
        // SAFETY: the caller contract — an rx_alloc buffer, fully written.
        let input = unsafe { Vec::from_raw_parts(ptr, len, len) };
        let out = super::handle(&String::from_utf8_lossy(&input));
        drop(input);
        hand_out(out.into_bytes())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rx_info() -> u64 {
        hand_out(super::info().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_names_the_genome() {
        assert!(info().contains(GENOME_ID));
    }

    #[test]
    fn malformed_line_answers_a_typed_error() {
        let out = handle("{not json");
        assert!(out.contains(r#""code":"bad_json""#), "{out}");
    }
}
