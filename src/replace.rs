use anyhow::Result;
use std::io::Write;

pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

impl Replacement {
    pub fn new(start: usize, end: usize, text: impl Into<String>) -> Replacement {
        let text = text.into();
        Replacement { start, end, text }
    }
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
    Ok(out.flush()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;
    use std::array::IntoIter;
    use std::str;

    #[test]
    fn replace_one() {
        let mut buf = Vec::new();
        let rep = &[Replacement::new(4, 4 + "hello".len(), "goodbye")];
        replace_all(&mut buf, "hi! hello world!", rep).unwrap();
        let o = str::from_utf8(&buf).unwrap();
        assert_eq!(o, "hi! goodbye world!");
    }

    #[test]
    fn replace_multiple() {
        let mut buf = Vec::new();
        let rep = &[
            Replacement::new(0, "hi!".len(), "woo!"),
            Replacement::new(4, 4 + "hello".len(), "goodbye"),
            Replacement::new(10, 10 + "world".len(), "universe"),
        ];
        replace_all(&mut buf, "hi! hello world!", rep).unwrap();
        let o = str::from_utf8(&buf).unwrap();
        assert_eq!(o, "woo! goodbye universe!");
    }

    #[test]
    fn replace_entire() {
        let mut buf = Vec::new();
        let rep = &[Replacement::new(0, "hello".len(), "goodbye")];
        replace_all(&mut buf, "hello", rep).unwrap();
        let o = str::from_utf8(&buf).unwrap();
        assert_eq!(o, "goodbye");
    }

    #[test]
    fn no_replacement() {
        for i in IntoIter::new(["", "foo"]) {
            let mut buf = Vec::new();
            replace_all(&mut buf, i, &[]).unwrap();
            let o = str::from_utf8(&buf).unwrap();
            assert_eq!(i, o);
        }
    }

    #[test]
    fn write_error() {
        assert!(replace_all(WriteErrorWriter, "foo", &[]).is_err());
    }

    #[test]
    fn flush_error() {
        assert!(replace_all(FlushErrorWriter, "foo", &[]).is_err());
    }
}
