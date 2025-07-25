//! Provides an implementation of symmetric authenticated encryption (AEAD) using ChaCha20-Poly1305.
//!
//! This module implements the ChaCha20-Poly1305 authenticated encryption with associated data
//! (AEAD) scheme. ChaCha20-Poly1305 combines the ChaCha20 stream cipher with the Poly1305
//! message authentication code to provide both confidentiality and authenticity.
//!
//! # Algorithm Components
//! - **ChaCha20**: A stream cipher designed by Daniel J. Bernstein
//! - **Poly1305**: A message authentication code also designed by Bernstein
//!
//! # Variants
//! - **ChaCha20-Poly1305**: Standard variant with 96-bit nonces
//! - **XChaCha20-Poly1305**: Extended variant with 192-bit nonces for better nonce misuse resistance
//!
//! # Security Features
//! - Authenticated encryption: provides both confidentiality and authenticity
//! - Constant-time implementation resistant to timing attacks
//! - No known cryptanalytic attacks against the full algorithm
//! - Designed to be secure even with nonce reuse (though not recommended)
//!
//! # Performance Characteristics
//! - Excellent software performance, especially on platforms without AES acceleration
//! - Constant-time implementation prevents side-channel attacks
//! - Suitable for high-performance applications and embedded systems
//! - No patent restrictions
//!
//! # Use Cases
//! - Applications requiring high-performance software encryption
//! - Systems without hardware AES acceleration
//! - Protocols requiring constant-time cryptographic operations
//! - Embedded systems with limited computational resources
//!
//! 提供了使用 ChaCha20-Poly1305 的对称认证加密（AEAD）实现。
//!
//! 此模块实现了 ChaCha20-Poly1305 带关联数据的认证加密 (AEAD) 方案。
//! ChaCha20-Poly1305 结合了 ChaCha20 流密码和 Poly1305 消息认证码，
//! 同时提供机密性和真实性。
//!
//! # 算法组件
//! - **ChaCha20**: 由 Daniel J. Bernstein 设计的流密码
//! - **Poly1305**: 同样由 Bernstein 设计的消息认证码
//!
//! # 变体
//! - **ChaCha20-Poly1305**: 具有 96 位 nonce 的标准变体
//! - **XChaCha20-Poly1305**: 具有 192 位 nonce 的扩展变体，具有更好的 nonce 误用抵抗性
//!
//! # 安全特性
//! - 认证加密：同时提供机密性和真实性
//! - 恒定时间实现，抵抗时序攻击
//! - 对完整算法没有已知的密码分析攻击
//! - 设计为即使在 nonce 重用时也是安全的（尽管不推荐）
//!
//! # 性能特征
//! - 出色的软件性能，特别是在没有 AES 加速的平台上
//! - 恒定时间实现防止侧信道攻击
//! - 适用于高性能应用程序和嵌入式系统
//! - 无专利限制
//!
//! # 使用场景
//! - 需要高性能软件加密的应用程序
//! - 没有硬件 AES 加速的系统
//! - 需要恒定时间加密操作的协议
//! - 计算资源有限的嵌入式系统

use crate::errors::Error;
use crate::prelude::*;
use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, AeadInPlace, Key, KeyInit, OsRng};
use chacha20poly1305::{
    ChaCha20Poly1305 as ChaCha20Poly1305Core, XChaCha20Poly1305 as XChaCha20Poly1305Core,
};
use std::marker::PhantomData;

// ------------------- Marker Structs and Trait for ChaCha20-Poly1305 Parameters -------------------
// ------------------- 用于 ChaCha20-Poly1305 参数的标记结构体和 Trait -------------------

mod private {
    pub trait Sealed {}
}

/// A sealed trait that defines the parameters for a ChaCha20-Poly1305 scheme.
///
/// 一个密封的 trait，用于定义 ChaCha20-Poly1305 方案的参数。
pub trait Chacha20Poly1305Params: private::Sealed + SchemeParams {
    /// The underlying `chacha20poly1305` AEAD cipher type.
    ///
    /// 底层的 `chacha20poly1305` AEAD 密码类型。
    type AeadCipher: Aead + AeadInPlace + KeyInit;
    /// The size of the key in bytes.
    ///
    /// 密钥的大小（以字节为单位）。
    const KEY_SIZE: usize;
    /// The size of the nonce in bytes.
    ///
    /// Nonce 的大小（以字节为单位）。
    const NONCE_SIZE: usize;
    /// The size of the authentication tag in bytes.
    ///
    /// 认证标签的大小（以字节为单位）。
    const TAG_SIZE: usize;
}

/// Marker struct for ChaCha20-Poly1305.
///
/// ChaCha20-Poly1305 的标记结构体。
#[derive(Clone, Debug, Default)]
pub struct ChaCha20Poly1305Params;
impl private::Sealed for ChaCha20Poly1305Params {}
impl SchemeParams for ChaCha20Poly1305Params {
    const NAME: &'static str = "ChaCha20-Poly1305";
    const ID: u32 = 0x02_02_01_01;
}
impl Chacha20Poly1305Params for ChaCha20Poly1305Params {
    type AeadCipher = ChaCha20Poly1305Core;
    const KEY_SIZE: usize = 32;
    const NONCE_SIZE: usize = 12;
    const TAG_SIZE: usize = 16;
}

/// Marker struct for XChaCha20-Poly1305.
///
/// XChaCha20-Poly1305 的标记结构体。
#[derive(Clone, Debug, Default)]
pub struct XChaCha20Poly1305Params;
impl private::Sealed for XChaCha20Poly1305Params {}
impl SchemeParams for XChaCha20Poly1305Params {
    const NAME: &'static str = "XChaCha20-Poly1305";
    const ID: u32 = 0x02_02_02_01;
}

impl Chacha20Poly1305Params for XChaCha20Poly1305Params {
    type AeadCipher = XChaCha20Poly1305Core;
    const KEY_SIZE: usize = 32;
    const NONCE_SIZE: usize = 24;
    const TAG_SIZE: usize = 16;
}

// ------------------- Generic ChaCha20-Poly1305 Implementation -------------------
// ------------------- 通用 ChaCha20-Poly1305 实现 -------------------

/// A generic struct representing the ChaCha20-Poly1305 cryptographic system.
///
/// 一个通用结构体，表示 ChaCha20-Poly1305 密码系统。
#[derive(Clone, Debug, Default)]
pub struct Chacha20Poly1305Scheme<P: Chacha20Poly1305Params> {
    _params: PhantomData<P>,
}

impl<P: Chacha20Poly1305Params> Algorithm for Chacha20Poly1305Scheme<P> {
    fn name() -> String {
        P::NAME.to_string()
    }
    const ID: u32 = P::ID;
}

impl<P: Chacha20Poly1305Params> SymmetricKeySet for Chacha20Poly1305Scheme<P> {
    type Key = SymmetricKey;
}

impl<P: Chacha20Poly1305Params> AeadCipher for Chacha20Poly1305Scheme<P> {
    const KEY_SIZE: usize = P::KEY_SIZE;
    const NONCE_SIZE: usize = P::NONCE_SIZE;
    const TAG_SIZE: usize = P::TAG_SIZE;
}

impl<P: Chacha20Poly1305Params> SymmetricKeyGenerator for Chacha20Poly1305Scheme<P> {
    const KEY_SIZE: usize = P::KEY_SIZE;

    fn generate_key() -> Result<SymmetricKey, Error> {
        let mut key_bytes = vec![0u8; P::KEY_SIZE];
        OsRng
            .try_fill_bytes(&mut key_bytes)
            .map_err(|_| Error::Key(KeyError::GenerationFailed))?;
        Ok(SymmetricKey::new(key_bytes))
    }
}

impl<P: Chacha20Poly1305Params> AeadEncryptor for Chacha20Poly1305Scheme<P> {
    fn encrypt_to_buffer(
        key: &Self::Key,
        nonce: &[u8],
        plaintext: &[u8],
        output: &mut [u8],
        aad: Option<AssociatedData>,
    ) -> Result<usize, Error> {
        if key.len() != P::KEY_SIZE {
            return Err(Error::Symmetric(SymmetricError::InvalidKeySize));
        }
        if nonce.len() != P::NONCE_SIZE {
            return Err(Error::Symmetric(SymmetricError::InvalidNonceSize));
        }

        let required_len = plaintext.len() + P::TAG_SIZE;
        if output.len() < required_len {
            return Err(Error::Symmetric(SymmetricError::OutputTooSmall));
        }

        let key = Key::<P::AeadCipher>::from_slice(key);
        let cipher = P::AeadCipher::new(key);
        let nonce_core = chacha20poly1305::aead::Nonce::<P::AeadCipher>::from_slice(nonce);

        let (ciphertext_buf, tag_buf) = output.split_at_mut(plaintext.len());
        ciphertext_buf.copy_from_slice(plaintext);

        let tag = cipher
            .encrypt_in_place_detached(nonce_core, aad.unwrap_or_default(), ciphertext_buf)
            .map_err(|_| Error::Symmetric(SymmetricError::Encryption))?;

        tag_buf[..P::TAG_SIZE].copy_from_slice(&tag);

        Ok(required_len)
    }
}

impl<P: Chacha20Poly1305Params> AeadDecryptor for Chacha20Poly1305Scheme<P> {
    fn decrypt_to_buffer(
        key: &Self::Key,
        nonce: &[u8],
        ciphertext_with_tag: &[u8],
        output: &mut [u8],
        aad: Option<AssociatedData>,
    ) -> Result<usize, Error> {
        if key.len() != P::KEY_SIZE {
            return Err(Error::Symmetric(SymmetricError::InvalidKeySize));
        }
        if nonce.len() != P::NONCE_SIZE {
            return Err(Error::Symmetric(SymmetricError::InvalidNonceSize));
        }
        if ciphertext_with_tag.len() < P::TAG_SIZE {
            return Err(Error::Symmetric(SymmetricError::InvalidCiphertext));
        }

        let (ciphertext, tag) =
            ciphertext_with_tag.split_at(ciphertext_with_tag.len() - P::TAG_SIZE);

        if output.len() < ciphertext.len() {
            return Err(Error::Symmetric(SymmetricError::OutputTooSmall));
        }

        let key = Key::<P::AeadCipher>::from_slice(key);
        let cipher = P::AeadCipher::new(key);
        let nonce_core = chacha20poly1305::aead::Nonce::<P::AeadCipher>::from_slice(nonce);
        let tag = chacha20poly1305::aead::Tag::<P::AeadCipher>::from_slice(tag);

        let plaintext_buf = &mut output[..ciphertext.len()];
        plaintext_buf.copy_from_slice(ciphertext);

        cipher
            .decrypt_in_place_detached(nonce_core, aad.unwrap_or_default(), plaintext_buf, tag)
            .map_err(|_| Error::Symmetric(SymmetricError::Decryption))?;

        Ok(plaintext_buf.len())
    }
}

// ------------------- Type Aliases -------------------
// ------------------- 类型别名 -------------------

/// A type alias for the ChaCha20-Poly1305 scheme.
///
/// ChaCha20-Poly1305 方案的类型别名。
pub type ChaCha20Poly1305 = Chacha20Poly1305Scheme<ChaCha20Poly1305Params>;

/// A type alias for the XChaCha20-Poly1305 scheme.
///
/// XChaCha20-Poly1305 方案的类型别名。
pub type XChaCha20Poly1305 = Chacha20Poly1305Scheme<XChaCha20Poly1305Params>;

/// A type alias for the authentication tag used in ChaCha20-Poly1305.
///
/// ChaCha20-Poly1305 中使用的认证标签的类型别名。
pub type Tag<'a> = &'a [u8];

// ------------------- Tests -------------------
// ------------------- 测试 -------------------

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    fn test_roundtrip<S>()
    where
        S: AeadEncryptor<Key = SymmetricKey>
            + AeadDecryptor<Key = SymmetricKey>
            + SymmetricKeyGenerator<Key = SymmetricKey>,
    {
        let key = S::generate_key().unwrap();
        let plaintext = b"this is a secret message".to_vec();
        let aad = b"this is authenticated data".to_vec();
        let empty_vec = Vec::new();
        let mut nonce = vec![0u8; S::NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce);

        // With AAD
        // 使用 AAD
        let ciphertext_aad = S::encrypt(&key, &nonce, &plaintext, Some(&aad)).unwrap();
        let decrypted_aad = S::decrypt(&key, &nonce, &ciphertext_aad, Some(&aad)).unwrap();
        assert_eq!(plaintext, decrypted_aad);

        // Test buffer encryption with AAD
        let mut encrypted_buffer_aad = vec![0u8; plaintext.len() + S::TAG_SIZE];
        let bytes_written = S::encrypt_to_buffer(
            &key,
            &nonce,
            &plaintext,
            &mut encrypted_buffer_aad,
            Some(&aad),
        )
        .unwrap();
        assert_eq!(bytes_written, ciphertext_aad.len());
        assert_eq!(ciphertext_aad, &encrypted_buffer_aad[..bytes_written]);

        let mut decrypted_buffer_aad = vec![0u8; plaintext.len()];
        let bytes_written = S::decrypt_to_buffer(
            &key,
            &nonce,
            &encrypted_buffer_aad,
            &mut decrypted_buffer_aad,
            Some(&aad),
        )
        .unwrap();
        assert_eq!(bytes_written, plaintext.len());
        assert_eq!(plaintext, &decrypted_buffer_aad[..bytes_written]);

        // Without AAD
        // 不使用 AAD
        let ciphertext_no_aad = S::encrypt(&key, &nonce, &plaintext, None).unwrap();
        let decrypted_no_aad = S::decrypt(&key, &nonce, &ciphertext_no_aad, None).unwrap();
        assert_eq!(plaintext, decrypted_no_aad);

        // Test buffer encryption without AAD
        let mut encrypted_buffer_no_aad = vec![0u8; plaintext.len() + S::TAG_SIZE];
        let bytes_written =
            S::encrypt_to_buffer(&key, &nonce, &plaintext, &mut encrypted_buffer_no_aad, None)
                .unwrap();
        assert_eq!(bytes_written, ciphertext_no_aad.len());
        assert_eq!(ciphertext_no_aad, &encrypted_buffer_no_aad[..bytes_written]);

        let mut decrypted_buffer_no_aad = vec![0u8; plaintext.len()];
        let bytes_written = S::decrypt_to_buffer(
            &key,
            &nonce,
            &encrypted_buffer_no_aad,
            &mut decrypted_buffer_no_aad,
            None,
        )
        .unwrap();
        assert_eq!(bytes_written, plaintext.len());
        assert_eq!(plaintext, &decrypted_buffer_no_aad[..bytes_written]);

        // Empty Plaintext with AAD
        // 空明文和 AAD
        let ciphertext_empty_pt = S::encrypt(&key, &nonce, &empty_vec, Some(&aad)).unwrap();
        let decrypted_empty_pt =
            S::decrypt(&key, &nonce, &ciphertext_empty_pt, Some(&aad)).unwrap();
        assert_eq!(empty_vec, decrypted_empty_pt);

        // Plaintext with Empty AAD
        // 明文和空 AAD
        let ciphertext_empty_aad = S::encrypt(&key, &nonce, &plaintext, Some(&[])).unwrap();
        let decrypted_empty_aad =
            S::decrypt(&key, &nonce, &ciphertext_empty_aad, Some(&[])).unwrap();
        assert_eq!(plaintext, decrypted_empty_aad);

        // Tampered Ciphertext
        // 篡改密文
        let mut tampered_ciphertext = ciphertext_aad.clone();
        tampered_ciphertext[0] ^= 0xff;
        assert!(S::decrypt(&key, &nonce, &tampered_ciphertext, Some(&aad)).is_err());

        // Tampered AAD
        // 篡改 AAD
        let tampered_aad = b"this is different authenticated data".to_vec();
        assert!(S::decrypt(&key, &nonce, &ciphertext_aad, Some(&tampered_aad)).is_err());
    }

    #[test]
    fn test_chacha20_poly1305_scheme() {
        test_roundtrip::<Chacha20Poly1305Scheme<ChaCha20Poly1305Params>>();
    }

    fn test_invalid_inputs<S>()
    where
        S: AeadEncryptor<Key = SymmetricKey>
            + AeadDecryptor<Key = SymmetricKey>
            + SymmetricKeyGenerator<Key = SymmetricKey>,
    {
        let key = S::generate_key().unwrap();
        let mut wrong_size_key = key.to_vec();
        wrong_size_key.push(0);
        let wrong_size_key = Zeroizing::new(wrong_size_key);

        let mut nonce = vec![0u8; S::NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce);
        let mut wrong_size_nonce = nonce.clone();
        wrong_size_nonce.push(0);

        let plaintext = b"plaintext";
        let ciphertext = S::encrypt(&key, &nonce, plaintext, None).unwrap();

        // Invalid key size
        // 无效密钥大小
        let err = S::encrypt(&wrong_size_key, &nonce, plaintext, None).unwrap_err();
        assert!(matches!(
            err,
            Error::Symmetric(SymmetricError::InvalidKeySize)
        ));
        let err = S::decrypt(&wrong_size_key, &nonce, &ciphertext, None).unwrap_err();
        assert!(matches!(
            err,
            Error::Symmetric(SymmetricError::InvalidKeySize)
        ));

        // Invalid nonce size
        // 无效 Nonce 大小
        let err = S::encrypt(&key, &wrong_size_nonce, plaintext, None).unwrap_err();
        assert!(matches!(
            err,
            Error::Symmetric(SymmetricError::InvalidNonceSize)
        ));
        let err = S::decrypt(&key, &wrong_size_nonce, &ciphertext, None).unwrap_err();
        assert!(matches!(
            err,
            Error::Symmetric(SymmetricError::InvalidNonceSize)
        ));
    }

    #[test]
    fn test_chacha20_poly1305_invalid_inputs() {
        test_invalid_inputs::<Chacha20Poly1305Scheme<ChaCha20Poly1305Params>>();
    }

    #[test]
    fn test_xchacha20_poly1305_invalid_inputs() {
        test_invalid_inputs::<Chacha20Poly1305Scheme<XChaCha20Poly1305Params>>();
    }
}
