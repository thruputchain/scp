// clang-format off
namespace hogd {

struct uint128_xdr {
  unsigned hyper highbits;
  unsigned hyper lowbits;
};

struct int128_xdr {
  hyper highbits;
  unsigned hyper lowbits;
};

// ed25519 sig len is 512 bits
typedef opaque Ed25519Signature[64]; // #[rust(newtype)]
// ed25519 key len is 256 bits
typedef opaque Ed25519PublicKey[32]; // #[rust(newtype)]
// ed25519 secret key len is 64 bytes
typedef opaque Ed25519SecretKey[64]; // #[rust(newtype)]

enum CryptographicScheme {
  Ed25519 = 0,
};

union Signature switch (CryptographicScheme scheme)
{
case Ed25519:
  Ed25519Signature ed25519;
};

union PublicKey switch (CryptographicScheme scheme)
{
case Ed25519:
  Ed25519PublicKey ed25519;
};

union SecretKey switch (CryptographicScheme scheme)
{
case Ed25519:
  Ed25519SecretKey ed25519;
};

// 256 bit hash
typedef opaque SHA256Hash[32]; // #[rust(newtype)]

} // namespace hogd
