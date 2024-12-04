//! Cloud Optimized GeoTIFF (COG) 的实现模块
//!
//! 本模块提供了读取和处理 Cloud Optimized GeoTIFF 文件的功能。COG 是一种优化的 TIFF 格式，
//! 专门用于云存储和网络传输场景。
//!
//! # 主要特点
//!
//! - **金字塔结构** - 包含多个分辨率层级的图像数据,支持快速预览和多尺度访问
//! - **内部分块** - 数据按块组织并独立压缩,支持高效的随机访问
//! - **地理空间信息** - 内置投影坐标系统和地理元数据
//! - **云优化** - 针对云存储和HTTP范围请求进行优化
//!
//! # 核心组件
//!
//! - [`CloudTiff`] - COG 文件的主要表示结构,提供文件读取和数据访问接口
//! - [`Level`] - 金字塔中的单个分辨率层级,管理图像数据和分块信息
//! - [`Compression`] - 支持的数据压缩方式,如 LZW、Deflate 等
//! - [`Predictor`] - 压缩预测器类型,用于提高压缩效率
//!
//! # 使用场景
//!
//! - 网络地图服务
//! - 遥感影像处理
//! - 大规模地理数据管理
//! - 云端 GIS 应用

use crate::geotags::GeoTags;
use crate::projection::Projection;
use crate::tiff::Tiff;
use crate::Region;
use std::fmt::Display;
use std::io::{BufReader, Read, Seek};

mod compression;
mod error;
mod level;

pub use compression::{Compression, DecompressError, Predictor};
pub use error::{CloudTiffError, CloudTiffResult};
pub use level::Level;

/// 表示一个 Cloud Optimized GeoTIFF 文件
///
/// CloudTiff 是对 COG 格式的高级抽象，它包含：
/// - 多分辨率层级的图像数据（金字塔结构）
/// - 地理空间参考信息
/// - 压缩和编码信息
///
/// # 示例
///
/// ```no_run
/// use cloud_tiff::CloudTiff;
/// use std::fs::File;
///
/// let mut file = File::open("example.tif").unwrap();
/// let cog = CloudTiff::open(&mut file).unwrap();
///
/// // 获取图像尺寸
/// let (width, height) = cog.full_dimensions();
///
/// // 获取地理边界
/// let bounds = cog.bounds_lat_lon_deg().unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct CloudTiff {
    /// 不同分辨率的图像层级,从高分辨率到低分辨率排序
    /// 第一个元素(index 0)是原始分辨率,后续元素是逐级降采样的概览图
    pub levels: Vec<Level>,

    /// 地理空间投影信息,包含坐标系统和变换参数
    pub projection: Projection,
}

impl CloudTiff {
    /// 从实现了 Read + Seek 的数据源打开 COG 文件
    ///
    /// # 参数
    ///
    /// * `source` - 实现了 Read + Seek trait 的数据源
    ///
    /// # 返回值
    ///
    /// 返回 Result<CloudTiff, CloudTiffError>
    ///
    /// # 错误
    ///
    /// 可能返回以下错误:
    /// - 文件格式错误
    /// - IO 错误
    /// - 地理标签解析错误
    pub fn open<R: Read + Seek>(source: &mut R) -> CloudTiffResult<Self> {
        // 创建缓冲读取器
        let stream = &mut BufReader::new(source);

        // 解析TIFF结构
        let tiff = Tiff::open(stream)?;

        // 解析地理标签
        let ifd0 = tiff.ifd0()?;
        let geo_tags = GeoTags::parse(ifd0)?;

        Self::from_tiff_and_geo(tiff, geo_tags)
    }

    /// 从已解析的 TIFF 结构和地理标签创建 CloudTiff
    ///
    /// # 参数
    ///
    /// * `tiff` - 解析后的 TIFF 结构
    /// * `geo` - 解析后的地理标签
    pub fn from_tiff_and_geo(tiff: Tiff, geo: GeoTags) -> CloudTiffResult<Self> {
        // 将 IFD 转换为 COG 层级
        //   注意:会跳过无效的 COG 层级
        //   TODO: 检查所有层级的形状是否一致
        let mut levels: Vec<Level> = tiff
            .ifds
            .iter()
            .filter_map(|ifd| Level::from_ifd(ifd, tiff.endian).ok())
            .collect();

        // 验证层级
        //   COG 层级应该已经按从大到小排序
        levels.sort_by(|a, b| (b.megapixels()).total_cmp(&a.megapixels()));
        for (i, level) in levels.iter_mut().enumerate() {
            level.overview = Some(i);
        }
        if levels.len() == 0 {
            return Err(CloudTiffError::NoLevels);
        }

        // 投影可以对任意层级进行地理参考
        let projection = Projection::from_geo_tags(&geo, levels[0].dimensions)?;

        Ok(Self { levels, projection })
    }

    /// 获取图像覆盖区域的经纬度边界
    ///
    /// 返回一个 Region 结构,包含最小/最大经纬度值
    pub fn bounds_lat_lon_deg(&self) -> CloudTiffResult<Region<f64>> {
        Ok(self.projection.bounds_lat_lon_deg()?)
    }

    /// 获取原始分辨率图像的尺寸
    ///
    /// 返回元组 (宽度, 高度),单位为像素
    pub fn full_dimensions(&self) -> (u32, u32) {
        self.levels[0].dimensions
    }

    /// 计算原始分辨率图像的总像素数(以百万为单位)
    pub fn full_megapixels(&self) -> f64 {
        self.levels[0].megapixels()
    }

    /// 计算图像的宽高比
    pub fn aspect_ratio(&self) -> f64 {
        let (w, h) = self.full_dimensions();
        w as f64 / h as f64
    }

    /// 获取金字塔中最大的层级索引
    pub fn max_level(&self) -> usize {
        let n = self.levels.len();
        assert!(n > 0, "CloudTIFF has no levels"); // Checked at initialization
        n - 1
    }

    /// 获取指定层级的图像数据
    ///
    /// # 参数
    ///
    /// * `level` - 层级索引,0 表示原始分辨率
    ///
    /// # 错误
    ///
    /// 如果层级索引超出范围,返回 TileLevelOutOfRange 错误
    pub fn get_level(&self, level: usize) -> CloudTiffResult<&Level> {
        self.levels
            .get(level)
            .ok_or(CloudTiffError::TileLevelOutOfRange((
                level,
                self.levels.len() - 1,
            )))
    }

    /// 获取每个层级的像素比例
    ///
    /// 返回一个包含所有层级像素比例的向量,每个元素是 (x比例, y比例) 元组
    pub fn pixel_scales(&self) -> Vec<(f64, f64)> {
        // 获取投影的缩放比例
        let scale = self.projection.scale;

        // 遍历每个层级,计算其像素比例
        self.levels
            .iter()
            .map(|level| {
                // 计算每个像素对应的实际距离
                // scale.0 和 scale.1 分别是 x 和 y 方向的总缩放比例
                // 除以像素数得到每个像素的比例
                (
                    scale.0 / level.dimensions.0 as f64, // x 方向像素比例
                    scale.1 / level.dimensions.0 as f64, // y 方向像素比例
                )
            })
            .collect()
    }

    /// 根据目标像素比例选择最合适的层级
    ///
    /// # 参数
    ///
    /// * `min_pixel_scale` - 目标最小像素比例
    ///
    /// # 返回值
    ///
    /// 返回像素比例小于目标值的最大层级
    pub fn level_at_pixel_scale(&self, min_pixel_scale: f64) -> CloudTiffResult<&Level> {
        // 获取所有层级的像素比例
        let level_scales = self.pixel_scales();

        // 从高层级向低层级遍历,找到第一个像素比例小于目标值的层级
        let level_index = level_scales
            .iter()
            .enumerate() // 添加索引
            .rev() // 从高层级向低层级遍历
            .find(|(_, (level_scale_x, level_scale_y))| {
                // 选择 x 和 y 方向较大的比例进行比较
                level_scale_x.max(*level_scale_y) < min_pixel_scale
            })
            .map(|(i, _)| i) // 提取层级索引
            .unwrap_or(0); // 如果没找到合适的层级,使用最低层级(0)

        // 返回对应层级
        self.get_level(level_index)
    }
}

impl Display for CloudTiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CloudTiff({} Levels)", self.levels.len())?;
        for level in self.levels.iter() {
            write!(f, "\n  {level}")?;
        }
        Ok(())
    }
}

pub fn disect<R: Read + Seek>(stream: &mut R) -> Result<(), CloudTiffError> {
    let tiff = Tiff::open(stream)?;
    println!("{tiff}");

    let geo = GeoTags::parse(tiff.ifd0()?)?;
    println!("{geo}");

    let cog = CloudTiff::from_tiff_and_geo(tiff, geo)?;
    println!("{cog}");
    println!("{:?}", cog.bounds_lat_lon_deg()?);

    Ok(())
}

#[cfg(feature = "async")]
mod not_sync {
    use {
        super::*,
        crate::AsyncReadRange,
        std::io::{Cursor, ErrorKind},
        tokio::io::{AsyncRead, AsyncReadExt},
    };
    impl CloudTiff {
        pub async fn open_from_async_range_reader<R: AsyncReadRange>(
            source: &R,
        ) -> CloudTiffResult<Self> {
            let fetch_size = 4096;
            let mut result = Err(CloudTiffError::TODO);
            let mut buffer = Vec::with_capacity(fetch_size);
            for _i in 0..10 {
                let mut bytes = vec![0; fetch_size];
                let start = buffer.len();
                // let end = start + bytes.len();
                let n = source.read_range_async(start as u64, &mut bytes).await?;
                buffer.extend_from_slice(&bytes[..n]);

                let mut cursor = Cursor::new(&buffer);
                result = Self::open(&mut cursor);
                if let Err(CloudTiffError::ReadError(e)) = &result {
                    if matches!(e.kind(), ErrorKind::UnexpectedEof) {
                        continue;
                    }
                }
                break;
            }
            result
        }

        pub async fn open_async<R: AsyncRead + Unpin>(source: &mut R) -> CloudTiffResult<Self> {
            let fetch_size = 4096;
            let mut result = Err(CloudTiffError::TODO);
            let mut buffer = Vec::with_capacity(fetch_size);
            for _i in 0..10 {
                let mut bytes = vec![0; fetch_size];
                let n = source.read(&mut bytes).await?;
                if n == 0 {
                    break;
                }
                buffer.extend_from_slice(&bytes[..n]);

                let mut cursor = Cursor::new(&buffer);
                result = Self::open(&mut cursor);
                if let Err(CloudTiffError::ReadError(e)) = &result {
                    if matches!(e.kind(), ErrorKind::UnexpectedEof) {
                        continue;
                    }
                }
                break;
            }
            result
        }
    }
}
