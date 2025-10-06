// Use Token and Doc
mod accents;
mod duplicated_word;
mod final_n;
mod forbidden_accent;
mod missing_accent_capital;
mod missing_double_accents;
mod mixed_scripts;
mod punctuation;

// Use raw text
mod ambiguous_char;
mod forbidden_char;
mod outdated_spelling;

pub use accents::{monosyllable_accented, multisyllable_not_accented};
pub use duplicated_word::duplicated_word;
pub use final_n::{add_final_n, remove_final_n};
pub use forbidden_accent::{forbidden_accent, forbidden_double_accent};
pub use missing_accent_capital::missing_accent_capital;
pub use missing_double_accents::missing_double_accents;
pub use mixed_scripts::mixed_scripts;
pub use punctuation::punctuation;

pub use ambiguous_char::ambiguous_char;
pub use forbidden_char::forbidden_char;
pub use outdated_spelling::outdated_spelling;
