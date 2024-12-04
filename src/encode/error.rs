//! COG 编码过程中的错误处理模块
//!
//! 本模块定义了在 COG (Cloud Optimized GeoTIFF) 文件编码过程中可能出现的各种错误类型。
//!
//! # 主要功能
//!
//! - 定义编码错误枚举类型
//! - 提供错误处理和转换方法
//! - 支持错误传播和上下文信息
//!
//! # 错误类型
//!
//! - IO 写入错误 - 文件写入和存储操作失败
//! - 栅格化处理错误 - 图像数据转换和处理异常
//! - 投影转换错误 - 不支持的坐标系统或转换失败
//! - 数据压缩错误 - 压缩和解压缩操作异常
//!
//! # 示例
//!
//! ```
//! use cloudtiff::encode::EncodeError;
//! use std::io;
//!
//! // 处理文件写入错误
//! let error = EncodeError::WriteError(io::Error::new(
//!     io::ErrorKind::Other,
//!     "写入失败"
//! ));
//! ```
use crate::cog::DecompressError;
use crate::raster::RasterError;
use std::fmt;
use std::io;

/// COG 编码操作的通用结果类型
///
/// 这是一个类型别名，用于简化返回 Result<T, EncodeError> 的函数签名。
/// 在编码过程的所有操作中统一使用此类型作为返回值。
pub type EncodeResult<T> = Result<T, EncodeError>;

/// COG 编码过程中可能出现的错误类型
///
/// # 变体说明
///
/// ## IO 错误
/// * `WriteError` - 文件写入错误，包含具体的 IO 错误信息
///
/// ## 数据处理错误
/// * `RasterizationError` - 栅格数据处理错误，如格式转换、重采样等操作失败
/// * `CompressionError` - 数据压缩或解压缩过程中的错误
///
/// ## 参数错误
/// * `UnsupportedProjection` - 不支持的投影类型，包含 EPSG 代码和错误说明
#[derive(Debug)]
pub enum EncodeError {
    /// 文件写入错误
    WriteError(io::Error),
    /// 栅格数据处理错误
    RasterizationError(RasterError),
    /// 不支持的投影类型，包含 (EPSG代码, 错误说明)
    UnsupportedProjection(u16, String),
    /// 数据压缩错误
    CompressionError(DecompressError),
}

/// 实现错误的显示格式化
///
/// 当前实现使用 Debug 格式输出错误信息，提供详细的错误上下文
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// 实现标准错误特征
///
/// 使 EncodeError 可以作为标准错误类型使用，支持错误传播和处理
impl std::error::Error for EncodeError {}

/// 从标准 IO 错误转换
///
/// 将标准库的 IO 错误转换为编码错误。这个实现允许在使用 ? 运算符时
/// 自动将 IO 错误转换为 EncodeError。
impl From<io::Error> for EncodeError {
    fn from(e: io::Error) -> Self {
        EncodeError::WriteError(e)
    }
}

/// 从栅格错误转换
///
/// 将栅格处理错误转换为编码错误。这个实现允许在使用 ? 运算符时
/// 自动将栅格处理错误转换为 EncodeError。
impl From<RasterError> for EncodeError {
    fn from(e: RasterError) -> Self {
        EncodeError::RasterizationError(e)
    }
}

/// 从压缩错误转换
///
/// 将压缩处理错误转换为编码错误。这个实现允许在使用 ? 运算符时
/// 自动将压缩错误转换为 EncodeError。
impl From<DecompressError> for EncodeError {
    fn from(e: DecompressError) -> Self {
        EncodeError::CompressionError(e)
    }
}
