use std::{
    collections::VecDeque,
    ffi::OsString,
    fs::{FileType, Metadata, Permissions, OpenOptions as StdOpenOptions},
    io::{self, SeekFrom},
    fmt,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    os::unix::fs::OpenOptionsExt
};

pub(crate) fn spawn_blocking<F, R>(_f: F) -> JoinHandle<R>
where
F: FnOnce() -> R + Send + 'static,
R: Send + 'static,
{
    assert_send_sync::<JoinHandle<std::cell::Cell<()>>>();
    panic!("requires the `rt` Tokio feature flag")
}

// cfg_fs! {
//     pub(crate) fn spawn_mandatory_blocking<F, R>(_f: F) -> Option<JoinHandle<R>>
//         where
//         F: FnOnce() -> R + Send + 'static,
//         R: Send + 'static,
//         {
//             panic!("requires the `rt` Tokio feature flag")
//         }
// }

pub(crate) struct JoinHandle<R> {
    _p: std::marker::PhantomData<R>,
}

unsafe impl<T: Send> Send for JoinHandle<T> {}
unsafe impl<T: Send> Sync for JoinHandle<T> {}

impl<R> Future for JoinHandle<R> {
    type Output = Result<R, std::io::Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        unreachable!()
    }
}

impl<T> fmt::Debug for JoinHandle<T>
where
T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("JoinHandle").finish()
    }
}

fn assert_send_sync<T: Send + Sync>() {
}
// feature! {
//     #![unix]
//
//     mod symlink;
//     pub use self::symlink::symlink;
// }
//
// cfg_windows! {
//     mod symlink_dir;
//     pub use self::symlink_dir::symlink_dir;
//
//     mod symlink_file;
//     pub use self::symlink_file::symlink_file;
// }
//

pub(crate) async fn asyncify<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            "background task failed",
        )),
    }
}

pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    let contents = contents.as_ref().to_owned();

    asyncify(move || std::fs::write(path, contents)).await
}

pub async fn try_exists(path: impl AsRef<Path>) -> io::Result<bool> {
    let path = path.as_ref().to_owned();
    // std's Path::try_exists is not available for current Rust min supported version.
    // Current implementation is based on its internal implementation instead.
    match asyncify(move || std::fs::metadata(path)).await {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub async fn symlink_metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref().to_owned();
    asyncify(|| std::fs::symlink_metadata(path)).await
}

// pub async fn symlink_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
//     let src = src.as_ref().to_owned();
//     let dst = dst.as_ref().to_owned();
//
//     asyncify(move || std::os::windows::fs::symlink_file(src, dst)).await
// }

// pub async fn symlink_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
//     let src = src.as_ref().to_owned();
//     let dst = dst.as_ref().to_owned();
//
//     asyncify(move || std::os::windows::fs::symlink_dir(src, dst)).await
// }

pub async fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let src = src.as_ref().to_owned();
    let dst = dst.as_ref().to_owned();

    asyncify(move || std::os::unix::fs::symlink(src, dst)).await
}

pub async fn set_permissions(path: impl AsRef<Path>, perm: Permissions) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(|| std::fs::set_permissions(path, perm)).await
}

pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();

    asyncify(move || std::fs::rename(from, to)).await
}

pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::remove_file(path)).await
}

pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::remove_dir_all(path)).await
}

pub async fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::remove_dir(path)).await
}

pub async fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::read_to_string(path)).await
}

pub async fn read_link(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::read_link(path)).await
}

//read_Dir
// #[cfg(test)]
// use super::mocks::spawn_blocking;
// #[cfg(test)]
// use super::mocks::JoinHandle;

const CHUNK_SIZE: usize = 32;

pub async fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let path = path.as_ref().to_owned();
    asyncify(|| -> io::Result<ReadDir> {
        let mut std = std::fs::read_dir(path)?;
        let mut buf = VecDeque::with_capacity(CHUNK_SIZE);
        let remain = ReadDir::next_chunk(&mut buf, &mut std);

        Ok(ReadDir(State::Idle(Some((buf, std, remain)))))
    })
    .await
}

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ReadDir(State);

#[derive(Debug)]
enum State {
    Idle(Option<(VecDeque<io::Result<DirEntry>>, std::fs::ReadDir, bool)>),
    Pending(JoinHandle<(VecDeque<io::Result<DirEntry>>, std::fs::ReadDir, bool)>),
}

impl ReadDir {
    pub async fn next_entry(&mut self) -> io::Result<Option<DirEntry>> {
        use crate::future::poll_fn;
        poll_fn(|cx| self.poll_next_entry(cx)).await
    }

    pub fn poll_next_entry(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<Option<DirEntry>>> {
        loop {
            match self.0 {
                State::Idle(ref mut data) => {
                    let (buf, _, ref remain) = data.as_mut().unwrap();

                    if let Some(ent) = buf.pop_front() {
                        return Poll::Ready(ent.map(Some));
                    } else if !remain {
                        return Poll::Ready(Ok(None));
                    }

                    let (mut buf, mut std, _) = data.take().unwrap();

                    self.0 = State::Pending(spawn_blocking(move || {
                        let remain = ReadDir::next_chunk(&mut buf, &mut std);
                        (buf, std, remain)
                    }));
                }
                State::Pending(ref mut rx) => {
                    self.0 = State::Idle(Some(ready!(Pin::new(rx).poll(cx))?));
                }
            }
        }
    }

    fn next_chunk(buf: &mut VecDeque<io::Result<DirEntry>>, std: &mut std::fs::ReadDir) -> bool {
        for _ in 0..CHUNK_SIZE {
            let ret = match std.next() {
                Some(ret) => ret,
                None => return false,
            };

            let success = ret.is_ok();

            buf.push_back(ret.map(|std| DirEntry {
                #[cfg(not(any(
                    target_os = "solaris",
                    target_os = "illumos",
                    target_os = "haiku",
                    target_os = "vxworks",
                    target_os = "aix",
                    target_os = "nto",
                    target_os = "vita",
                )))]
                file_type: std.file_type().ok(),
                std: Arc::new(std),
            }));

            if !success {
                break;
            }
        }

        true
    }
}

// feature! {
//     #![unix]
//
//     use std::os::unix::fs::DirEntryExt;
//
//     impl DirEntry {
//         pub fn ino(&self) -> u64 {
//             self.as_inner().ino()
//         }
//     }
// }

#[derive(Debug)]
pub struct DirEntry {
    #[cfg(not(any(
        target_os = "solaris",
        target_os = "illumos",
        target_os = "haiku",
        target_os = "vxworks",
        target_os = "aix",
        target_os = "nto",
        target_os = "vita",
    )))]
    file_type: Option<FileType>,
    std: Arc<std::fs::DirEntry>,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.std.path()
    }

    pub fn file_name(&self) -> OsString {
        self.std.file_name()
    }

    pub async fn metadata(&self) -> io::Result<Metadata> {
        let std = self.std.clone();
        asyncify(move || std.metadata()).await
    }

    pub async fn file_type(&self) -> io::Result<FileType> {
        #[cfg(not(any(
            target_os = "solaris",
            target_os = "illumos",
            target_os = "haiku",
            target_os = "vxworks",
            target_os = "aix",
            target_os = "nto",
            target_os = "vita",
        )))]
        if let Some(file_type) = self.file_type {
            return Ok(file_type);
        }

        let std = self.std.clone();
        asyncify(move || std.file_type()).await
    }

    /// Returns a reference to the underlying `std::fs::DirEntry`.
    #[cfg(unix)]
    pub(super) fn as_inner(&self) -> &std::fs::DirEntry {
        &self.std
    }
}

pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::read(path)).await
}

// Open_Options
#[derive(Clone, Debug)]
pub struct Open_Options(StdOpenOptions);

impl Open_Options {
    pub fn new() -> Open_Options {
        Open_Options(StdOpenOptions::new())
    }

    pub fn read(&mut self, read: bool) -> &mut Open_Options {
        self.0.read(read);
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Open_Options {
        self.0.write(write);
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Open_Options {
        self.0.append(append);
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Open_Options {
        self.0.truncate(truncate);
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Open_Options {
        self.0.create(create);
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Open_Options {
        self.0.create_new(create_new);
        self
    }

    pub async fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let opts = self.0.clone();

        let std = asyncify(move || opts.open(path)).await?;
        Ok(File::from_std(std))
    }

    /// Returns a mutable reference to the underlying `std::fs::OpenOptions`
    pub(super) fn as_inner_mut(&mut self) -> &mut StdOpenOptions {
        &mut self.0
    }
}

// feature! {
//     #![unix]
//
//     impl OpenOptions {
//         pub fn mode(&mut self, mode: u32) -> &mut OpenOptions {
//             self.as_inner_mut().mode(mode);
//             self
//         }
//
//         pub fn custom_flags(&mut self, flags: i32) -> &mut OpenOptions {
//             self.as_inner_mut().custom_flags(flags);
//             self
//         }
//     }
// }

impl From<StdOpenOptions> for Open_Options {
    fn from(options: StdOpenOptions) -> Open_Options {
        Open_Options(options)
    }
}

impl Default for Open_Options {
    fn default() -> Self {
        Self::new()
    }
}

// mocks


pub async fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref().to_owned();
    asyncify(|| std::fs::metadata(path)).await
}

pub async fn hard_link(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let src = src.as_ref().to_owned();
    let dst = dst.as_ref().to_owned();

    asyncify(move || std::fs::hard_link(src, dst)).await
}

// file
// #[cfg(test)]
// use super::mocks::JoinHandle;
// #[cfg(test)]
// use super::mocks::MockFile as StdFile;
// #[cfg(test)]
// use super::mocks::{spawn_blocking, spawn_mandatory_blocking};

// #[cfg(not(test))]
// use crate::blocking::JoinHandle;
// #[cfg(not(test))]
// use crate::blocking::{spawn_blocking, spawn_mandatory_blocking};
// #[cfg(not(test))]
use std::fs::File as StdFile;

use crate::io::AsyncSeek;
use crate::io::Buf;

pub struct File {
    std: Arc<StdFile>,
    inner: Mutex<Inner>,
}

struct Inner {
    state: State,

    /// Errors from writes/flushes are returned in write/flush calls. If a write
    /// error is observed while performing a read, it is saved until the next
    /// write / flush call.
    last_write_err: Option<io::ErrorKind>,

    pos: u64,
}

#[derive(Debug)]
enum State {
    Idle(Option<Buf>),
    Busy(JoinHandle<(Operation, Buf)>),
}

#[derive(Debug)]
enum Operation {
    Read(io::Result<usize>),
    Write(io::Result<()>),
    Seek(io::Result<u64>),
}

impl File {
    pub async fn open(path: impl AsRef<Path>) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let std = asyncify(|| StdFile::open(path)).await?;

        Ok(File::from_std(std))
    }

    pub async fn create(path: impl AsRef<Path>) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let std_file = asyncify(move || StdFile::create(path)).await?;
        Ok(File::from_std(std_file))
    }

    #[must_use]
    pub fn options() -> Open_Options {
        Open_Options::new()
    }

    pub fn from_std(std: StdFile) -> File {
        File {
            std: Arc::new(std),
            inner: Mutex::new(Inner {
                state: State::Idle(Some(Buf::with_capacity(0))),
                last_write_err: None,
                pos: 0,
            }),
        }
    }

    pub async fn sync_all(&self) -> io::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.complete_inflight().await;

        let std = self.std.clone();
        asyncify(move || std.sync_all()).await
    }

    pub async fn sync_data(&self) -> io::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.complete_inflight().await;

        let std = self.std.clone();
        asyncify(move || std.sync_data()).await
    }

    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.complete_inflight().await;

        let mut buf = match inner.state {
            State::Idle(ref mut buf_cell) => buf_cell.take().unwrap(),
            _ => unreachable!(),
        };

        let seek = if !buf.is_empty() {
            Some(SeekFrom::Current(buf.discard_read()))
        } else {
            None
        };

        let std = self.std.clone();

        inner.state = State::Busy(spawn_blocking(move || {
            let res = if let Some(seek) = seek {
                (&*std).seek(seek).and_then(|_| std.set_len(size))
            } else {
                std.set_len(size)
            }
            .map(|()| 0); // the value is discarded later

            // Return the result as a seek
            (Operation::Seek(res), buf)
        }));

        let (op, buf) = match inner.state {
            State::Idle(_) => unreachable!(),
            State::Busy(ref mut rx) => rx.await?,
        };

        inner.state = State::Idle(Some(buf));

        match op {
            Operation::Seek(res) => res.map(|pos| {
                inner.pos = pos;
            }),
            _ => unreachable!(),
        }
    }

    pub async fn metadata(&self) -> io::Result<Metadata> {
        let std = self.std.clone();
        asyncify(move || std.metadata()).await
    }

    pub async fn try_clone(&self) -> io::Result<File> {
        self.inner.lock().await.complete_inflight().await;
        let std = self.std.clone();
        let std_file = asyncify(move || std.try_clone()).await?;
        Ok(File::from_std(std_file))
    }

    pub async fn into_std(mut self) -> StdFile {
        self.inner.get_mut().complete_inflight().await;
        Arc::try_unwrap(self.std).expect("Arc::try_unwrap failed")
    }

    pub fn try_into_std(mut self) -> Result<StdFile, Self> {
        match Arc::try_unwrap(self.std) {
            Ok(file) => Ok(file),
            Err(std_file_arc) => {
                self.std = std_file_arc;
                Err(self)
            }
        }
    }

    pub async fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        let std = self.std.clone();
        asyncify(move || std.set_permissions(perm)).await
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dst: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // ready!(crate::trace::trace_leaf(cx));
        let me = self.get_mut();
        let inner = me.inner.get_mut();

        loop {
            match inner.state {
                State::Idle(ref mut buf_cell) => {
                    let mut buf = buf_cell.take().unwrap();

                    if !buf.is_empty() {
                        buf.copy_to(dst);
                        *buf_cell = Some(buf);
                        return Poll::Ready(Ok(()));
                    }

                    buf.ensure_capacity_for(dst);
                    let std = me.std.clone();

                    inner.state = State::Busy(spawn_blocking(move || {
                        let res = buf.read_from(&mut &*std);
                        (Operation::Read(res), buf)
                    }));
                }
                State::Busy(ref mut rx) => {
                    let (op, mut buf) = ready!(Pin::new(rx).poll(cx))?;

                    match op {
                        Operation::Read(Ok(_)) => {
                            buf.copy_to(dst);
                            inner.state = State::Idle(Some(buf));
                            return Poll::Ready(Ok(()));
                        }
                        Operation::Read(Err(e)) => {
                            assert!(buf.is_empty());

                            inner.state = State::Idle(Some(buf));
                            return Poll::Ready(Err(e));
                        }
                        Operation::Write(Ok(())) => {
                            assert!(buf.is_empty());
                            inner.state = State::Idle(Some(buf));
                            continue;
                        }
                        Operation::Write(Err(e)) => {
                            assert!(inner.last_write_err.is_none());
                            inner.last_write_err = Some(e.kind());
                            inner.state = State::Idle(Some(buf));
                        }
                        Operation::Seek(result) => {
                            assert!(buf.is_empty());
                            inner.state = State::Idle(Some(buf));
                            if let Ok(pos) = result {
                                inner.pos = pos;
                            }
                            continue;
                        }
                    }
                }
            }
        }
    }
}

impl AsyncSeek for File {
    fn start_seek(self: Pin<&mut Self>, mut pos: SeekFrom) -> io::Result<()> {
        let me = self.get_mut();
        let inner = me.inner.get_mut();

        match inner.state {
            State::Busy(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "other file operation is pending, call poll_complete before start_seek",
            )),
            State::Idle(ref mut buf_cell) => {
                let mut buf = buf_cell.take().unwrap();

                // Factor in any unread data from the buf
                if !buf.is_empty() {
                    let n = buf.discard_read();

                    if let SeekFrom::Current(ref mut offset) = pos {
                        *offset += n;
                    }
                }

                let std = me.std.clone();

                inner.state = State::Busy(spawn_blocking(move || {
                    let res = (&*std).seek(pos);
                    (Operation::Seek(res), buf)
                }));
                Ok(())
            }
        }
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        // ready!(crate::trace::trace_leaf(cx));
        let inner = self.inner.get_mut();

        loop {
            match inner.state {
                State::Idle(_) => return Poll::Ready(Ok(inner.pos)),
                State::Busy(ref mut rx) => {
                    let (op, buf) = ready!(Pin::new(rx).poll(cx))?;
                    inner.state = State::Idle(Some(buf));

                    match op {
                        Operation::Read(_) => {}
                        Operation::Write(Err(e)) => {
                            assert!(inner.last_write_err.is_none());
                            inner.last_write_err = Some(e.kind());
                        }
                        Operation::Write(_) => {}
                        Operation::Seek(res) => {
                            if let Ok(pos) = res {
                                inner.pos = pos;
                            }
                            return Poll::Ready(res);
                        }
                    }
                }
            }
        }
    }
}

impl AsyncWrite for File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        src: &[u8],
    ) -> Poll<io::Result<usize>> {
        // ready!(crate::trace::trace_leaf(cx));
        let me = self.get_mut();
        let inner = me.inner.get_mut();

        if let Some(e) = inner.last_write_err.take() {
            return Poll::Ready(Err(e.into()));
        }

        loop {
            match inner.state {
                State::Idle(ref mut buf_cell) => {
                    let mut buf = buf_cell.take().unwrap();

                    let seek = if !buf.is_empty() {
                        Some(SeekFrom::Current(buf.discard_read()))
                    } else {
                        None
                    };

                    let n = buf.copy_from(src);
                    let std = me.std.clone();

                    let blocking_task_join_handle = spawn_mandatory_blocking(move || {
                        let res = if let Some(seek) = seek {
                            (&*std).seek(seek).and_then(|_| buf.write_to(&mut &*std))
                        } else {
                            buf.write_to(&mut &*std)
                        };

                        (Operation::Write(res), buf)
                    })
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "background task failed")
                    })?;

                    inner.state = State::Busy(blocking_task_join_handle);

                    return Poll::Ready(Ok(n));
                }
                State::Busy(ref mut rx) => {
                    let (op, buf) = ready!(Pin::new(rx).poll(cx))?;
                    inner.state = State::Idle(Some(buf));

                    match op {
                        Operation::Read(_) => {
                            // We don't care about the result here. The fact
                            // that the cursor has advanced will be reflected in
                            // the next iteration of the loop
                            continue;
                        }
                        Operation::Write(res) => {
                            // If the previous write was successful, continue.
                            // Otherwise, error.
                            res?;
                            continue;
                        }
                        Operation::Seek(_) => {
                            // Ignore the seek
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        // ready!(crate::trace::trace_leaf(cx));
        let me = self.get_mut();
        let inner = me.inner.get_mut();

        if let Some(e) = inner.last_write_err.take() {
            return Poll::Ready(Err(e.into()));
        }

        loop {
            match inner.state {
                State::Idle(ref mut buf_cell) => {
                    let mut buf = buf_cell.take().unwrap();

                    let seek = if !buf.is_empty() {
                        Some(SeekFrom::Current(buf.discard_read()))
                    } else {
                        None
                    };

                    let n = buf.copy_from_bufs(bufs);
                    let std = me.std.clone();

                    let blocking_task_join_handle = spawn_mandatory_blocking(move || {
                        let res = if let Some(seek) = seek {
                            (&*std).seek(seek).and_then(|_| buf.write_to(&mut &*std))
                        } else {
                            buf.write_to(&mut &*std)
                        };

                        (Operation::Write(res), buf)
                    })
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "background task failed")
                    })?;

                    inner.state = State::Busy(blocking_task_join_handle);

                    return Poll::Ready(Ok(n));
                }
                State::Busy(ref mut rx) => {
                    let (op, buf) = ready!(Pin::new(rx).poll(cx))?;
                    inner.state = State::Idle(Some(buf));

                    match op {
                        Operation::Read(_) => {
                            // We don't care about the result here. The fact
                            // that the cursor has advanced will be reflected in
                            // the next iteration of the loop
                            continue;
                        }
                        Operation::Write(res) => {
                            // If the previous write was successful, continue.
                            // Otherwise, error.
                            res?;
                            continue;
                        }
                        Operation::Seek(_) => {
                            // Ignore the seek
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // ready!(crate::trace::trace_leaf(cx));
        let inner = self.inner.get_mut();
        inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        // ready!(crate::trace::trace_leaf(cx));
        self.poll_flush(cx)
    }
}

impl From<StdFile> for File {
    fn from(std: StdFile) -> Self {
        Self::from_std(std)
    }
}

impl fmt::Debug for File {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("tokio::fs::File")
            .field("std", &self.std)
            .finish()
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for File {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.std.as_raw_fd()
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsFd for File {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        unsafe {
            std::os::unix::io::BorrowedFd::borrow_raw(std::os::unix::io::AsRawFd::as_raw_fd(self))
        }
    }
}

#[cfg(unix)]
impl std::os::unix::io::FromRawFd for File {
    unsafe fn from_raw_fd(fd: std::os::unix::io::RawFd) -> Self {
        StdFile::from_raw_fd(fd).into()
    }
}

// cfg_windows! {
//     use crate::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle, AsHandle, BorrowedHandle};
//
//     impl AsRawHandle for File {
//         fn as_raw_handle(&self) -> RawHandle {
//             self.std.as_raw_handle()
//         }
//     }
//
//     impl AsHandle for File {
//         fn as_handle(&self) -> BorrowedHandle<'_> {
//             unsafe {
//                 BorrowedHandle::borrow_raw(
//                     AsRawHandle::as_raw_handle(self),
//                 )
//             }
//         }
//     }
//
//     impl FromRawHandle for File {
//         unsafe fn from_raw_handle(handle: RawHandle) -> Self {
//             StdFile::from_raw_handle(handle).into()
//         }
//     }
// }

impl Inner {
    async fn complete_inflight(&mut self) {
        use crate::future::poll_fn;

        poll_fn(|cx| self.poll_complete_inflight(cx)).await;
    }

    fn poll_complete_inflight(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        // ready!(crate::trace::trace_leaf(cx));
        match self.poll_flush(cx) {
            Poll::Ready(Err(e)) => {
                self.last_write_err = Some(e.kind());
                Poll::Ready(())
            }
            Poll::Ready(Ok(())) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        if let Some(e) = self.last_write_err.take() {
            return Poll::Ready(Err(e.into()));
        }

        let (op, buf) = match self.state {
            State::Idle(_) => return Poll::Ready(Ok(())),
            State::Busy(ref mut rx) => ready!(Pin::new(rx).poll(cx))?,
        };

        // The buffer is not used here
        self.state = State::Idle(Some(buf));

        match op {
            Operation::Read(_) => Poll::Ready(Ok(())),
            Operation::Write(res) => Poll::Ready(res),
            Operation::Seek(_) => Poll::Ready(Ok(())),
        }
    }
}

// dir_builder
#[derive(Debug, Default)]
pub struct Dir_Builder {
    /// Indicates whether to create parent directories if they are missing.
    recursive: bool,

    /// Sets the Unix mode for newly created directories.
    #[cfg(unix)]
    pub(super) mode: Option<u32>,
}

impl Dir_Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.recursive = recursive;
        self
    }

    pub async fn create(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref().to_owned();
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(self.recursive);

        #[cfg(unix)]
        {
            if let Some(mode) = self.mode {
                std::os::unix::fs::DirBuilderExt::mode(&mut builder, mode);
            }
        }

        asyncify(move || builder.create(path)).await
    }
}

// feature! {
//     #![unix]
//
//     impl DirBuilder {
//         pub fn mode(&mut self, mode: u32) -> &mut Self {
//             self.mode = Some(mode);
//             self
//         }
//     }
// }

pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::create_dir_all(path)).await
}

pub async fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::create_dir(path)).await
}

pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64, std::io::Error> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    asyncify(|| std::fs::copy(from, to)).await
}

pub async fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::canonicalize(path)).await
}
