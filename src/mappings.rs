use std::{fs, iter, path::Path};

use miette::{Diagnostic, LabeledSpan};
use serde::Deserialize;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use crate::Error;

pub struct Mappings(Vec<Option<VIRTUAL_KEY>>);

// http://www.music.mcgill.ca/~ich/classes/mumt306/StandardMIDIfileformat.html#BMA1_3
// https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
impl Mappings {
    const LEN: usize = 128;

    fn empty() -> Self {
        Self(vec![None; Self::LEN])
    }

    pub fn hardcoded() -> Self {
        let mut mappings = Self::empty();

        // C3 -> space
        mappings.0[48] = Some(VIRTUAL_KEY(0x20));
        // C4 -> C
        mappings.0[60] = Some(VIRTUAL_KEY(0x43));
        // D4 -> D
        mappings.0[62] = Some(VIRTUAL_KEY(0x44));
        // E4 -> E
        mappings.0[64] = Some(VIRTUAL_KEY(0x45));
        // F4 -> F
        mappings.0[65] = Some(VIRTUAL_KEY(0x46));
        // G4 -> G
        mappings.0[67] = Some(VIRTUAL_KEY(0x47));

        mappings
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file_contents = fs::read_to_string(path)?;
        let file_mappings: FileMappings = match toml::from_str(&file_contents) {
            Ok(file_mappings) => file_mappings,
            Err(error) => {
                return Err(Error::Config(MappingsError {
                    inner: error,
                    source: file_contents,
                }));
            }
        };

        let mut mappings = Self::empty();
        for mapping in file_mappings.mapping {
            if let Some(key) = mappings.0.get_mut(mapping.note as usize) {
                key.replace(VIRTUAL_KEY(mapping.key as u16));
            }
        }

        Ok(mappings)
    }

    pub fn get(&self, note: u8) -> Option<VIRTUAL_KEY> {
        self.0.get(note as usize).copied().flatten()
    }
}

#[derive(Deserialize)]
struct FileMappings {
    mapping: Vec<FileMapping>,
}

#[derive(Deserialize)]
struct FileMapping {
    note: u8,
    key: u8,
}

#[derive(Debug)]
pub struct MappingsError {
    inner: toml::de::Error,
    source: String,
}

impl std::fmt::Display for MappingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Configuration error")
    }
}
impl std::error::Error for MappingsError {}

impl Diagnostic for MappingsError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new("config"))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        if let Some(span) = self.inner.span() {
            Some(Box::new(iter::once(LabeledSpan::at(
                span,
                self.inner.message(),
            ))))
        } else {
            None
        }
    }
}
