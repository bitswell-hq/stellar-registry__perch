//! Seed-key handling. Seeds arrive ONLY via environment variables — argv leaks
//! through `ps`, so no CLI flag ever carries key material. One S… seed yields
//! both the auth-entry signer (raw ed25519 over the OZ digest) and the
//! fee-paying G-account for the envelope.
//!
//! Every signature goes through [`SignKey`]. A dry run is given a
//! [`PublicKeyOnly`], which knows the key's public half and cannot sign, so a
//! dry run never reads the seed's environment variable at all.

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer as _, SigningKey};

/// A key that signs auth entries and the fee envelope.
pub trait SignKey {
    /// The 32-byte ed25519 public key.
    fn public(&self) -> [u8; 32];
    /// An ed25519 signature over `message`.
    fn sign(&self, message: &[u8]) -> Result<[u8; 64]>;
    /// The G… account of [`SignKey::public`].
    fn account(&self) -> String {
        g_strkey(&self.public())
    }
}

fn g_strkey(public: &[u8; 32]) -> String {
    // strkey 0.0.16 returns a heapless string; widen to an owned String.
    stellar_strkey::ed25519::PublicKey(*public)
        .to_string()
        .as_str()
        .into()
}

/// The public half of a signing key, for a dry run: it names the fee payer
/// and the signer the account's rule is matched against, and refuses to sign.
pub struct PublicKeyOnly {
    public: [u8; 32],
}

impl PublicKeyOnly {
    /// From a G… strkey.
    pub fn from_strkey(g: &str) -> Result<Self> {
        match stellar_strkey::Strkey::from_string(g.trim()) {
            Ok(stellar_strkey::Strkey::PublicKeyEd25519(k)) => Ok(Self { public: k.0 }),
            _ => bail!("not a G… ed25519 public key: {g}"),
        }
    }
}

impl SignKey for PublicKeyOnly {
    fn public(&self) -> [u8; 32] {
        self.public
    }
    fn sign(&self, _message: &[u8]) -> Result<[u8; 64]> {
        bail!("a dry run holds no signing key and signs nothing")
    }
}

/// The key a command signs with: the seed in `var` for a live run, or only
/// the public key `public` for a dry run, which never reads `var`. `public`
/// is refused on a live run.
pub fn signing_key(var: &str, public: Option<&str>, dry_run: bool) -> Result<Box<dyn SignKey>> {
    if dry_run {
        let public = public.with_context(|| {
            format!("--dry-run needs --public-key G… (a dry run never reads {var})")
        })?;
        return Ok(Box::new(PublicKeyOnly::from_strkey(public)?));
    }
    if public.is_some() {
        bail!("--public-key goes with --dry-run; a live run signs with {var}");
    }
    Ok(Box::new(SeedKey::from_env(var)?))
}

pub struct SeedKey {
    pub signing: SigningKey,
    /// The 32-byte ed25519 public key derived from the seed.
    pub public: [u8; 32],
}

impl SeedKey {
    pub fn from_env(var: &str) -> Result<Self> {
        let seed = std::env::var(var)
            .with_context(|| format!("{var} not set (expects an S… ed25519 seed, env only)"))?;
        let sk = match stellar_strkey::Strkey::from_string(seed.trim()) {
            Ok(stellar_strkey::Strkey::PrivateKeyEd25519(k)) => k,
            Ok(_) => bail!("{var} is a strkey but not an S… ed25519 seed"),
            Err(e) => bail!("{var} does not parse as a strkey: {e:?}"),
        };
        let signing = SigningKey::from_bytes(&sk.0);
        let public = signing.verifying_key().to_bytes();
        Ok(Self { signing, public })
    }

    /// A key from raw seed bytes, for host tools that generate their own
    /// throwaway testnet keys.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let signing = SigningKey::from_bytes(&seed);
        let public = signing.verifying_key().to_bytes();
        Self { signing, public }
    }

    /// The G… account funded to pay fees — same key pair as the auth signer.
    pub fn account(&self) -> String {
        g_strkey(&self.public)
    }
}

impl SignKey for SeedKey {
    fn public(&self) -> [u8; 32] {
        self.public
    }
    fn sign(&self, message: &[u8]) -> Result<[u8; 64]> {
        Ok(self.signing.sign(message).to_bytes())
    }
}
