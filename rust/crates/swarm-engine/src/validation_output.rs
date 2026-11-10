//! Output retention, separate from process lifetime and execution authority.
use crate::{EngineSettingsError, EngineSettingsValues};
use std::io::{self, Write};

pub struct ValidationOutput {
    limit: Option<usize>,
    retained: Vec<u8>,
    total: u64,
}
impl std::fmt::Debug for ValidationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationOutput")
            .field("limit", &self.limit)
            .field("retained_bytes", &self.retained.len())
            .field("total", &self.total)
            .finish()
    }
}
impl ValidationOutput {
    pub fn from_settings(values: &EngineSettingsValues) -> Result<Self, EngineSettingsError> {
        Ok(Self {
            limit: values.validation_output_limit()?,
            retained: Vec::new(),
            total: 0,
        })
    }
    pub fn retained(&self) -> &[u8] {
        &self.retained
    }
    pub fn total(&self) -> u64 {
        self.total
    }
    pub fn truncated(&self) -> bool {
        self.total > self.retained.len() as u64
    }
}
impl Write for ValidationOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let total = self
            .total
            .checked_add(u64::try_from(bytes.len()).map_err(io::Error::other)?)
            .ok_or_else(|| io::Error::other("validation output count representation"))?;
        let retained = self.limit.map_or(bytes.len(), |limit| {
            bytes.len().min(limit - self.retained.len())
        });
        self.retained
            .try_reserve(retained)
            .map_err(io::Error::other)?;
        self.retained.extend_from_slice(&bytes[..retained]);
        self.total = total;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_counter_overflow_refuses_without_mutation_and_debug_keeps_bytes_private() {
        let values = EngineSettingsValues::installation_defaults().unwrap();
        let mut output = ValidationOutput::from_settings(&values).unwrap();
        output.write_all(b"private-output").unwrap();
        assert!(!format!("{output:?}").contains("private-output"));
        output.total = u64::MAX;
        let before = output.retained.clone();
        assert!(output.write(b"x").is_err());
        assert_eq!(output.retained, before);
        assert_eq!(output.total, u64::MAX);
        output.total = before.len() as u64;
        output.write_all(b"x").unwrap();
        assert_eq!(output.total(), before.len() as u64 + 1);
    }
}
