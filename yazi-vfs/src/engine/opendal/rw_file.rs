use std::{io, pin::Pin, task::{Context, Poll}};

use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};
use yazi_fs::file::File;
use yazi_shared::url::{AsUrl, Url, UrlBuf, UrlLike};

use super::read_only;

#[pin_project]
pub struct RwFile {
	#[pin]
	inner:  Compat<::opendal::FuturesAsyncReader>,
	reader: ::opendal::Reader,
	stat:   yazi_fs::stat::Stat,
	url:    UrlBuf,
}

impl AsUrl for RwFile {
	fn as_url(&self) -> Url<'_> { self.url.as_url() }
}

impl UrlLike for RwFile {}

impl RwFile {
	pub(super) async fn new(
		reader: ::opendal::Reader,
		stat: yazi_fs::stat::Stat,
		url: impl Into<UrlBuf>,
	) -> io::Result<Self> {
		Self::at(reader, stat, url.into(), 0).await
	}

	async fn at(
		reader: ::opendal::Reader,
		stat: yazi_fs::stat::Stat,
		url: UrlBuf,
		offset: u64,
	) -> io::Result<Self> {
		let inner = reader.clone().into_futures_async_read(offset.min(stat.len)..stat.len).await?;

		Ok(Self { inner: inner.compat(), reader, stat, url })
	}

	pub(crate) async fn cursor(&self, offset: u64) -> io::Result<Self> {
		Self::at(self.reader.clone(), self.stat, self.url.clone(), offset).await
	}

	pub(crate) fn metadata(&self) -> yazi_fs::stat::Stat { self.stat }

	pub(crate) fn into_file(self: Pin<Box<Self>>) -> File {
		File { url: self.url.clone(), stat: self.stat, extra: Default::default() }
	}
}

impl AsyncRead for RwFile {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		self.project().inner.poll_read(cx, buf)
	}
}

impl AsyncSeek for RwFile {
	fn start_seek(self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
		self.project().inner.start_seek(position)
	}

	fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
		self.project().inner.poll_complete(cx)
	}
}

impl AsyncWrite for RwFile {
	fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, _: &[u8]) -> Poll<io::Result<usize>> {
		Poll::Ready(Err(read_only()))
	}

	fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
		Poll::Ready(Ok(()))
	}

	fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
		Poll::Ready(Ok(()))
	}
}
