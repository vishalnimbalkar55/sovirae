//! SpeakIt product core: text rules, the speech document index, and the
//! session model shared by every entry path. No UI or audio code lives here.

pub mod document;
pub mod pronounce;
pub mod session;
pub mod text;

pub use document::{Segment, SpeechDocument};
pub use pronounce::Rule as PronunciationRule;
pub use session::{PlaybackSnapshot, PlaybackStatus, SourceKind, SourceReference, SpeakRequest};
pub use text::{estimate_minutes, validate, word_count, TextError, MAX_TEXT_UTF16};
