//! api/ — gRPC server implementation
//!
//! Each module corresponds to one .proto service definition.
//! Handlers are empty stubs that return Unimplemented for now.

pub mod auth;
pub mod battery;
pub mod crypto;
pub mod lifecycle;

// Re-export for convenient access in main.rs
pub use auth::AuthServiceImpl;
pub use auth::AuthServiceServer;
pub use battery::BatteryServiceImpl;
pub use battery::BatteryServiceServer;
pub use crypto::CryptoServiceImpl;
pub use crypto::CryptoServiceServer;
pub use lifecycle::LifecycleServiceImpl;
pub use lifecycle::LifecycleServiceServer;
