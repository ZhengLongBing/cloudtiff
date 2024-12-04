//! 这是一个用于处理Cloud Optimized GeoTIFF(COG)格式的库
//!
//! COG是一种优化的TIFF格式,专门用于云存储和流式传输地理空间数据。
//! 主要特点包括:
//! - 支持分块存储和按需读取
//! - 内置金字塔层级结构
//! - 支持地理空间元数据和投影
//! - 兼容标准TIFF阅读器
//!
//! # 主要功能
//! - 读取和解析COG文件
//! - 支持HTTP和S3远程读取
//! - 地理空间投影转换
//! - 图像重采样和渲染
//! - 异步操作支持
//!
//! # 示例
//! ```rust
//! use cogrs::{CloudTiff, HttpReader};
//!
//! // 从HTTP URL打开COG文件
//! let reader = HttpReader::new("https://example.com/data.tif")?;
//! let cog = CloudTiff::open(&mut reader)?;
//!
//! // 获取元数据
//! println!("图像尺寸: {:?}", cog.full_dimensions());
//! println!("投影: {:?}", cog.projection);
//! ```

// 导出主要模块
pub mod cog; // COG文件格式处理
pub mod encode; // 编码相关功能
pub mod geotags; // 地理标签处理
pub mod io; // IO操作
pub mod projection; // 投影转换
pub mod raster; // 栅格数据处理
pub mod render; // 渲染功能
pub mod tiff; // TIFF格式处理

// 重新导出常用类型
pub use cog::{disect, CloudTiff, CloudTiffError};
pub use encode::{EncodeError, Encoder, SupportedCompression};
pub use proj4rs::Proj;
pub use projection::primatives::{Point2D, Region, UnitFloat};
pub use projection::Projection;
pub use raster::{Raster, ResizeFilter};
pub use render::tiles;

// IO相关导出
#[cfg(feature = "http")]
pub use io::http::HttpReader;
#[cfg(feature = "s3")]
pub use io::s3::S3Reader;
#[cfg(feature = "async")]
pub use io::AsyncReadRange;
pub use io::ReadRange;
