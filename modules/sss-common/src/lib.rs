pub mod seeds;
pub mod validation;

pub use seeds::*;
pub use validation::*;

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 10;
pub const MAX_URI_LEN: usize = 200;
pub const MAX_REASON_LEN: usize = 128;
