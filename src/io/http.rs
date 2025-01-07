//! HTTP 范围读取实现模块
//!
//! 本模块提供了通过 HTTP Range 请求实现的异步读取功能。
//! 支持按需获取远程文件的指定字节范围。

#![cfg(feature = "http")]

use super::AsyncReadRange;
use futures::future::BoxFuture;
use futures::FutureExt;
use reqwest::header::RANGE;
use reqwest::{Client, IntoUrl, Url};
use std::fmt;
use std::future::Future;
use std::io::{Error, ErrorKind, Result};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;

/// HTTP 范围读取器
///
/// 通过 HTTP Range 请求实现远程文件的异步读取
pub struct HttpReader {
    /// 远程文件的 URL
    url: Url,
    /// 当前读取位置
    position: u64,
    /// 当前正在进行的读取请求
    _read_request: Option<PendingRequest>,
}

/// 表示一个挂起的 HTTP 请求
///
/// 包装了一个异步的字节数组结果
type PendingRequest = Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Sync + Send>>;

impl fmt::Debug for HttpReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpReader")
            .field("url", &self.url)
            .field("position", &self.position)
            .finish()
    }
}

impl HttpReader {
    /// 创建新的 HTTP 读取器
    ///
    /// # 参数
    /// * `url` - 远程文件的 URL
    ///
    /// # 错误
    /// 如果 URL 解析失败则返回错误
    pub fn new<U: IntoUrl>(url: U) -> Result<Self> {
        Ok(Self {
            url: url
                .into_url()
                .map_err(|e| Error::new(ErrorKind::AddrNotAvailable, format!("{e:?}")))?,
            position: 0,
            _read_request: None,
        })
    }

    /// 获取或创建读取请求
    ///
    /// # 参数
    /// * `n` - 要读取的字节数
    ///
    /// # 返回
    /// 返回一个表示读取操作的 Future
    pub fn get_or_create_read_request(&mut self, n: usize) -> PendingRequest {
        match self._read_request.take() {
            Some(req) => req,
            None => {
                let start = self.position;
                // 注意: HTTP Range 是包含性的,所以需要减1
                let end = start + n as u64 - 1;

                // 创建新的 HTTP 请求
                let fut = Client::new()
                    .get(self.url.clone())
                    .header(RANGE, format!("bytes={start}-{end}"))
                    .send();

                // 包装请求结果处理
                Box::pin(fut.then(|result| async move {
                    match result {
                        Ok(response) => match response.bytes().await {
                            Ok(bytes) => Ok(bytes.to_vec()),
                            Err(e) => Err(Error::new(ErrorKind::InvalidData, format!("{e:?}"))),
                        },
                        Err(e) => Err(Error::new(ErrorKind::NotConnected, format!("{e:?}"))),
                    }
                }))
            }
        }
    }
}

impl AsyncReadRange for HttpReader {
    /// 异步读取指定范围的数据
    ///
    /// # 参数
    /// * `start` - 起始字节位置
    /// * `buf` - 目标缓冲区
    fn read_range_async<'a>(
        &'a self,
        start: u64,
        buf: &'a mut [u8],
    ) -> BoxFuture<'a, Result<usize>> {
        // 计算请求范围的结束位置
        let end = start + buf.len() as u64 - 1;

        // 构建带有Range头的HTTP请求
        let request_builder = Client::new()
            .get(self.url.clone())
            .header(RANGE, format!("bytes={start}-{end}"));

        Box::pin(async move {
            // 发送HTTP请求
            let request = request_builder.send();
            let response = request
                .await
                .map_err(|e| Error::new(ErrorKind::NotConnected, format!("{e:?}")))?;

            // 获取响应体字节数据
            let bytes = response
                .bytes()
                .await
                .map_err(|e| Error::new(ErrorKind::InvalidData, format!("{e:?}")))?;

            // 将响应数据复制到目标缓冲区
            let n = bytes.len();
            buf[..n].copy_from_slice(&bytes[..]);

            // 返回读取的字节数
            Ok(bytes.len())
        })
    }
}

impl AsyncRead for HttpReader {
    /// 实现异步读取接口
    ///
    /// # 参数
    /// * `cx` - 任务上下文
    /// * `buf` - 读取缓冲区
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // 获取或创建读取请求
        let mut fut = self.get_or_create_read_request(buf.remaining());

        match fut.poll_unpin(cx) {
            // 请求未完成,保存请求并返回 Pending
            Poll::Pending => {
                self._read_request = Some(fut);
                Poll::Pending
            }
            // 请求出错
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            // 请求成功完成
            Poll::Ready(Ok(bytes)) => {
                let n = bytes.len().max(buf.remaining());
                let target = buf.initialize_unfilled_to(n);
                target.copy_from_slice(&bytes[..]);
                buf.advance(n);
                self.position += n as u64;
                Poll::Ready(Ok(()))
            }
        }
    }
}
