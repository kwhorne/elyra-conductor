//! Re-encrypt the Tauri updater signing key under a new password — keeping the
//! keypair, so every shipped build's embedded public key stays valid — and
//! verify signatures against that public key.
//!
//! Why this exists: `tauri signer` can generate a key or sign with one, but not
//! change a key's password, and generating a fresh key would break auto-update
//! for every existing install. The `minisign` crate that Tauri uses keeps its
//! `encrypt` step crate-private, so the re-encryption XOR is done here, using the
//! same scrypt parameter derivation the crate uses. Nothing is written until the
//! result has been re-opened *by the unpatched crate itself*, shown to hold the
//! same secret material and public key, and shown to refuse the empty password.
//!
//! Passwords come from the environment (OLD_PW / NEW_PW), never argv, so they do
//! not appear in `ps`.
//!
//!   OLD_PW='' NEW_PW='…' rekey-signing-key rekey  <key-file> <pub-file> <out-file>
//!                        rekey-signing-key verify <pub-file> <signed-file> <sig-file>
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use minisign::{PublicKey, PublicKeyBox, SecretKey, SecretKeyBox, SignatureBox};
use std::io::Read;
use std::{cmp, env, fs, process};

// Serialized secret-key layout (minisign / rsign), all offsets in bytes.
const SIG_ALG: usize = 0; //  2  "Ed"
const KDF_ALG: usize = 2; //  2  "Sc" (scrypt) or 0,0 (unencrypted)
const CHK_ALG: usize = 4; //  2  "B2"
const KDF_SALT: usize = 6; // 32
const OPSLIMIT: usize = 38; //  8  u64 LE
const MEMLIMIT: usize = 46; //  8  u64 LE
const KEYNUM: usize = 54; //  8
const SK: usize = 62; // 64
const CHK: usize = 126; // 32
const TOTAL: usize = 158;
const N_LOG2_MAX: u8 = 20;
const MEMLIMIT_MAX: usize = 1_073_741_824;
const COMMENT: &str = "rsign encrypted secret key";

fn die(msg: &str) -> ! {
    eprintln!("FAIL: {msg}");
    process::exit(1)
}

/// Tauri stores key files as base64 of the minisign text box.
fn tauri_file(path: &str) -> String {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| die(&format!("{path}: {e}")));
    let bytes = B64
        .decode(raw.trim())
        .unwrap_or_else(|e| die(&format!("{path}: not a Tauri (base64) key file: {e}")));
    String::from_utf8(bytes).unwrap_or_else(|e| die(&format!("{path}: {e}")))
}

fn pubkey(path: &str) -> PublicKey {
    PublicKeyBox::from_string(&tauri_file(path))
        .and_then(|b| b.into_public_key())
        .unwrap_or_else(|e| die(&format!("public key: {e}")))
}

fn u64_le(b: &[u8]) -> u64 {
    u64::from_le_bytes(b.try_into().unwrap())
}

/// Verbatim port of minisign's `raw_scrypt_params` (helpers.rs), so the new key
/// is encrypted with exactly the parameters the crate will derive when opening it.
fn scrypt_params(memlimit: usize, opslimit: u64) -> scrypt::Params {
    let opslimit = cmp::max(32768, opslimit);
    let mut n_log2 = 1u8;
    let r = 8u32;
    let p;
    if opslimit < (memlimit / 32) as u64 {
        p = 1;
        let maxn = opslimit / (u64::from(r) * 4);
        while n_log2 < 63 {
            if 1u64 << n_log2 > maxn / 2 {
                break;
            }
            n_log2 += 1;
        }
    } else {
        let maxn = memlimit as u64 / (u64::from(r) * 128);
        while n_log2 < 63 {
            if 1u64 << n_log2 > maxn / 2 {
                break;
            }
            n_log2 += 1;
        }
        let maxrp = cmp::min(0x3fff_ffff_u32, ((opslimit / 4) / (1u64 << n_log2)) as u32);
        p = maxrp / r;
    }
    if n_log2 > N_LOG2_MAX || memlimit > MEMLIMIT_MAX {
        die("scrypt parameters in the key are out of range");
    }
    scrypt::Params::new(n_log2, r, p, scrypt::Params::RECOMMENDED_LEN)
        .unwrap_or_else(|e| die(&format!("scrypt params: {e}")))
}

fn rekey(key: &str, pubk: &str, out: &str) {
    let old_pw = env::var("OLD_PW").unwrap_or_default();
    let new_pw = env::var("NEW_PW").unwrap_or_else(|_| die("NEW_PW is not set"));
    if new_pw.chars().count() < 16 {
        die("NEW_PW must be at least 16 characters");
    }
    if new_pw == old_pw {
        die("NEW_PW equals OLD_PW");
    }
    let pk = pubkey(pubk);

    // 1. Open with the old password. The crate verifies the checksum, so this
    //    fails loudly if OLD_PW is wrong.
    let plain = SecretKeyBox::from_string(&tauri_file(key))
        .and_then(|b| b.into_secret_key(Some(old_pw)))
        .unwrap_or_else(|e| die(&format!("could not open key with OLD_PW: {e}")));
    let derived = PublicKey::from_secret_key(&plain).unwrap_or_else(|e| die(&format!("{e}")));
    if derived.to_base64() != pk.to_base64() {
        die("the secret key does not correspond to the public key file");
    }

    // 2. Re-encrypt: fresh salt, same ops/mem limits, XOR keynum‖sk‖chk with the
    //    scrypt stream — precisely what the crate's own `encrypt` does.
    let mut bytes = plain.to_bytes();
    if bytes.len() != TOTAL || &bytes[KDF_ALG..KDF_ALG + 2] != b"Sc" {
        die("unexpected key layout");
    }
    let mut salt = [0u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut salt))
        .unwrap_or_else(|e| die(&format!("random salt: {e}")));
    bytes[KDF_SALT..KDF_SALT + 32].copy_from_slice(&salt);
    let opslimit = u64_le(&bytes[OPSLIMIT..OPSLIMIT + 8]);
    let memlimit = u64_le(&bytes[MEMLIMIT..MEMLIMIT + 8]) as usize;
    let params = scrypt_params(memlimit, opslimit);
    let mut stream = [0u8; 8 + 64 + 32];
    scrypt::scrypt(new_pw.as_bytes(), &salt, &params, &mut stream)
        .unwrap_or_else(|e| die(&format!("scrypt: {e}")));
    for (i, b) in bytes[KEYNUM..TOTAL].iter_mut().enumerate() {
        *b ^= stream[i];
    }
    let _ = (SIG_ALG, CHK_ALG, SK, CHK); // layout documentation; offsets above are contiguous
    let encrypted = SecretKey::from_bytes(&bytes).unwrap_or_else(|e| die(&format!("{e}")));
    let text = encrypted
        .to_box(Some(COMMENT))
        .unwrap_or_else(|e| die(&format!("{e}")))
        .into_string();

    // 3. Prove the result before writing it, using the unpatched crate:
    //    opens with NEW_PW, same secret material, same public key, and the
    //    empty password no longer works.
    let back = SecretKeyBox::from_string(&text)
        .and_then(|b| b.into_secret_key(Some(new_pw)))
        .unwrap_or_else(|e| die(&format!("round-trip: crate cannot open the new key: {e}")));
    if back != plain {
        die("round-trip: secret material changed");
    }
    if PublicKey::from_secret_key(&back).unwrap().to_base64() != pk.to_base64() {
        die("round-trip: public key changed");
    }
    if SecretKeyBox::from_string(&text)
        .unwrap()
        .into_secret_key(Some(String::new()))
        .is_ok()
    {
        die("round-trip: the empty password still opens the key");
    }

    fs::write(out, B64.encode(text.as_bytes())).unwrap_or_else(|e| die(&format!("write {out}: {e}")));
    println!("ok: re-encrypted with a fresh salt; keypair unchanged; public key {}", pk.to_base64());
}

fn verify(pubk: &str, file: &str, sig: &str) {
    let pk = pubkey(pubk);
    let raw = fs::read_to_string(sig).unwrap_or_else(|e| die(&format!("{sig}: {e}")));
    // Tauri's .sig files are base64 of the minisign signature text; accept both.
    let text = B64
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or(raw);
    let sb = SignatureBox::from_string(&text).unwrap_or_else(|e| die(&format!("signature: {e}")));
    let f = fs::File::open(file).unwrap_or_else(|e| die(&format!("{file}: {e}")));
    minisign::verify(&pk, &sb, f, true, false, true)
        .unwrap_or_else(|e| die(&format!("signature does NOT verify: {e}")));
    println!("ok: signature over {file} verifies with public key {}", pk.to_base64());
}

fn main() {
    let a: Vec<String> = env::args().collect();
    match (a.get(1).map(String::as_str), a.len()) {
        (Some("rekey"), 5) => rekey(&a[2], &a[3], &a[4]),
        (Some("verify"), 5) => verify(&a[2], &a[3], &a[4]),
        _ => {
            eprintln!("usage:\n  OLD_PW=… NEW_PW=… rekey-signing-key rekey  <key> <pub> <out>\n  rekey-signing-key verify <pub> <file> <sig>");
            process::exit(2)
        }
    }
}
