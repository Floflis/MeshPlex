//! Ed25519 signed service advertisement records.
use crate::{OverlayEndpoint, OverlayId, ServiceId, ServiceIdentityError};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::{collections::BTreeSet, net::IpAddr};
const DOMAIN: &[u8] = b"meshplex/signed-advertisement/v1\0";
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServiceAdvertisement {
    pub service_id: ServiceId,
    pub public_key: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub endpoints: Vec<OverlayEndpoint>,
    pub signature: [u8; 64],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignedAdvertisementError {
    Identity(ServiceIdentityError),
    NoEndpoints,
    DuplicateEndpoint,
    InvalidValidityWindow,
    InvalidPublicKey,
    InvalidSignature,
    NotYetValid,
    Expired,
    ClockSkew,
    UnsupportedEndpoint,
}
impl std::fmt::Display for SignedAdvertisementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(e) => std::fmt::Display::fmt(e, f),
            Self::NoEndpoints => f.write_str("no endpoints"),
            Self::DuplicateEndpoint => f.write_str("duplicate endpoint"),
            Self::InvalidValidityWindow => f.write_str("invalid validity interval"),
            Self::InvalidPublicKey => f.write_str("invalid public key"),
            Self::InvalidSignature => f.write_str("signature verification failed"),
            Self::NotYetValid => f.write_str("not yet valid"),
            Self::Expired => f.write_str("expired"),
            Self::ClockSkew => f.write_str("issue time exceeds allowed clock skew"),
            Self::UnsupportedEndpoint => f.write_str("unsupported endpoint encoding"),
        }
    }
}
impl std::error::Error for SignedAdvertisementError {}
impl From<ServiceIdentityError> for SignedAdvertisementError {
    fn from(e: ServiceIdentityError) -> Self {
        Self::Identity(e)
    }
}
impl SignedServiceAdvertisement {
    /// Sign a record with an Ed25519 key. Timestamps are Unix seconds.
    pub fn sign(
        key: &SigningKey,
        endpoints: Vec<OverlayEndpoint>,
        issued_at: u64,
        expires_at: u64,
    ) -> Result<Self, SignedAdvertisementError> {
        validate(&endpoints)?;
        if expires_at <= issued_at {
            return Err(SignedAdvertisementError::InvalidValidityWindow);
        }
        let public_key = key.verifying_key().to_bytes();
        let service_id = ServiceId::from_public_key(&public_key)?;
        let mut a = Self {
            service_id,
            public_key,
            issued_at,
            expires_at,
            endpoints,
            signature: [0; 64],
        };
        a.signature = key.sign(&a.payload()?).to_bytes();
        Ok(a)
    }
    /// Verify signature, key binding, and validity. now/skew are Unix seconds.
    pub fn verify_at(&self, now: u64, skew: u64) -> Result<(), SignedAdvertisementError> {
        validate(&self.endpoints)?;
        if self.expires_at <= self.issued_at {
            return Err(SignedAdvertisementError::InvalidValidityWindow);
        }
        // Accept a small future-issued timestamp only within the caller's skew allowance.
        if self.issued_at.saturating_sub(now) > skew {
            return Err(SignedAdvertisementError::ClockSkew);
        }
        if now >= self.expires_at {
            return Err(SignedAdvertisementError::Expired);
        }
        if ServiceId::from_public_key(&self.public_key)? != self.service_id {
            return Err(SignedAdvertisementError::InvalidSignature);
        }
        let key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|_| SignedAdvertisementError::InvalidPublicKey)?;
        key.verify(&self.payload()?, &Signature::from_bytes(&self.signature))
            .map_err(|_| SignedAdvertisementError::InvalidSignature)
    }
    /// Deterministic encoding; endpoint input order does not alter the signature.
    fn payload(&self) -> Result<Vec<u8>, SignedAdvertisementError> {
        let mut eps = self.endpoints.clone();
        eps.sort_by_key(sort_key);
        let mut b = Vec::new();
        b.extend_from_slice(DOMAIN);
        b.extend_from_slice(&self.public_key);
        b.extend_from_slice(self.service_id.as_bytes());
        b.extend_from_slice(&self.issued_at.to_be_bytes());
        b.extend_from_slice(&self.expires_at.to_be_bytes());
        let n =
            u32::try_from(eps.len()).map_err(|_| SignedAdvertisementError::UnsupportedEndpoint)?;
        b.extend_from_slice(&n.to_be_bytes());
        for e in eps {
            encode(&mut b, &e)?
        }
        Ok(b)
    }
}
fn validate(e: &[OverlayEndpoint]) -> Result<(), SignedAdvertisementError> {
    if e.is_empty() {
        return Err(SignedAdvertisementError::NoEndpoints);
    }
    let mut s = BTreeSet::new();
    for x in e {
        if !s.insert(x.clone()) {
            return Err(SignedAdvertisementError::DuplicateEndpoint);
        }
    }
    Ok(())
}
fn sort_key(e: &OverlayEndpoint) -> Vec<u8> {
    let mut k = Vec::new();
    match &e.network {
        OverlayId::Cjdns => k.push(0),
        OverlayId::Yggdrasil => k.push(1),
        OverlayId::Other(s) => {
            k.push(2);
            let len = u16::try_from(s.len()).unwrap_or(u16::MAX);
            k.extend(len.to_be_bytes());
            k.extend(s.as_bytes());
        }
    }
    match e.address {
        IpAddr::V4(a) => {
            k.push(4);
            k.extend(a.octets())
        }
        IpAddr::V6(a) => {
            k.push(6);
            k.extend(a.octets())
        }
    }
    if let Some(p) = e.port {
        k.push(1);
        k.extend(p.to_be_bytes())
    } else {
        k.push(0)
    }
    k
}
fn encode(b: &mut Vec<u8>, e: &OverlayEndpoint) -> Result<(), SignedAdvertisementError> {
    match &e.network {
        OverlayId::Cjdns => b.push(0),
        OverlayId::Yggdrasil => b.push(1),
        OverlayId::Other(s) => {
            b.push(2);
            let n = u16::try_from(s.len())
                .map_err(|_| SignedAdvertisementError::UnsupportedEndpoint)?;
            b.extend(n.to_be_bytes());
            b.extend(s.as_bytes())
        }
    }
    match e.address {
        IpAddr::V4(a) => {
            b.push(4);
            b.extend(a.octets())
        }
        IpAddr::V6(a) => {
            b.push(6);
            b.extend(a.octets())
        }
    }
    if let Some(p) = e.port {
        b.push(1);
        b.extend(p.to_be_bytes())
    } else {
        b.push(0)
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;
    fn ep() -> OverlayEndpoint {
        OverlayEndpoint {
            network: OverlayId::Yggdrasil,
            address: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: Some(443),
        }
    }
    #[test]
    fn signature_and_expiry() {
        let k = SigningKey::from_bytes(&[7; 32]);
        let a = SignedServiceAdvertisement::sign(&k, vec![ep()], 100, 200).unwrap();
        assert_eq!(a.verify_at(150, 5), Ok(()));
        assert_eq!(a.verify_at(200, 5), Err(SignedAdvertisementError::Expired));
        assert_eq!(a.verify_at(90, 5), Err(SignedAdvertisementError::ClockSkew));
        assert_eq!(a.verify_at(98, 5), Ok(()));
    }
    #[test]
    fn tampering_rejected() {
        let k = SigningKey::from_bytes(&[9; 32]);
        let mut a = SignedServiceAdvertisement::sign(&k, vec![ep()], 100, 200).unwrap();
        a.endpoints[0].port = Some(8443);
        assert_eq!(
            a.verify_at(150, 5),
            Err(SignedAdvertisementError::InvalidSignature)
        )
    }
    #[test]
    fn canonical_order() {
        let k = SigningKey::from_bytes(&[3; 32]);
        let x = ep();
        let y = OverlayEndpoint {
            network: OverlayId::Cjdns,
            address: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: Some(80),
        };
        let a = SignedServiceAdvertisement::sign(&k, vec![x.clone(), y.clone()], 1, 10).unwrap();
        let b = SignedServiceAdvertisement::sign(&k, vec![y, x], 1, 10).unwrap();
        assert_eq!(a.signature, b.signature)
    }
    #[test]
    fn other_network_names_are_canonicalized_with_length_prefix() {
        let key = SigningKey::from_bytes(&[5; 32]);
        let a = OverlayEndpoint {
            network: OverlayId::Other("mesh-a".into()),
            address: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: Some(10),
        };
        let b = OverlayEndpoint {
            network: OverlayId::Other("mesh-a-long".into()),
            address: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: Some(20),
        };
        let first =
            SignedServiceAdvertisement::sign(&key, vec![a.clone(), b.clone()], 1, 10).unwrap();
        let reversed = SignedServiceAdvertisement::sign(&key, vec![b, a], 1, 10).unwrap();
        assert_eq!(first.signature, reversed.signature);
    }

    #[test]
    fn rejects_empty_and_duplicate() {
        let k = SigningKey::from_bytes(&[1; 32]);
        assert_eq!(
            SignedServiceAdvertisement::sign(&k, vec![], 1, 2).unwrap_err(),
            SignedAdvertisementError::NoEndpoints
        );
        let x = ep();
        assert_eq!(
            SignedServiceAdvertisement::sign(&k, vec![x.clone(), x], 1, 2).unwrap_err(),
            SignedAdvertisementError::DuplicateEndpoint
        )
    }
}
