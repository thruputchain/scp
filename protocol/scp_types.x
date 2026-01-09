// clang-format off
namespace scp {

struct SCP_uint128_xdr {
  unsigned hyper highbits;
  unsigned hyper lowbits;
};

struct SCP_int128_xdr {
  hyper highbits;
  unsigned hyper lowbits;
};

// ed25519 sig len is 512 bits
typedef opaque SCPEd25519Signature[64]; // #[rust(newtype)]
// ed25519 key len is 256 bits
typedef opaque SCPEd25519PublicKey[32]; // #[rust(newtype)]
// ed25519 secret key len is 64 bytes
typedef opaque SCPEd25519SecretKey[64]; // #[rust(newtype)]

enum SCPCryptographicScheme {
  Ed25519 = 0,
};

union SCPSignature switch (SCPCryptographicScheme scheme)
{
case Ed25519:
  SCPEd25519Signature ed25519;
};

union SCPPublicKey switch (SCPCryptographicScheme scheme)
{
case Ed25519:
  SCPEd25519PublicKey ed25519;
};

union SCPSecretKey switch (SCPCryptographicScheme scheme)
{
case Ed25519:
  SCPEd25519SecretKey ed25519;
};

// 256 bit hash
typedef opaque SCPSHA256Hash[32]; // #[rust(newtype)]

} // namespace scp
