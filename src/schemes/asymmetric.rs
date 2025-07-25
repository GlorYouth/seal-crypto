//! Asymmetric cryptographic schemes.
//!
//! 非对称加密方案。

/// Traditional asymmetric cryptographic schemes.
///
/// 传统非对称加密方案。
pub mod traditional {
    /// RSA based schemes.
    ///
    /// 基于 RSA 的方案。
    pub mod rsa {
        #[cfg(feature = "rsa-default")]
        pub use crate::systems::asymmetric::traditional::rsa::*;
    }

    /// Elliptic Curve Cryptography based schemes.
    ///
    /// 基于椭圆曲线密码学的方案。
    pub mod ecc {
        #[cfg(feature = "ecc-default")]
        pub use crate::systems::asymmetric::traditional::ecc::*;
    }

    /// Elliptic Curve Diffie-Hellman based schemes.
    ///
    /// 基于椭圆曲线迪菲-赫尔曼的方案。
    pub mod ecdh {
        #[cfg(feature = "ecdh-default")]
        pub use crate::systems::asymmetric::traditional::ecdh::*;
    }
}

/// Post-quantum cryptography schemes
///
/// 后量子密码学方案
pub mod post_quantum {
    /// Kyber KEM, a post-quantum key encapsulation method.
    ///
    /// Kyber KEM，一种后量子密钥封装方法。
    #[cfg(feature = "kyber-default")]
    pub mod kyber {
        pub use crate::systems::asymmetric::post_quantum::kyber::*;
    }

    /// Dilithium, a post-quantum signature scheme.
    ///
    /// Dilithium，一种后量子签名方案。
    #[cfg(feature = "dilithium-default")]
    pub mod dilithium {
        pub use crate::systems::asymmetric::post_quantum::dilithium::*;
    }
}
