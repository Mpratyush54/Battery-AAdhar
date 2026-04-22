//! api/ — gRPC server implementation
//!
//! Each module corresponds to one .proto service definition.
//! Handlers are empty stubs that return Unimplemented for now.

pub mod crypto;
pub mod battery;
pub mod auth;
pub mod lifecycle;

// Re-export for convenient access in main.rs
pub use crypto::CryptoServiceImpl;
pub use battery::BatteryServiceImpl;
pub use auth::AuthServiceImpl;
pub use lifecycle::LifecycleServiceImpl;
pub use crypto::CryptoServiceServer;
pub use battery::BatteryServiceServer;
pub use auth::AuthServiceServer;
pub use lifecycle::LifecycleServiceServer;
