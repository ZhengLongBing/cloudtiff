//! TIFF错误处理模块
//!
//! 本模块定义了TIFF文件处理过程中可能遇到的各种错误类型。
//! 主要包括文件格式错误、标签错误和IO错误等。

use std::fmt;
use std::io;

use super::TagId;

/// TIFF错误枚举
///
/// 表示在处理TIFF文件时可能出现的错误情况
#[derive(Debug)]
pub enum TiffError {
    /// TIFF文件魔数错误
    ///
    /// 当文件开头的魔数不是有效的TIFF标识时返回此错误
    BadMagicBytes,

    /// 缺少IFD0错误
    ///
    /// 当TIFF文件中没有找到第一个图像文件目录(IFD0)时返回此错误
    NoIfd0,

    /// IO读取错误
    ///
    /// 当发生底层IO操作错误时返回此错误
    ReadError(io::Error),

    /// 缺少必需标签错误
    ///
    /// 当TIFF文件中缺少必需的标签时返回此错误
    MissingTag(TagId),

    /// 标签数据错误
    ///
    /// 当标签的数据格式或内容不正确时返回此错误
    BadTag(TagId),
}

/// 从IO错误转换为TIFF错误
impl From<io::Error> for TiffError {
    fn from(e: io::Error) -> Self {
        TiffError::ReadError(e)
    }
}

/// 实现错误显示格式化
impl fmt::Display for TiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TiffError::BadMagicBytes => write!(f, "无效的TIFF文件魔数"),
            TiffError::NoIfd0 => write!(f, "未找到IFD0"),
            TiffError::ReadError(e) => write!(f, "IO读取错误: {}", e),
            TiffError::MissingTag(tag) => write!(f, "缺少必需的标签: {:?}", tag),
            TiffError::BadTag(tag) => write!(f, "标签数据错误: {:?}", tag),
        }
    }
}

/// 实现标准错误特征
impl std::error::Error for TiffError {}
