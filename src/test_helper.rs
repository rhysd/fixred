use crate::resolve::Resolver;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time;

pub(crate) struct WriteErrorWriter;
impl Write for WriteErrorWriter {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        Err(Error::new(ErrorKind::Other, "test"))
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

pub(crate) struct FlushErrorWriter;
impl Write for FlushErrorWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Err(Error::new(ErrorKind::Other, "test"))
    }
}

pub(crate) enum TestDirEntry<'a> {
    Dir(&'a str),
    File(&'a str, &'a str),
    Binary(&'a str, &'a [u8]),
}

impl<'a> TestDirEntry<'a> {
    fn path(&self, root: &Path) -> PathBuf {
        let mut path = root.to_owned();
        let p = match self {
            Self::Dir(p) => p,
            Self::File(p, _) => p,
            Self::Binary(p, _) => p,
        };
        for name in p.split('/').filter(|n| !n.is_empty()) {
            path.push(name);
        }
        path
    }

    fn create(&self, root: &Path) -> Result<Option<(PathBuf, String)>> {
        let path = self.path(root);
        match self {
            TestDirEntry::Dir(_) => {
                fs::create_dir(&path)?;
                Ok(None)
            }
            TestDirEntry::File(_, content) => {
                fs::write(&path, content.as_bytes())?;
                Ok(Some((path, content.to_string())))
            }
            TestDirEntry::Binary(_, content) => {
                fs::write(&path, content)?;
                Ok(None)
            }
        }
    }
}

// Since tests are run in parallel, test directory name must contains unique ID to avoid name conflict.
static TEST_DIR_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct TestDir {
    pub root: PathBuf,
    pub files: Vec<(PathBuf, String)>,
}

impl TestDir {
    pub fn new<'a>(iter: impl IntoIterator<Item = &'a TestDirEntry<'a>>) -> Result<Self> {
        let mut root = env::temp_dir();
        let root_name = format!(
            "redfix-test-{}-{}",
            TEST_DIR_ID.fetch_add(1, Ordering::Relaxed),
            time::SystemTime::now()
                .duration_since(time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        root.push(root_name);
        fs::create_dir(&root)?;
        let mut dir = TestDir {
            root,
            files: vec![],
        };
        for p in iter {
            if let Some((p, c)) = p.create(&dir.root)? {
                dir.files.push((p, c));
            }
        }
        Ok(dir)
    }

    pub fn delete(&self) -> Result<()> {
        fs::remove_dir_all(&self.root)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        self.delete().unwrap();
    }
}

pub(crate) fn assert_files(files: &[(PathBuf, String)]) {
    for (path, want) in files {
        let have = fs::read_to_string(path).unwrap();
        assert_eq!(want, &have, "content in {:?} mismatched", path);
    }
}

// redirect foo -> bar -> piyo
#[derive(Default)]
pub(crate) struct FooToPiyoResolver {
    pub shallow: bool,
}

impl Resolver for FooToPiyoResolver {
    fn shallow(&mut self, b: bool) {
        self.shallow = b;
    }
    fn resolve(&self, url: &str) -> Option<String> {
        let to = if self.shallow { "bar" } else { "piyo" };
        let new = url.replace("foo", to);
        (url != new).then(|| new)
    }
}
