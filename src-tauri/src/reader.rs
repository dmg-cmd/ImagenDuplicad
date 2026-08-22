use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub struct FileData {
    pub mmap: Mmap,
}

impl FileData {
    pub fn open(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mmap = unsafe { Mmap::map(&file).ok()? };
        Some(Self { mmap })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.mmap
    }
}
