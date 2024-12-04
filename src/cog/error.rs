//! Cloud Optimized GeoTIFF 错误处理模块
//!
//! 本模块提供了 CloudTiff 库的错误处理机制,包含以下主要功能:
//!
//! - 定义了统一的错误类型 [`CloudTiffError`]
//! - 实现了错误类型之间的转换
//! - 提供了错误处理的辅助工具
//!
//! # 错误分类
//!
//! ## 文件格式错误
//! - TIFF 文件结构错误
//! - GeoTIFF 标签解析错误
//!
//! ## 数据访问错误  
//! - 金字塔层级索引越界
//! - 图像分块索引无效
//! - 像素坐标超出范围
//!
//! ## IO 错误
//! - 文件读写错误
//! - 网络传输错误
//!
//! ## 数据处理错误
//! - 压缩数据解码失败
//! - 栅格化转换异常
//! - 坐标投影计算错误
//!
//! # 使用示例
//!
//! ```
//! use cloud_tiff::CloudTiffResult;
//!
//! fn process_image() -> CloudTiffResult<()> {
//!     // 处理过程中的错误会自动转换为 CloudTiffError
//!     Ok(())
//! }
//! ```
use super::compression::DecompressError;
use crate::geotags::GeoTiffError;
use crate::projection::ProjectionError;
use crate::raster::RasterError;
use crate::tiff::TiffError;
use std::fmt;
use std::io;
use std::sync::PoisonError;

/// CloudTiff 操作的通用结果类型
///
/// 这是一个类型别名，用于简化返回 Result<T, CloudTiffError> 的函数签名
pub type CloudTiffResult<T> = Result<T, CloudTiffError>;

/// CloudTiff 库中所有可能出现的错误类型
///
/// # 变体说明
///
/// ## 文件格式错误
/// * `BadTiff` - TIFF 文件格式错误
/// * `BadGeoTiff` - GeoTIFF 标签错误
///
/// ## 数据访问错误
/// * `TileLevelOutOfRange` - 请求的金字塔层级超出范围，包含 (请求的层级, 最大层级)
/// * `TileIndexOutOfRange` - 请求的分块索引超出范围，包含 (请求的索引, 最大索引)
/// * `ImageCoordOutOfRange` - 图像坐标超出有效范围，包含 (x, y)
///
/// ## IO 错误
/// * `ReadError` - 文件读取错误
/// * `ReadRangeError` - 范围读取错误，包含错误描述
///
/// ## 数据处理错误
/// * `DecompresionError` - 数据解压缩错误
/// * `RasterizationError` - 栅格化处理错误
/// * `ProjectionError` - 坐标投影转换错误
///
/// ## 其他错误
/// * `NoLevels` - COG 文件中没有有效的图像层级
/// * `RegionOutOfBounds` - 请求的区域超出边界，包含 (请求的区域, 有效区域)
/// * `MutexError` - 互斥锁错误，包含错误描述
/// * `NotSupported` - 不支持的操作，包含具体说明
/// * `BadPath` - 无效的文件路径
/// * `TODO` - 未实现的功能
/// * `AsyncJoinError` - 异步任务连接错误（仅在启用 async 特性时可用）
#[derive(Debug)]
pub enum CloudTiffError {
    /// TIFF 文件格式错误
    BadTiff(TiffError),
    /// GeoTIFF 标签错误
    BadGeoTiff(GeoTiffError),
    /// 请求的金字塔层级超出范围,包含(请求的层级,最大层级)
    TileLevelOutOfRange((usize, usize)),
    /// 请求的分块索引超出范围,包含(请求的索引,最大索引)
    TileIndexOutOfRange((usize, usize)),
    /// 图像坐标超出有效范围,包含(x,y)
    ImageCoordOutOfRange((f64, f64)),
    /// 文件读取错误
    ReadError(io::Error),
    /// 数据解压缩错误
    DecompresionError(DecompressError),
    /// 栅格化处理错误
    RasterizationError(RasterError),
    /// 坐标投影转换错误
    ProjectionError(ProjectionError),
    /// COG文件中没有有效的图像层级
    NoLevels,
    /// 请求的区域超出边界,包含(请求的区域,有效区域)
    RegionOutOfBounds(((f64, f64, f64, f64), (f64, f64, f64, f64))),
    /// 范围读取错误,包含错误描述
    ReadRangeError(String),
    /// 互斥锁错误,包含错误描述
    MutexError(String),
    /// 不支持的操作,包含具体说明
    NotSupported(String),
    /// 无效的文件路径
    BadPath(String),
    /// 未实现的功能
    TODO,
    /// 异步任务连接错误(仅在启用async特性时可用)
    #[cfg(feature = "async")]
    AsyncJoinError(tokio::task::JoinError),
}

/// 实现错误的显示格式化
///
/// 当前实现简单地使用 Debug 格式输出错误信息
impl fmt::Display for CloudTiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// 实现标准错误特征
impl std::error::Error for CloudTiffError {}

/// 从 TIFF 错误转换
///
/// 特殊处理了 TIFF 的 IO 错误，将其转换为 CloudTiffError::ReadError
impl From<TiffError> for CloudTiffError {
    fn from(e: TiffError) -> Self {
        match e {
            TiffError::ReadError(io_error) => CloudTiffError::ReadError(io_error),
            tiff_error => CloudTiffError::BadTiff(tiff_error),
        }
    }
}

/// 从 GeoTIFF 错误转换
impl From<GeoTiffError> for CloudTiffError {
    fn from(e: GeoTiffError) -> Self {
        CloudTiffError::BadGeoTiff(e)
    }
}

/// 从标准 IO 错误转换
impl From<io::Error> for CloudTiffError {
    fn from(e: io::Error) -> Self {
        CloudTiffError::ReadError(e)
    }
}

/// 从解压缩错误转换
impl From<DecompressError> for CloudTiffError {
    fn from(e: DecompressError) -> Self {
        CloudTiffError::DecompresionError(e)
    }
}

/// 从栅格化错误转换
impl From<RasterError> for CloudTiffError {
    fn from(e: RasterError) -> Self {
        CloudTiffError::RasterizationError(e)
    }
}

/// 从投影错误转换
impl From<ProjectionError> for CloudTiffError {
    fn from(e: ProjectionError) -> Self {
        CloudTiffError::ProjectionError(e)
    }
}

/// 从互斥锁毒化错误转换
impl<G> From<PoisonError<G>> for CloudTiffError {
    fn from(e: PoisonError<G>) -> Self {
        CloudTiffError::MutexError(format!("{e:?}"))
    }
}
