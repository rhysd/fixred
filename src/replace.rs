use anyhow::Result;
use std::io::Write;

pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub fn replace_all<W: Write>(mut out: W, input: &str, replacements: &[Replacement]) -> Result<()> {
    let mut i = 0;
    for replacement in replacements.iter() {
        let Replacement { start, end, text } = replacement;
        out.write_all(input[i..*start].as_bytes())?;
        out.write_all(text.as_bytes())?;
        i = *end;
    }
    out.write_all(input[i..].as_bytes())?;
    Ok(())
}
