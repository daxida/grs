mod accents;
mod duplicated_word;
mod final_n;
mod missing_accent_capital;
mod missing_double_accents;
mod mixed_scripts;
mod outdated_spelling;

pub use accents::{monosyllable_accented, multisyllable_not_accented};
pub use duplicated_word::duplicated_word;
pub use final_n::{add_final_n, remove_final_n};
pub use missing_accent_capital::missing_accent_capital;
pub use missing_double_accents::missing_double_accents;
pub use mixed_scripts::mixed_scripts;
pub use outdated_spelling::outdated_spelling;
