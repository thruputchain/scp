// clang-format off
%#include "scp.hh"

namespace hogd {

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

} // namespace hogd
