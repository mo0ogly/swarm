//! Descriptor-owned initial stage. No deletion, chmod, retry or pathname adoption.
use super::{combined, io, Context, Error, IoPhase, Mode, Progress, Publication};
use rustix::fs::{self as unix, AtFlags, FileType, Mode as FileMode, OFlags, RenameFlags};
use std::{
    ffi::OsStr,
    fs::File,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
};
/// Six sibling roots preserve the bootstrap loader's no-overlap contract.
const CHILDREN: [&str; 6] = [
    "identity",
    "business",
    "personal",
    "configuration",
    "configuration-witness",
    "personal-witness",
];
/// Retained parent, root and all six directory descriptors; no Clone. Paths are
/// rechecked against held objects before/after pathname-based journal operations.
pub(super) struct Tree {
    files: Vec<File>,
    names: Vec<String>,
    parent_path: PathBuf,
    pub path: PathBuf,
    final_name: String,
    owner: u32,
}
/// Close once and preserve the actual error; never retry a possibly reused FD.
fn close(file: File) -> Result<(), Error> {
    nix::unistd::close(file)
        .map_err(|e| io(IoPhase::Close, std::io::Error::from_raw_os_error(e as i32)))
}
/// Attempt every close, retaining independent errors in original order.
fn close_all(files: Vec<File>) -> Result<(), Error> {
    let mut result = Ok(());
    for f in files.into_iter().rev() {
        result = combined(result, close(f));
    }
    result
}
/// Check regular0600single-link/euid object before SQLite or sync can touch it.
fn private_file(f: &File, uid: u32) -> Result<(), Error> {
    let s = unix::fstat(f).map_err(|e| io(IoPhase::Stat, e))?;
    if FileType::from_raw_mode(s.st_mode) != FileType::RegularFile
        || s.st_mode & 0o7777 != 0o600
        || s.st_uid != uid
        || s.st_nlink != 1
    {
        Err(Error::Binding)
    } else {
        Ok(())
    }
}
/// Validate both held file and current entry before/after sync. Inode identity
/// never comes from an operator label or a pathname alone.
fn named_file(dir: &File, name: &OsStr, file: &File, uid: u32) -> Result<(), Error> {
    private_file(file, uid)?;
    let a = unix::fstat(file).map_err(|e| io(IoPhase::Stat, e))?;
    let b = unix::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|e| io(IoPhase::Stat, e))?;
    if a.st_dev != b.st_dev || a.st_ino != b.st_ino {
        Err(Error::Binding)
    } else {
        Ok(())
    }
}
/// Compare actual held object and current name, including private directory mode.
fn directory(parent: &File, name: &OsStr, held: &File, uid: u32) -> Result<(), Error> {
    let a = unix::fstat(held).map_err(|e| io(IoPhase::Stat, e))?;
    let b =
        unix::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|e| io(IoPhase::Stat, e))?;
    if a.st_dev != b.st_dev
        || a.st_ino != b.st_ino
        || a.st_uid != uid
        || a.st_mode & 0o7777 != 0o700
        || FileType::from_raw_mode(a.st_mode) != FileType::Directory
    {
        Err(Error::Binding)
    } else {
        Ok(())
    }
}
/// Open one child directory without following links; caller retains/explicitly closes.
fn open_dir(parent: &File, name: &str) -> Result<File, Error> {
    unix::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        FileMode::empty(),
    )
    .map(File::from)
    .map_err(|e| io(IoPhase::Open, e))
}
/// A missing name is an explicit precondition; all other stat errors remain errors.
fn absent(parent: &File, name: &str) -> Result<(), Error> {
    match unix::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(()),
        Ok(_) => Err(Error::Exists),
        Err(e) => Err(io(IoPhase::Stat, e)),
    }
}
impl Tree {
    /// Create exclusively, reopen an explicitly chosen stage, or inspect the final
    /// root. Real euid/private parent required before effects; all acquired FDs close
    /// on failure. Partial stages remain for pinned explicit recovery, never deleted.
    pub fn open(
        ctx: &Context,
        root: &str,
        uid: u32,
        mode: Mode,
        progress: &mut Progress,
    ) -> Result<Self, Error> {
        ctx.check()?;
        let path = Path::new(root);
        let parent_path = path.parent().ok_or(Error::Binding)?.to_path_buf();
        let final_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(Error::Binding)?
            .to_owned();
        let name = if mode == Mode::RecoverPublished {
            final_name.clone()
        } else {
            format!("{final_name}.paea-stage-v1")
        };
        let parent = swarm_enrollment_config::store::open_private_directory(&parent_path, uid)
            .map_err(Error::Storage)?;
        let mut files = vec![parent];
        let result = (|| {
            ctx.check()?;
            if mode != Mode::RecoverPublished {
                absent(&files[0], &final_name)?;
            }
            if mode == Mode::Create {
                progress.stage = Publication::Unknown;
                match unix::mkdirat(&files[0], name.as_str(), FileMode::from_raw_mode(0o700)) {
                    Ok(()) => progress.stage = Publication::Visible,
                    Err(rustix::io::Errno::EXIST) => {
                        progress.stage = Publication::NotAttempted;
                        return Err(Error::Exists);
                    }
                    Err(e) => return Err(io(IoPhase::Create, e)),
                }
            }
            files.push(open_dir(&files[0], &name)?);
            unix::flock(&files[1], unix::FlockOperation::NonBlockingLockExclusive).map_err(
                |e| {
                    if e == rustix::io::Errno::WOULDBLOCK {
                        Error::Storage(swarm_enrollment_config::Error::Busy)
                    } else {
                        io(IoPhase::Open, e)
                    }
                },
            )?;
            directory(&files[0], OsStr::new(&name), &files[1], uid)?;
            if mode != Mode::Create {
                progress.stage = Publication::Visible;
            }
            Ok(())
        })();
        if let Err(e) = result {
            return combined(Err(e), close_all(files));
        }
        let path = parent_path.join(&name);
        let names = vec![name];
        Ok(Self {
            files,
            names,
            parent_path,
            path,
            final_name,
            owner: uid,
        })
    }
    /// Current root and every child still name the exact held inode; revalidate the
    /// absolute parent with shared nofollow ancestor protection as well. No OS fence.
    pub fn check(&self, ctx: &Context) -> Result<(), Error> {
        ctx.check()?;
        let current =
            swarm_enrollment_config::store::open_private_directory(&self.parent_path, self.owner)
                .map_err(Error::Storage)?;
        let result = (|| {
            let a = unix::fstat(&self.files[0]).map_err(|e| io(IoPhase::Stat, e))?;
            let b = unix::fstat(&current).map_err(|e| io(IoPhase::Stat, e))?;
            if a.st_dev != b.st_dev || a.st_ino != b.st_ino {
                return Err(Error::Binding);
            }
            directory(
                &self.files[0],
                OsStr::new(&self.names[0]),
                &self.files[1],
                self.owner,
            )?;
            for (i, name) in self.names.iter().enumerate().skip(1) {
                directory(
                    &self.files[1],
                    OsStr::new(name),
                    &self.files[i + 1],
                    self.owner,
                )?;
            }
            let mut seen = Vec::new();
            for file in &self.files {
                let s = unix::fstat(file).map_err(|e| io(IoPhase::Stat, e))?;
                if seen.contains(&(s.st_dev, s.st_ino)) {
                    return Err(Error::Binding);
                }
                seen.push((s.st_dev, s.st_ino));
            }
            Ok(())
        })();
        combined(result, close(current))
    }
    /// Empty pre-intent stage may resume only when no data exists anywhere. The
    /// installer lock itself is the only allowed file; no stored identity is adopted.
    pub fn empty(&self) -> Result<bool, Error> {
        for name in Self::entries(&self.files[1])? {
            if name == OsStr::new("private-documents.lock") {
                continue;
            }
            let Some(child) = CHILDREN.iter().find(|c| name == OsStr::new(c)) else {
                return Ok(false);
            };
            let f = open_dir(&self.files[1], child)?;
            let result = directory(&self.files[1], OsStr::new(child), &f, self.owner)
                .and_then(|()| Self::entries(&f).map(|e| e.is_empty()));
            if !combined(result, close(f))? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Complete only the fixed private skeleton AFTER verified durable intent.
    /// Retain every acquired descriptor for outer cleanup, even on partial error.
    pub fn children(&mut self, ctx: &Context, mode: Mode) -> Result<(), Error> {
        self.check(ctx)?;
        for child in CHILDREN {
            ctx.check()?;
            if mode != Mode::RecoverPublished {
                match unix::mkdirat(&self.files[1], child, FileMode::from_raw_mode(0o700)) {
                    Ok(()) => (),
                    Err(rustix::io::Errno::EXIST) if mode == Mode::ResumeStage => (),
                    Err(e) => return Err(io(IoPhase::Create, e)),
                }
            }
            self.files.push(open_dir(&self.files[1], child)?);
            self.names.push(child.into());
            directory(
                &self.files[1],
                OsStr::new(child),
                self.files.last().ok_or(Error::Binding)?,
                self.owner,
            )?;
        }
        self.check(ctx)
    }
    /// Root may contain only initial child directories, its intent and lock. No
    /// child creation/repair is allowed before that intent has been verified.
    pub fn root_inventory(&self) -> Result<(), Error> {
        for n in Self::entries(&self.files[1])? {
            if !CHILDREN.iter().any(|c| n == OsStr::new(c))
                && n != OsStr::new("service-installation-v1.intent")
                && n != OsStr::new("private-documents.lock")
            {
                return Err(Error::Binding);
            }
        }
        Ok(())
    }
    /// Deterministic original staging path for comparing immutable stage documents
    /// during published recovery. This path is data, never opened after publication.
    pub fn original_stage(&self) -> PathBuf {
        self.parent_path
            .join(format!("{}.paea-stage-v1", self.final_name))
    }
    /// Acknowledge the stage/root name only after its exact independent intent
    /// has been verified. Opening an existing mismatched root never fsyncs it.
    pub fn acknowledge_stage(&self, ctx: &Context, progress: &mut Progress) -> Result<(), Error> {
        self.check(ctx)?;
        self.files[1].sync_all().map_err(|e| io(IoPhase::Sync, e))?;
        self.files[0].sync_all().map_err(|e| io(IoPhase::Sync, e))?;
        progress.stage = Publication::Durable;
        self.check(ctx)
    }
    /// Resolve only fixed internal child names to their retained descriptor.
    fn child(&self, n: &str) -> Result<&File, Error> {
        let index = CHILDREN
            .iter()
            .position(|c| *c == n)
            .ok_or(Error::Binding)?;
        self.files.get(index + 2).ok_or(Error::Binding)
    }
    /// Read names through the held Linux descriptor, not an operator-selected path.
    /// Unknown entries refuse rather than disappearing during root publication.
    fn entries(dir: &File) -> Result<Vec<std::ffi::OsString>, Error> {
        std::fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd()))
            .map_err(|e| io(IoPhase::ReadDirectory, e))?
            .map(|e| {
                e.map(|v| v.file_name())
                    .map_err(|e| io(IoPhase::ReadDirectory, e))
            })
            .collect()
    }
    /// Closed initial tree inventory. SQLite sidecars and known lock names are
    /// protocol outputs, not extension points or arbitrary files to be published.
    pub fn inventory(&self) -> Result<(), Error> {
        self.root_inventory()?;
        for child in CHILDREN {
            let allowed: &[&str] = match child {
                "identity" => &[
                    "private-documents.lock",
                    "service-installation-v1.intent",
                    "bootstrap-v3.json",
                    "bootstrap-control-v2.json",
                    "staging-v3.json",
                    "staging-control-v2.json",
                    "personal-authority.seed",
                ],
                "business" | "personal" => {
                    &["journal.sqlite", "journal.sqlite-wal", "journal.sqlite-shm"]
                }
                "personal-witness" => &["private-documents.lock"],
                _ => &["config.lock"],
            };
            let dir = self.child(child)?;
            for n in Self::entries(dir)? {
                if !allowed.iter().any(|a| n == OsStr::new(a)) {
                    return Err(Error::Binding);
                }
                self.checked_file(dir, &n, false)?;
            }
        }
        Ok(())
    }
    /// Open/check/close one regular file. Create an absent DB only after intent;
    /// unsafe existing objects never turn into successful empty journals.
    fn checked_file(&self, dir: &File, name: &OsStr, create: bool) -> Result<(), Error> {
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        let f = if create {
            match unix::openat(
                dir,
                name,
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::EXCL
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC
                    | OFlags::NONBLOCK,
                FileMode::from_raw_mode(0o600),
            ) {
                Ok(f) => f,
                Err(rustix::io::Errno::EXIST) => unix::openat(dir, name, flags, FileMode::empty())
                    .map_err(|e| io(IoPhase::Open, e))?,
                Err(e) => return Err(io(IoPhase::Create, e)),
            }
        } else {
            unix::openat(dir, name, flags, FileMode::empty()).map_err(|e| io(IoPhase::Open, e))?
        };
        let f = File::from(f);
        let result = named_file(dir, name, &f, self.owner);
        combined(result, close(f))
    }
    /// Refuse unsafe preexisting DB/WAL/SHM before SQLite can follow any of them.
    pub fn journals(&self, ctx: &Context, create: bool) -> Result<(), Error> {
        self.check(ctx)?;
        self.inventory()?;
        for child in ["business", "personal"] {
            let dir = self.child(child)?;
            self.checked_file(dir, OsStr::new("journal.sqlite"), create)?;
            for name in ["journal.sqlite-wal", "journal.sqlite-shm"] {
                match unix::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
                    Err(rustix::io::Errno::NOENT) => (),
                    Err(e) => return Err(io(IoPhase::Stat, e)),
                    Ok(_) => self.checked_file(dir, OsStr::new(name), false)?,
                }
            }
        }
        self.check(ctx)
    }
    /// Flush all checked regular files, then child directories/root. Explicitly
    /// close each file and preserve a separate sync/close failure. No recursive path
    /// traversal, extra file adoption, hardware guarantee or current-authority claim.
    pub fn sync(&self, ctx: &Context) -> Result<(), Error> {
        self.check(ctx)?;
        self.inventory()?;
        for child in CHILDREN {
            let dir = self.child(child)?;
            for name in Self::entries(dir)? {
                ctx.check()?;
                let f = unix::openat(
                    dir,
                    name.as_os_str(),
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                    FileMode::empty(),
                )
                .map(File::from)
                .map_err(|e| io(IoPhase::Open, e))?;
                let result = named_file(dir, name.as_os_str(), &f, self.owner)
                    .and_then(|()| f.sync_all().map_err(|e| io(IoPhase::Sync, e)))
                    .and_then(|()| named_file(dir, name.as_os_str(), &f, self.owner));
                combined(result, close(f))?;
            }
            dir.sync_all().map_err(|e| io(IoPhase::Sync, e))?;
        }
        self.files[1].sync_all().map_err(|e| io(IoPhase::Sync, e))?;
        self.check(ctx)
    }
    /// Atomic Linux NOREPLACE after all prerequisite checks. EEXIST proves this
    /// rename did not replace the competing name; other rename errors stay Unknown.
    /// Update retained current name immediately after success, before any later hook.
    pub fn publish(&mut self, ctx: &Context, progress: &mut Progress) -> Result<(), Error> {
        self.check(ctx)?;
        progress.published = Publication::Unknown;
        match unix::renameat_with(
            &self.files[0],
            self.names[0].as_str(),
            &self.files[0],
            self.final_name.as_str(),
            RenameFlags::NOREPLACE,
        ) {
            Ok(()) => (),
            Err(rustix::io::Errno::EXIST) => {
                progress.published = Publication::NotAttempted;
                return Err(Error::Exists);
            }
            Err(e) => return Err(io(IoPhase::Rename, e)),
        };
        self.names[0] = self.final_name.clone();
        self.path = self.parent_path.join(&self.final_name);
        progress.published = Publication::Visible;
        self.check(ctx)
    }
    /// Acknowledge the final directory entry only after explicit parent fsync.
    /// A later check/close error preserves Durable in progress, never erases effects.
    pub fn acknowledge(&self, ctx: &Context, progress: &mut Progress) -> Result<(), Error> {
        self.check(ctx)?;
        self.files[0].sync_all().map_err(|e| io(IoPhase::Sync, e))?;
        progress.published = Publication::Durable;
        self.check(ctx)
    }
    /// Consume all directory owners, attempting every close even after a failure.
    pub fn close(self) -> Result<(), Error> {
        let released = unix::flock(&self.files[1], unix::FlockOperation::Unlock)
            .map_err(|e| io(IoPhase::Close, e));
        combined(released, close_all(self.files))
    }
}
