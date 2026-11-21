//! Bounded, versioned binary envelope for signed service advertisements.
use crate::{OverlayEndpoint, OverlayId, SignedServiceAdvertisement, SignedAdvertisementError, ServiceId};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
const MAGIC:&[u8;4]=b"MPAD";const VERSION:u8=1;const MAX_SIZE:usize=16384;const MAX_ENDPOINTS:usize=64;const MAX_NAME:usize=128;
#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub enum AdvertisementWireError{TooLarge,Truncated,InvalidMagic,UnsupportedVersion,InvalidEndpointCount,InvalidOverlay,InvalidName,InvalidAddress,InvalidPortFlag,TrailingBytes,InvalidRecord(SignedAdvertisementError)}
impl std::fmt::Display for AdvertisementWireError{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{write!(f,"wire error: {self:?}")}}
impl std::error::Error for AdvertisementWireError{}
impl SignedServiceAdvertisement{
 /// Encode bounded v1 envelope. Signature payload is independent of this format.
 pub fn encode(&self)->Result<Vec<u8>,AdvertisementWireError>{
  validate(&self.endpoints).map_err(AdvertisementWireError::InvalidRecord)?;
  let mut b=Vec::with_capacity(128);b.extend(MAGIC);b.push(VERSION);b.extend(self.public_key);b.extend(self.service_id.as_bytes());b.extend(self.issued_at.to_be_bytes());b.extend(self.expires_at.to_be_bytes());b.extend((self.endpoints.len() as u16).to_be_bytes());
  for e in &self.endpoints{match &e.network{OverlayId::Cjdns=>b.push(0),OverlayId::Yggdrasil=>b.push(1),OverlayId::Other(n)=>{if n.is_empty()||n.len()>MAX_NAME{return Err(AdvertisementWireError::InvalidName)}b.push(2);b.push(n.len() as u8);b.extend(n.as_bytes())}}
   match e.address{IpAddr::V4(a)=>{b.push(4);b.extend(a.octets())},IpAddr::V6(a)=>{b.push(6);b.extend(a.octets())}}
   match e.port{Some(p)=>{b.push(1);b.extend(p.to_be_bytes())},None=>b.push(0)}}
  b.extend(self.signature);if b.len()>MAX_SIZE{return Err(AdvertisementWireError::TooLarge)}Ok(b)
 }
 /// Decode structurally only; verify signature and time before trusting.
 pub fn decode(input:&[u8])->Result<Self,AdvertisementWireError>{
  if input.len()>MAX_SIZE{return Err(AdvertisementWireError::TooLarge)}let mut r=Reader{b:input,p:0};if r.take(4)?!=MAGIC{return Err(AdvertisementWireError::InvalidMagic)}if r.u8()?!=VERSION{return Err(AdvertisementWireError::UnsupportedVersion)}
  let key:[u8;32]=r.take(32)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?;let id:[u8;32]=r.take(32)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?;
  let service_id=ServiceId::from_public_key(&key).map_err(|e|AdvertisementWireError::InvalidRecord(SignedAdvertisementError::Identity(e)))?;if service_id.as_bytes()!=&id{return Err(AdvertisementWireError::InvalidRecord(SignedAdvertisementError::InvalidSignature))}
  let issued_at=r.u64()?;let expires_at=r.u64()?;if expires_at<=issued_at{return Err(AdvertisementWireError::InvalidRecord(SignedAdvertisementError::InvalidValidityWindow))}
  let n=r.u16()? as usize;if n==0||n>MAX_ENDPOINTS{return Err(AdvertisementWireError::InvalidEndpointCount)}let mut endpoints=Vec::with_capacity(n);
  for _ in 0..n{let network=match r.u8()?{0=>OverlayId::Cjdns,1=>OverlayId::Yggdrasil,2=>{let len=r.u8()? as usize;if len==0||len>MAX_NAME{return Err(AdvertisementWireError::InvalidName)}OverlayId::Other(std::str::from_utf8(r.take(len)?).map_err(|_|AdvertisementWireError::InvalidName)?.to_owned())},_=>return Err(AdvertisementWireError::InvalidOverlay)};
   let address=match r.u8()?{4=>{let octets:[u8;4]=r.take(4)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?;IpAddr::V4(Ipv4Addr::from(octets))},6=>{let octets:[u8;16]=r.take(16)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?;IpAddr::V6(Ipv6Addr::from(octets))},_=>return Err(AdvertisementWireError::InvalidAddress)};
   let port=match r.u8()?{0=>None,1=>Some(r.u16()?),_=>return Err(AdvertisementWireError::InvalidPortFlag)};endpoints.push(OverlayEndpoint{network,address,port})}
  let signature:[u8;64]=r.take(64)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?;if !r.done(){return Err(AdvertisementWireError::TrailingBytes)}validate(&endpoints).map_err(AdvertisementWireError::InvalidRecord)?;
  Ok(Self{service_id,public_key:key,issued_at,expires_at,endpoints,signature})
 }
}
struct Reader<'a>{b:&'a[u8],p:usize}
impl<'a> Reader<'a>{fn take(&mut self,n:usize)->Result<&'a[u8],AdvertisementWireError>{let end=self.p.checked_add(n).ok_or(AdvertisementWireError::Truncated)?;let v=self.b.get(self.p..end).ok_or(AdvertisementWireError::Truncated)?;self.p=end;Ok(v)}fn u8(&mut self)->Result<u8,AdvertisementWireError>{Ok(self.take(1)?[0])}fn u16(&mut self)->Result<u16,AdvertisementWireError>{Ok(u16::from_be_bytes(self.take(2)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?))}fn u64(&mut self)->Result<u64,AdvertisementWireError>{Ok(u64::from_be_bytes(self.take(8)?.try_into().map_err(|_|AdvertisementWireError::Truncated)?))}fn done(&self)->bool{self.p==self.b.len()}}
fn validate(e:&[OverlayEndpoint])->Result<(),SignedAdvertisementError>{if e.is_empty(){return Err(SignedAdvertisementError::NoEndpoints)}if e.len()>MAX_ENDPOINTS{return Err(SignedAdvertisementError::UnsupportedEndpoint)}for(i,x)in e.iter().enumerate(){if let OverlayId::Other(n)=&x.network{if n.is_empty()||n.len()>MAX_NAME{return Err(SignedAdvertisementError::UnsupportedEndpoint)}}if e[..i].contains(x){return Err(SignedAdvertisementError::DuplicateEndpoint)}}Ok(())}
#[cfg(test)]mod tests{use super::*;use ed25519_dalek::SigningKey;use std::net::Ipv6Addr;fn ep()->OverlayEndpoint{OverlayEndpoint{network:OverlayId::Other("mesh".into()),address:IpAddr::V6(Ipv6Addr::LOCALHOST),port:Some(443)}}
#[test]fn encode_decode_verify(){let k=SigningKey::from_bytes(&[22;32]);let a=SignedServiceAdvertisement::sign(&k,vec![ep()],100,200).unwrap();let d=SignedServiceAdvertisement::decode(&a.encode().unwrap()).unwrap();assert_eq!(a,d);assert_eq!(d.verify_at(150,0),Ok(()))}
#[test]fn truncated_and_trailing_rejected(){let k=SigningKey::from_bytes(&[23;32]);let a=SignedServiceAdvertisement::sign(&k,vec![ep()],100,200).unwrap();let mut b=a.encode().unwrap();b.pop();assert_eq!(SignedServiceAdvertisement::decode(&b),Err(AdvertisementWireError::Truncated));let mut b=a.encode().unwrap();b.push(0);assert_eq!(SignedServiceAdvertisement::decode(&b),Err(AdvertisementWireError::TrailingBytes))}
#[test]fn bad_magic_and_oversize_rejected(){assert_eq!(SignedServiceAdvertisement::decode(b"bad!"),Err(AdvertisementWireError::InvalidMagic));assert_eq!(SignedServiceAdvertisement::decode(&vec![0;MAX_SIZE+1]),Err(AdvertisementWireError::TooLarge))}}