// Shared files
pub mod sort;

// Dev files
#[cfg(feature = "dev")]
pub mod game_data_dev;
#[cfg(feature = "dev")]
pub mod mod_dev;
/// Dev file is exported as main manager
#[cfg(feature = "dev")]
pub use mod_dev::*; 

// Public files
#[cfg(not(feature = "dev"))]
pub mod game_data_pub;
#[cfg(not(feature = "dev"))]
pub mod mod_pub;
/// Public file is exported as main manager
#[cfg(not(feature = "dev"))]
pub use mod_pub::*;