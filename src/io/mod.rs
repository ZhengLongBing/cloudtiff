//! I/O 特性模块
//!
//! 本模块提供了两个主要的 I/O 特性:
//! - `ReadRange`: 无状态的同步范围读取特性
//! - `AsyncReadRange`: 无状态的异步范围读取特性
//!
//! 这些特性是 std::io::{Read + Seek} 和 tokio::io::{AsyncRead + AsyncSeek} 的超集。
//! 主要区别在于 self 是不可变的,这使其成为 HTTP 字节范围请求等场景的强大抽象。

use std::io::{Error, ErrorKind, Result};
use std::io::{Read, Seek};
use std::sync::Mutex;

pub mod http;
pub mod s3;

/// 无状态的同步范围读取特性
///
/// 这个特性是 std::io::{Read + Seek} 的超集,主要区别是 self 是不可变的。
/// 这使其成为并发 I/O 操作的理想抽象。
///
/// # 必需方法
/// - `read_range`: 从指定偏移量读取字节
///
/// # 提供的方法
/// - `read_range_exact`: 精确读取指定长度的字节
/// - `read_range_to_vec`: 读取指定范围的字节到向量
pub trait ReadRange {
    /// 从指定偏移量读取字节
    ///
    /// # 参数
    /// * `start` - 起始字节偏移量
    /// * `buf` - 目标缓冲区
    ///
    /// # 返回
    /// 返回实际读取的字节数
    fn read_range(&self, start: u64, buf: &mut [u8]) -> Result<usize>;

    /// 精确读取指定长度的字节
    ///
    /// # 参数
    /// * `start` - 起始字节偏移量
    /// * `buf` - 目标缓冲区,长度决定了要读取的字节数
    ///
    /// # 错误
    /// 如果无法完全填充缓冲区则返回错误
    fn read_range_exact(&self, start: u64, buf: &mut [u8]) -> Result<()> {
        let n = buf.len();
        let bytes_read = self.read_range(start, buf)?;
        if bytes_read == n {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                format!("无法完全填充缓冲区: {bytes_read} < {n}"),
            ))
        }
    }

    /// 读取指定范围的字节到向量
    ///
    /// # 参数
    /// * `start` - 起始字节偏移量
    /// * `end` - 结束字节偏移量(不包含)
    ///
    /// # 返回
    /// 包含读取字节的向量
    fn read_range_to_vec(&self, start: u64, end: u64) -> Result<Vec<u8>> {
        let n = (end - start) as usize;
        let mut buf = vec![0; n];
        let _bytes_read = self.read_range_exact(start, &mut buf)?;
        Ok(buf)
    }
}

/// 为实现了 Read + Seek 的类型实现 ReadRange
impl<R: Read + Seek> ReadRange for Mutex<R> {
    fn read_range(&self, start: u64, buf: &mut [u8]) -> Result<usize> {
        let mut locked_self = self
            .lock()
            .map_err(|e| Error::new(ErrorKind::Other, format!("{e:?}")))?;
        locked_self.seek(std::io::SeekFrom::Start(start))?;
        locked_self.read(buf)
    }
}

#[cfg(feature = "async")]
pub use not_sync::*;

/// 异步 I/O 实现模块
#[cfg(feature = "async")]
mod not_sync {
    use super::*;
    use futures::future::BoxFuture;
    use futures::FutureExt;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};
    use tokio::sync::Mutex as TokioMutex;

    /// 无状态的异步范围读取特性
    ///
    /// 这个特性是 tokio::io::{AsyncRead + AsyncSeek} 的超集,
    /// 主要区别是 self 是不可变的。这使其成为并发 HTTP 范围请求等场景的理想抽象。
    pub trait AsyncReadRange: Send + Sync {
        /// 异步从指定偏移量读取字节
        ///
        /// # 参数
        /// * `start` - 起始字节偏移量
        /// * `buf` - 目标缓冲区
        ///
        /// # 返回
        /// 返回包含实际读取字节数的 Future
        fn read_range_async<'a>(
            &'a self,
            start: u64,
            buf: &'a mut [u8],
        ) -> BoxFuture<'a, Result<usize>>;

        /// 异步精确读取指定长度的字节
        ///
        /// # 参数
        /// * `start` - 起始字节偏移量
        /// * `buf` - 目标缓冲区,长度决定了要读取的字节数
        ///
        /// # 错误
        /// 如果无法完全填充缓冲区则返回错误
        fn read_range_exact_async<'a>(
            &'a self,
            start: u64,
            buf: &'a mut [u8],
        ) -> BoxFuture<'a, Result<()>> {
            let n = buf.len();
            async move {
                match self.read_range_async(start, buf).await {
                    Ok(bytes_read) if bytes_read == n => Ok(()),
                    Ok(bytes_read) => Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        format!("无法完全填充缓冲区: {bytes_read} < {n}"),
                    )),
                    Err(e) => Err(e),
                }
            }
            .boxed()
        }

        /// 异步读取指定范围的字节到向量
        ///
        /// # 参数
        /// * `start` - 起始字节偏移量
        /// * `end` - 结束字节偏移量(不包含)
        ///
        /// # 返回
        /// 包含读取字节的向量的 Future
        fn read_range_to_vec_async(&self, start: u64, end: u64) -> BoxFuture<Result<Vec<u8>>> {
            let n = (end - start) as usize;
            Box::pin(async move {
                let mut buf = vec![0; n];
                match self.read_range_async(start, &mut buf).await {
                    Ok(bytes_read) if bytes_read == n => Ok(buf),
                    Ok(bytes_read) => Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        format!("无法完全填充缓冲区: {bytes_read} < {n}"),
                    )),
                    Err(e) => Err(e),
                }
            })
        }
    }

    /// 为实现了 AsyncRead + AsyncSeek 的类型实现 AsyncReadRange
    impl<R: AsyncRead + AsyncSeek + Send + Sync + Unpin> AsyncReadRange for TokioMutex<R> {
        fn read_range_async<'a>(
            &'a self,
            start: u64,
            buf: &'a mut [u8],
        ) -> BoxFuture<'a, Result<usize>> {
            Box::pin(async move {
                let mut locked_self = self.lock().await;
                locked_self.seek(std::io::SeekFrom::Start(start)).await?;
                locked_self.read_exact(buf).await
            })
        }
    }
}
