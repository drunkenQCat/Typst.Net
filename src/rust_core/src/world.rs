use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::{DateTime, Datelike, Local};
use typst::diag::{FileError, FileResult, StrResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryBuilder, World};
use typst_kit::{
    fonts::FontSearcher,
    package::PackageStorage,
};

use crate::download::SlientDownload;


/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The root relative to which absolute paths are resolved.
    root: PathBuf,
    /// The input path.
    main: FileId,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts.
    book: LazyHash<FontBook>,
    /// Locations of and storage for lazily loaded fonts.
    fonts: Arc<typst_kit::fonts::Fonts>,
    /// Maps file ids to source files and buffers.
    slots: Mutex<HashMap<FileId, FileSlot>>,
    /// Holds information about where packages are stored.
    package_storage: PackageStorage,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation. Reset between compilations.
    now: OnceLock<DateTime<Local>>,
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.slot(id, |slot| slot.source(&self.root, &self.package_storage))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id, |slot| slot.file(&self.root, &self.package_storage))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.fonts[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = self.now.get_or_init(chrono::Local::now);

        let naive = match offset {
            None => now.naive_local(),
            Some(o) => now.naive_utc() + chrono::Duration::hours(o),
        };

        Datetime::from_ymd(
            naive.year(),
            naive.month().try_into().ok()?,
            naive.day().try_into().ok()?,
        )
    }
}

impl SystemWorld {
    pub fn new(
        root: PathBuf,
        font_paths: &[PathBuf],
        inputs: typst::foundations::Dict,
        main_content: &str,
        include_system_fonts: bool,
    ) -> StrResult<Self> {
        let mut font_searcher = FontSearcher::new();
        font_searcher.include_system_fonts(include_system_fonts);
        let fonts = font_searcher.search_with(font_paths);

        let mut slots = HashMap::new();
        let main_id = FileId::new_fake(VirtualPath::new("<main>"));
        let mut main_slot = FileSlot::new(main_id);
        main_slot.source.init(Source::new(main_id, main_content.to_string()));
        slots.insert(main_id, main_slot);

        Ok(Self {
            root,
            main: main_id,
            library: LazyHash::new(
                LibraryBuilder::default()
                    .with_features(vec![typst::Feature::Html].into_iter().collect())
                    .with_inputs(inputs)
                    .build(),
            ),
            book: LazyHash::new(fonts.book.clone()),
            fonts: Arc::new(fonts),
            slots: Mutex::new(slots),
            package_storage: PackageStorage::new(None, None, crate::download::downloader()),
            now: OnceLock::new(),
        })
    }

    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock().unwrap();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

struct FileSlot {
    id: FileId,
    source: SlotCell<Source>,
    file: SlotCell<Bytes>,
}

impl FileSlot {
    fn new(id: FileId) -> Self {
        Self {
            id,
            file: SlotCell::new(),
            source: SlotCell::new(),
        }
    }

    fn source(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
    ) -> FileResult<Source> {
        let id = self.id;
        self.source.get_or_init(
            || system_path(project_root, id, package_storage),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    fn file(&mut self, project_root: &Path, package_storage: &PackageStorage) -> FileResult<Bytes> {
        let id = self.id;
        self.file.get_or_init(
            || system_path(project_root, id, package_storage),
            |data, _| Ok(Bytes::new(data)),
        )
    }
}

fn system_path(root: &Path, id: FileId, package_storage: &PackageStorage) -> FileResult<PathBuf> {
    let buf;
    let mut root = root;
    if let Some(spec) = id.package() {
        buf = package_storage.prepare_package(spec, &mut SlientDownload(&spec))?;
        root = &buf;
    }
    id.vpath().resolve(root).ok_or(FileError::AccessDenied)
}

struct SlotCell<T> {
    data: Option<FileResult<T>>,
    fingerprint: u128,
    accessed: bool,
}

impl<T: Clone> SlotCell<T> {
    fn new() -> Self {
        Self {
            data: None,
            fingerprint: 0,
            accessed: false,
        }
    }

    fn init(&mut self, data: T) {
        self.data = Some(Ok(data));
        self.accessed = true;
    }

    fn get_or_init(
        &mut self,
        path: impl FnOnce() -> FileResult<PathBuf>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        if std::mem::replace(&mut self.accessed, true) {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let result = path().and_then(|p| read(&p));
        let fingerprint = typst::utils::hash128(&result);

        if std::mem::replace(&mut self.fingerprint, fingerprint) == fingerprint {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

fn read(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    Ok(std::str::from_utf8(
        buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf),
    )?)
}
