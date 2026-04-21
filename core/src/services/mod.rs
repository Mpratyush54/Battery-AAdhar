pub mod encryption;
pub use encryption::*;
pub mod hash_chain;
pub use hash_chain::*;
pub mod bpan_generator;
pub use bpan_generator::*;
pub mod qr_service;
pub use qr_service::*;
pub mod battery_lifecycle;
pub use battery_lifecycle::*;
pub mod validation;
pub use validation::*;
pub mod registration;
pub use registration::*;
pub mod key_manager; // HKDF key hierarchy  (stub from R2)
pub mod signing;
pub mod zk_proofs; // ZK range proofs      (trait only today) // Ed25519 signing       (trait only today)
