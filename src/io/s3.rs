//! S3 范围读取实现模块
//!
//! 本模块提供了通过 AWS S3 GetObject API 实现的异步范围读取功能。
//! 支持按需获取 S3 对象的指定字节范围。

#![cfg(feature = "s3")]

use super::AsyncReadRange;
use aws_sdk_s3::{self, operation::get_object::builders::GetObjectFluentBuilder, Client};
use futures::future::BoxFuture;
use std::fmt;
use std::io::{Error, ErrorKind, Result};

/// S3 范围读取器
///
/// 通过 AWS S3 GetObject API 实现对象的异步读取
pub struct S3Reader {
    /// GetObject 请求构建器
    request: GetObjectFluentBuilder,
}

impl S3Reader {
    /// 创建新的 S3 读取器
    ///
    /// # 参数
    /// * `client` - AWS S3 客户端
    /// * `bucket` - S3 存储桶名称
    /// * `key` - 对象键名
    pub fn new(client: Client, bucket: &str, key: &str) -> Self {
        let request = client.get_object().bucket(bucket).key(key);
        Self { request }
    }

    /// 从已有的请求构建器创建读取器
    ///
    /// # 参数
    /// * `request` - GetObject 请求构建器
    pub fn from_request_builder(request: GetObjectFluentBuilder) -> Self {
        Self { request }
    }
}

/// 实现 Debug trait 以支持调试输出
impl fmt::Debug for S3Reader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Reader")
            .field("bucket", &self.request.get_bucket().as_ref())
            .field("key", &self.request.get_key().as_ref())
            .finish()
    }
}

/// 实现异步范围读取特性
impl AsyncReadRange for S3Reader {
    fn read_range_async<'a>(
        &'a self,
        start: u64,
        buf: &'a mut [u8],
    ) -> BoxFuture<'a, Result<usize>> {
        let n = buf.len();
        // S3 Range 请求是包含性的,所以结束位置需要减1
        let end = start + n as u64 - 1;
        // 克隆请求构建器并添加 Range 头
        let request_builder = self.request.clone().range(format!("bytes={start}-{end}"));

        Box::pin(async move {
            // 发送 GetObject 请求
            let request = request_builder.send();
            let mut response = request
                .await
                .map_err(|e| Error::new(ErrorKind::NotConnected, format!("{e:?}")))?;

            // 从响应流中读取数据到缓冲区
            let mut pos = 0;
            while let Some(bytes) = response.body.try_next().await.map_err(|err| {
                Error::new(
                    ErrorKind::Interrupted,
                    format!("从 S3 下载流读取失败: {err:?}"),
                )
            })? {
                // 计算本次要复制的字节数
                let bytes_len = bytes.len();
                let bytes_top = bytes_len.min(n - pos);
                let buf_top = n.min(pos + bytes_len);
                // 将数据复制到目标缓冲区
                buf[pos..buf_top].copy_from_slice(&bytes[..bytes_top]);
                pos += bytes_len;
            }
            Ok(pos)
        })
    }
}
