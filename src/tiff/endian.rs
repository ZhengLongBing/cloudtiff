//! 字节序处理模块
//!
//! 本模块提供了大端和小端字节序的编解码功能。
//! 主要用于处理TIFF文件中的数据读写和转换。

use eio::{FromBytes, ReadExt, ToBytes};
use num_traits::{cast::NumCast, ToPrimitive};
use std::io::{Read, Result, Write};
use std::mem;

/// 字节序枚举
///
/// 表示数据的字节序类型:
/// - Big: 大端字节序,高位字节在前
/// - Little: 小端字节序,低位字节在前
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Endian {
    /// 大端字节序
    Big,
    /// 小端字节序
    Little,
}

impl Endian {
    /// 从流中读取指定大小的数据并按字节序解码
    ///
    /// # 参数
    /// * `stream` - 实现了Read trait的输入流
    ///
    /// # 类型参数
    /// * `N` - 要读取的字节数
    /// * `T` - 目标数据类型
    pub fn read<const N: usize, T: FromBytes<N>>(&self, stream: &mut impl Read) -> Result<T> {
        let mut buf = [0u8; N];
        stream.read_exact(&mut buf)?;
        self.decode(buf)
    }

    /// 将字节数组按字节序解码为指定类型
    ///
    /// # 参数
    /// * `bytes` - 要解码的字节数组
    pub fn decode<const N: usize, T: FromBytes<N>>(&self, bytes: [u8; N]) -> Result<T> {
        match self {
            Endian::Big => bytes.as_slice().read_be(),
            Endian::Little => bytes.as_slice().read_le(),
        }
    }

    /// 将字节切片按字节序解码为指定类型的向量
    ///
    /// # 参数
    /// * `bytes` - 要解码的字节切片
    pub fn decode_all<const N: usize, T: FromBytes<N>>(&self, bytes: &[u8]) -> Option<Vec<T>> {
        bytes
            .chunks_exact(mem::size_of::<T>())
            .map(|chunk| {
                chunk
                    .try_into()
                    .ok()
                    .and_then(|arr| self.decode::<N, T>(arr).ok())
            })
            .collect()
    }

    /// 将字节数组解码并转换为基本数值类型
    ///
    /// # 参数
    /// * `bytes` - 要解码的字节数组
    ///
    /// # 类型参数
    /// * `A` - 中间类型
    /// * `T` - 目标数值类型
    pub fn decode_to_primative<const N: usize, A: FromBytes<N> + ToPrimitive, T: NumCast>(
        &self,
        bytes: [u8; N],
    ) -> Option<T> {
        self.decode::<N, A>(bytes).ok().and_then(|v| T::from(v))
    }

    /// 将字节切片解码并转换为基本数值类型的向量
    ///
    /// # 参数
    /// * `bytes` - 要解码的字节切片
    pub fn decode_all_to_primative<const N: usize, A: FromBytes<N> + ToPrimitive, T: NumCast>(
        &self,
        bytes: &[u8],
    ) -> Option<Vec<T>> {
        self.decode_all::<N, A>(bytes)?
            .into_iter()
            .map(|v| T::from(v))
            .collect()
    }

    /// 将值按字节序编码为字节数组
    ///
    /// # 参数
    /// * `value` - 要编码的值
    pub fn encode<const N: usize, T: ToBytes<N>>(&self, value: T) -> [u8; N] {
        match self {
            Endian::Big => value.to_be_bytes(),
            Endian::Little => value.to_le_bytes(),
        }
    }

    /// 将值切片按字节序编码为字节向量
    ///
    /// # 参数
    /// * `values` - 要编码的值切片
    pub fn encode_all<const N: usize, T: ToBytes<N> + Copy>(&self, values: &[T]) -> Vec<u8> {
        values.iter().flat_map(|v| self.encode(*v)).collect()
    }

    /// 将值按字节序写入输出流
    ///
    /// # 参数
    /// * `stream` - 实现了Write trait的输出流
    /// * `value` - 要写入的值
    pub fn write<const N: usize, T: ToBytes<N>>(
        &self,
        stream: &mut impl Write,
        value: T,
    ) -> Result<()> {
        stream.write_all(&self.encode(value))
    }
}
