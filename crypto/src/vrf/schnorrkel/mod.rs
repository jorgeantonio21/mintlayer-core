use crate::random::{CryptoRng, Rng};
use serialization::{Decode, Encode};

const PUBKEY_LEN: usize = 32;
const PRIVKEY_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct SchnorrkelPublicKey {
    key: schnorrkel::PublicKey,
}

impl SchnorrkelPublicKey {
    #[allow(dead_code)]
    pub fn from_private_key(private_key: &SchnorrkelPrivateKey) -> Self {
        SchnorrkelPublicKey {
            key: private_key.key.to_public(),
        }
    }
}

impl Encode for SchnorrkelPublicKey {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.key.to_bytes().using_encoded(f)
    }

    fn encoded_size(&self) -> usize {
        debug_assert_eq!(self.key.to_bytes().len(), PUBKEY_LEN);
        PUBKEY_LEN
    }
}

impl Decode for SchnorrkelPublicKey {
    fn encoded_fixed_size() -> Option<usize> {
        Some(PUBKEY_LEN)
    }

    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        const ERR_MSG: &str = "Failed to read schnorrkel public key";
        let mut v = [0; PUBKEY_LEN];
        input.read(v.as_mut_slice())?;
        let key = schnorrkel::PublicKey::from_bytes(&v)
            .map_err(|_| parity_scale_codec::Error::from(ERR_MSG))?;
        Ok(Self { key })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct SchnorrkelPrivateKey {
    key: schnorrkel::SecretKey,
}

impl SchnorrkelPrivateKey {
    #[allow(dead_code)]
    pub fn new<R: Rng + CryptoRng>(rng: &mut R) -> (SchnorrkelPrivateKey, SchnorrkelPublicKey) {
        let sk = schnorrkel::SecretKey::generate_with(rng);
        let pk = sk.to_public();
        let sk = Self { key: sk };
        let pk = SchnorrkelPublicKey { key: pk };
        (sk, pk)
    }
}

impl Encode for SchnorrkelPrivateKey {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.key.to_bytes().using_encoded(f)
    }

    fn encoded_size(&self) -> usize {
        debug_assert_eq!(self.key.to_bytes().len(), PRIVKEY_LEN);
        PRIVKEY_LEN
    }
}

impl Decode for SchnorrkelPrivateKey {
    fn encoded_fixed_size() -> Option<usize> {
        Some(PRIVKEY_LEN)
    }

    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        const ERR_MSG: &str = "Failed to read schnorrkel private key";
        let mut v = [0; PRIVKEY_LEN];
        input.read(v.as_mut_slice())?;
        let key = schnorrkel::SecretKey::from_bytes(&v)
            .map_err(|_| parity_scale_codec::Error::from(ERR_MSG))?;
        Ok(Self { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::make_true_rng;
    use hex::FromHex;
    use schnorrkel::{signing_context, Keypair};
    use serialization::{DecodeAll, Encode};

    #[test]
    fn key_serialization() {
        let mut rng = make_true_rng();
        let (sk, pk) = SchnorrkelPrivateKey::new(&mut rng);

        let encoded_sk = sk.encode();
        let encoded_pk = pk.encode();

        assert_eq!(encoded_sk.len(), PRIVKEY_LEN);
        assert_eq!(encoded_pk.len(), PUBKEY_LEN);

        let decoded_sk = SchnorrkelPrivateKey::decode_all(&mut encoded_sk.as_slice()).unwrap();
        let decoded_pk = SchnorrkelPublicKey::decode_all(&mut encoded_pk.as_slice()).unwrap();

        assert_eq!(sk, decoded_sk);
        assert_eq!(pk, decoded_pk);

        let encoded_sk_again = decoded_sk.encode();
        let encoded_pk_again = decoded_pk.encode();

        assert_eq!(encoded_sk, encoded_sk_again);
        assert_eq!(encoded_pk, encoded_pk_again);
    }

    #[test]
    fn fixed_keys() {
        let encoded_sk: Vec<u8> = FromHex::from_hex("414978f2c626250805d5e036249cccae02d6dca262daa8d7a880617da1eeed023effa71123f8172cd5e45b15c92a17fa143aba6010a741353d4dcbe382ae1944").unwrap();
        let encoded_pk: Vec<u8> =
            FromHex::from_hex("86a720458a04160e17441c3622c41933094d28b06a38632933689ec89fa8fb3c")
                .unwrap();

        let decoded_sk = SchnorrkelPrivateKey::decode_all(&mut encoded_sk.as_slice()).unwrap();
        let decoded_pk = SchnorrkelPublicKey::decode_all(&mut encoded_pk.as_slice()).unwrap();

        assert_eq!(
            decoded_pk,
            SchnorrkelPublicKey::from_private_key(&decoded_sk)
        )
    }

    #[test]
    fn vrf_internal_simple() {
        let mut csprng = make_true_rng();

        let keypair1 = Keypair::generate_with(&mut csprng);

        let ctx = signing_context(b"yoo!");
        let msg = b"meow";
        let (io1, proof1, proof1batchable) = keypair1.vrf_sign(ctx.bytes(msg));
        let out1 = &io1.to_preout();
        assert_eq!(
            proof1,
            proof1batchable.shorten_vrf(&keypair1.public, ctx.bytes(msg), out1).unwrap(),
            "Oops `shorten_vrf` failed"
        );
        let (io1too, proof1too) = keypair1
            .public
            .vrf_verify(ctx.bytes(msg), out1, &proof1)
            .expect("Correct VRF verification failed!");
        assert_eq!(
            io1too, io1,
            "Output differs between signing and verification!"
        );
        assert_eq!(
            proof1batchable, proof1too,
            "VRF verification yielded incorrect batchable proof"
        );
        assert_eq!(
            keypair1.vrf_sign(ctx.bytes(msg)).0,
            io1,
            "Rerunning VRF gave different output"
        );

        assert!(
            keypair1.public.vrf_verify(ctx.bytes(b"not meow"), out1, &proof1).is_err(),
            "VRF verification with incorrect message passed!"
        );

        let keypair2 = Keypair::generate_with(&mut csprng);
        assert!(
            keypair2.public.vrf_verify(ctx.bytes(msg), out1, &proof1).is_err(),
            "VRF verification with incorrect signer passed!"
        );
    }
}