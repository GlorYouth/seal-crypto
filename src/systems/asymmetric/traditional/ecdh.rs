//! Provides implementations for Elliptic Curve Diffie-Hellman (ECDH).
//!
//! This module implements the Elliptic Curve Diffie-Hellman key agreement protocol,
//! which allows two parties to establish a shared secret over an insecure channel.
//! ECDH provides the same security as traditional Diffie-Hellman but with smaller
//! key sizes and better performance.
//!
//! # Supported Curves
//! - **NIST P-256**: Also known as secp256r1, provides ~128 bits of security
//!
//! # Key Agreement Process
//! 1. Each party generates a key pair (private key, public key)
//! 2. Parties exchange public keys over an insecure channel
//! 3. Each party computes the shared secret using their private key and the other's public key
//! 4. Both parties arrive at the same shared secret
//!
//! # Security Properties
//! - Forward secrecy when ephemeral keys are used
//! - Security based on the Elliptic Curve Discrete Logarithm Problem (ECDLP)
//! - Resistant to passive eavesdropping attacks
//! - Vulnerable to man-in-the-middle attacks without authentication
//!
//! # Key Formats
//! Keys are expected to be in PKCS#8 DER format for interoperability with other systems.
//!
//! # Performance Characteristics
//! - Much faster than RSA for equivalent security levels
//! - Smaller key sizes compared to traditional Diffie-Hellman
//! - Efficient implementations available with hardware acceleration
//!
//! # Security Considerations
//! - Use ephemeral keys for forward secrecy
//! - Authenticate the key exchange to prevent man-in-the-middle attacks
//! - Validate public keys to prevent invalid curve attacks
//! - Use the shared secret with a key derivation function (e.g., HKDF)
//! - Consider post-quantum alternatives for long-term security
//!
//! 提供了椭圆曲线迪菲-赫尔曼 (ECDH) 的实现。
//!
//! 此模块实现了椭圆曲线迪菲-赫尔曼密钥协商协议，
//! 它允许两方在不安全的信道上建立共享密钥。
//! ECDH 提供与传统迪菲-赫尔曼相同的安全性，但密钥大小更小，性能更好。
//!
//! # 支持的曲线
//! - **NIST P-256**: 也称为 secp256r1，提供约 128 位的安全性
//!
//! # 密钥协商过程
//! 1. 每一方生成一个密钥对（私钥、公钥）
//! 2. 各方通过不安全信道交换公钥
//! 3. 每一方使用自己的私钥和对方的公钥计算共享密钥
//! 4. 双方得到相同的共享密钥
//!
//! # 安全属性
//! - 使用临时密钥时提供前向保密性
//! - 安全性基于椭圆曲线离散对数问题 (ECDLP)
//! - 抵抗被动窃听攻击
//! - 在没有认证的情况下容易受到中间人攻击
//!
//! # 密钥格式
//! 密钥应为 PKCS#8 DER 格式，以便与其他系统互操作。
//!
//! # 性能特征
//! - 在相同安全级别下比 RSA 快得多
//! - 与传统迪菲-赫尔曼相比密钥大小更小
//! - 可用硬件加速的高效实现
//!
//! # 安全考虑
//! - 使用临时密钥以获得前向保密性
//! - 认证密钥交换以防止中间人攻击
//! - 验证公钥以防止无效曲线攻击
//! - 将共享密钥与密钥派生函数（例如 HKDF）一起使用
//! - 考虑后量子替代方案以获得长期安全性

use crate::errors::Error;
use crate::prelude::*;
use elliptic_curve::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use p256::{NistP256, PublicKey as P256PublicKey, SecretKey, ecdh};
use rand_core_elliptic_curve::OsRng;
use std::convert::TryFrom;
use std::marker::PhantomData;
use zeroize::{Zeroize, Zeroizing};

// ------------------- Marker Structs and Trait for ECDH Parameters -------------------
// ------------------- 用于 ECDH 参数的标记结构体和 Trait -------------------

mod private {
    pub trait Sealed {}
}

/// A trait that defines the parameters for a specific ECDH scheme.
/// This is a sealed trait, meaning only types within this crate can implement it.
///
/// 一个定义特定 ECDH 方案参数的 trait。
/// 这是一个密封的 trait，意味着只有此 crate 中的类型才能实现它。
pub trait EcdhParams: private::Sealed + SchemeParams {
    type Curve: elliptic_curve::Curve + elliptic_curve::PrimeCurveArithmetic;

    fn validate_public_key(bytes: &[u8]) -> Result<(), Error>;
    fn validate_private_key(bytes: &[u8]) -> Result<(), Error>;
}

/// Marker struct for ECDH with NIST P-256 parameters.
///
/// 使用 NIST P-256 参数的 ECDH 的标记结构体。
#[derive(Debug, Default, Clone)]
pub struct EcdhP256Params;
impl private::Sealed for EcdhP256Params {}
impl SchemeParams for EcdhP256Params {
    const NAME: &'static str = "ECDH-P256";
    const ID: u32 = 0x01_01_03_01;
}
impl EcdhParams for EcdhP256Params {
    type Curve = NistP256;

    fn validate_public_key(bytes: &[u8]) -> Result<(), Error> {
        P256PublicKey::from_public_key_der(bytes)
            .map(|_| ())
            .map_err(|_| Error::KeyAgreement(KeyAgreementError::InvalidPeerPublicKey))
    }

    fn validate_private_key(bytes: &[u8]) -> Result<(), Error> {
        p256::SecretKey::from_pkcs8_der(bytes)
            .map(|_| ())
            .map_err(|_| Error::Key(KeyError::InvalidEncoding))
    }
}

// ------------------- Newtype Wrappers for ECDH Keys -------------------
// ------------------- ECDH 密钥的 Newtype 包装器 -------------------

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EcdhPublicKey<P: EcdhParams> {
    bytes: Vec<u8>,
    _params: PhantomData<P>,
}

impl<P: EcdhParams> PartialEq for EcdhPublicKey<P> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<P: EcdhParams> Eq for EcdhPublicKey<P> {}

impl<P: EcdhParams> Clone for EcdhPublicKey<P> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            _params: PhantomData,
        }
    }
}

impl<'a, P: EcdhParams> From<&'a EcdhPublicKey<P>> for EcdhPublicKey<P> {
    fn from(key: &'a EcdhPublicKey<P>) -> Self {
        key.clone()
    }
}

impl<P: EcdhParams> TryFrom<&[u8]> for EcdhPublicKey<P> {
    type Error = Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Key::from_bytes(bytes)
    }
}

impl<P: EcdhParams> Key for EcdhPublicKey<P> {
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        P::validate_public_key(bytes)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            _params: PhantomData,
        })
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.bytes.clone())
    }
}

impl<P: EcdhParams> PublicKey for EcdhPublicKey<P> {}

#[derive(Debug, Zeroize, Clone, Eq, PartialEq)]
#[zeroize(drop)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EcdhPrivateKey<P: EcdhParams> {
    bytes: Zeroizing<Vec<u8>>,
    _params: PhantomData<P>,
}

impl<P: EcdhParams> Key for EcdhPrivateKey<P> {
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        P::validate_private_key(bytes)?;
        Ok(Self {
            bytes: Zeroizing::new(bytes.to_vec()),
            _params: PhantomData,
        })
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.bytes.to_vec())
    }
}

impl<P: EcdhParams> TryFrom<&[u8]> for EcdhPrivateKey<P> {
    type Error = Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Key::from_bytes(bytes)
    }
}

impl<P: EcdhParams + Clone> PrivateKey<EcdhPublicKey<P>> for EcdhPrivateKey<P> {}

// ------------------- Generic ECDH Scheme Implementation -------------------
// ------------------- 通用 ECDH 方案实现 -------------------

/// A generic struct representing the ECDH scheme for a given parameter set.
///
/// 一个通用结构体，表示给定参数集的 ECDH 方案。
#[derive(Clone, Debug, Default)]
pub struct EcdhScheme<P: EcdhParams> {
    _params: PhantomData<P>,
}

impl<P: EcdhParams + Clone> AsymmetricKeySet for EcdhScheme<P> {
    type PublicKey = EcdhPublicKey<P>;
    type PrivateKey = EcdhPrivateKey<P>;
}

impl<P: EcdhParams + Clone> Algorithm for EcdhScheme<P> {
    fn name() -> String {
        P::NAME.to_string()
    }
    const ID: u32 = P::ID;
}

impl KeyGenerator for EcdhScheme<EcdhP256Params> {
    fn generate_keypair() -> Result<(Self::PublicKey, Self::PrivateKey), Error> {
        let secret = SecretKey::random(&mut OsRng);
        let public_key = secret.public_key();

        let private_key_der = secret
            .to_pkcs8_der()
            .map_err(|_| Error::Key(KeyError::GenerationFailed))?;

        let public_key_der = public_key
            .to_public_key_der()
            .map_err(|_| Error::Key(KeyError::GenerationFailed))?;

        Ok((
            EcdhPublicKey {
                bytes: public_key_der.as_bytes().to_vec(),
                _params: PhantomData,
            },
            EcdhPrivateKey {
                bytes: Zeroizing::new(private_key_der.as_bytes().to_vec()),
                _params: PhantomData,
            },
        ))
    }
}

impl KeyAgreement for EcdhScheme<EcdhP256Params> {
    fn agree(
        private_key: &Self::PrivateKey,
        public_key: &Self::PublicKey,
    ) -> Result<SharedSecret, Error> {
        let pk = P256PublicKey::from_public_key_der(&public_key.bytes)
            .map_err(|_| Error::KeyAgreement(KeyAgreementError::InvalidPeerPublicKey))?;

        let sk = SecretKey::from_pkcs8_der(&private_key.bytes)
            .map_err(|_| Error::Key(KeyError::InvalidEncoding))?;
        let shared_secret = ecdh::diffie_hellman(sk.to_nonzero_scalar(), pk.as_affine());

        Ok(Zeroizing::new(shared_secret.raw_secret_bytes().to_vec()))
    }
}

// ------------------- Type Aliases for Specific ECDH Schemes -------------------
// ------------------- 特定 ECDH 方案的类型别名 -------------------

/// A type alias for the ECDH P-256 scheme.
///
/// ECDH P-256 方案的类型别名。
pub type EcdhP256 = EcdhScheme<EcdhP256Params>;

// ------------------- Tests -------------------
// ------------------- 测试 -------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecdh_p256_key_agreement() {
        // Alice generates a keypair
        let (alice_pk, alice_sk) = EcdhP256::generate_keypair().unwrap();

        // Bob generates a keypair
        let (bob_pk, bob_sk) = EcdhP256::generate_keypair().unwrap();

        // They perform key agreement
        let alice_shared = EcdhP256::agree(&alice_sk, &bob_pk).unwrap();
        let bob_shared = EcdhP256::agree(&bob_sk, &alice_pk).unwrap();

        // The shared secrets must be equal
        assert_eq!(alice_shared, bob_shared);

        // Test key serialization/deserialization
        let alice_pk_bytes = alice_pk.to_bytes().unwrap();
        let alice_sk_bytes = alice_sk.to_bytes().unwrap();

        let _ = EcdhPublicKey::<EcdhP256Params>::from_bytes(&alice_pk_bytes).unwrap();
        let alice_sk2 = EcdhPrivateKey::<EcdhP256Params>::from_bytes(&alice_sk_bytes).unwrap();

        let alice_shared2 = EcdhP256::agree(&alice_sk2, &bob_pk).unwrap();
        assert_eq!(alice_shared, alice_shared2);
    }
}
