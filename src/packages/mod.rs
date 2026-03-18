// Package detection and management

pub mod cargo;
pub mod detector;
pub mod npm;
pub mod types;

pub use detector::MultiPackageDetector;
