//! Cloud Optimized GeoTIFF (COG) 编码模块
//!
//! 本模块提供了 COG 文件的编码和创建功能。COG 是一种专为云存储和网络传输优化的 GeoTIFF 格式。
//!
//! # 核心特性
//!
//! ## 输入数据支持
//! - 支持多种源数据格式(栅格、图像等)
//! - 自动数据类型转换
//!
//! ## 压缩方案
//! - LZW 无损压缩
//! - Deflate/ZIP 压缩
//! - 无压缩选项
//!
//! ## 多分辨率金字塔
//! - 自动构建金字塔层级
//! - 可选重采样算法
//! - 优化的分块结构
//!
//! ## 地理信息处理
//! - 支持主流坐标参考系统
//! - 内置坐标转换功能
//! - 地理元数据管理
//!
//! ## 参数配置
//! - 自定义分块尺寸
//! - 压缩参数优化
//! - 金字塔层级设置
//!
//! # 典型应用
//!
//! - 云端地理空间数据服务
//! - Web 地图瓦片服务
//! - 大型遥感数据管理
//!
//! # 示例
//!
//! ```no_run
//! use cloudtiff::encode::{CogEncoder, SupportedCompression};
//! use cloudtiff::raster::Raster;
//!
//! // 创建编码器
//! let encoder = CogEncoder::new(raster)
//!     .compression(SupportedCompression::Lzw)
//!     .tile_size(256, 256);
//!
//! // 编码并保存
//! encoder.encode("output.tif")?;
//! ```
use crate::cog::{Compression, Predictor};
use crate::geotags::{GeoKeyId, GeoKeyValue, GeoTags};
use crate::raster::{PlanarConfiguration, Raster, ResizeFilter};
use crate::tiff::{Endian, TagData, TagId, Tiff, TiffVariant};
use crate::Region;
use image::DynamicImage;
use std::io::{Seek, SeekFrom, Write};

pub mod error;

pub use error::{EncodeError, EncodeResult};

/// COG 编码器支持的压缩方式
///
/// # 变体说明
///
/// * `Lzw` - LZW 无损压缩，适用于大多数场景
/// * `Deflate` - Deflate/ZIP 压缩，提供较好的压缩比
/// * `Uncompressed` - 不压缩，适用于需要快速访问的场景
#[derive(Debug, Copy, Clone)]
pub enum SupportedCompression {
    /// LZW 无损压缩算法
    ///
    /// 提供较好的压缩效果和解压速度的平衡，是 TIFF 格式最常用的压缩方式之一。
    /// 特别适合于包含重复模式的图像数据。
    Lzw,

    /// Deflate/ZIP 压缩算法
    ///
    /// 使用 DEFLATE 算法进行压缩，通常可以获得比 LZW 更高的压缩比。
    /// 适用于需要最大程度减小文件大小的场景。
    Deflate,

    /// 不进行压缩
    ///
    /// 直接存储原始数据，没有压缩开销。
    /// 适用于：
    /// - 需要最快访问速度的场景
    /// - 数据本身已经压缩的情况（如 JPEG 图像）
    /// - 调试和开发过程
    Uncompressed,
}

/// COG 文件编码器
///
/// 用于配置和执行 COG 文件的编码过程。
///
/// # 字段说明
///
/// * `raster` - 源栅格数据
/// * `projection` - 地理空间投影信息 (EPSG代码, 边界范围)
/// * `endian` - 字节序
/// * `variant` - TIFF 变体类型（标准/BigTIFF）
/// * `compression` - 压缩方式
/// * `tile_dimensions` - 分块尺寸
/// * `filter` - 金字塔层级重采样滤波器
#[derive(Debug)]
pub struct Encoder {
    /// 源栅格数据
    ///
    /// 包含原始图像数据和元数据信息
    raster: Raster,

    /// 地理空间投影信息
    ///
    /// 包含 EPSG 代码和地理范围:
    /// - `u16`: EPSG 坐标系统代码
    /// - `Region<f64>`: 地理边界范围
    projection: Option<(u16, Region<f64>)>,

    /// 字节序
    ///
    /// 指定数据的字节顺序(大端/小端)
    endian: Endian,

    /// TIFF 变体类型
    ///
    /// 指定使用标准 TIFF 还是 BigTIFF 格式
    variant: TiffVariant,

    /// 压缩方式
    ///
    /// 指定图像数据的压缩算法
    compression: SupportedCompression,

    /// 分块尺寸
    ///
    /// 指定图像分块的宽度和高度(像素)
    tile_dimensions: (u16, u16),

    /// 重采样滤波器
    ///
    /// 用于生成金字塔层级时的图像重采样方法
    filter: ResizeFilter,
    // TODO tiff tags
}

impl Encoder {
    /// 从图像创建编码器
    ///
    /// # 参数
    ///
    /// * `img` - 源图像数据
    ///
    /// # 返回值
    ///
    /// 返回配置了默认参数的编码器：
    /// - 小端字节序
    /// - BigTIFF 格式
    /// - LZW 压缩
    /// - 512x512 分块大小
    /// - 最近邻重采样
    #[cfg(feature = "image")]
    pub fn from_image(img: &DynamicImage) -> EncodeResult<Self> {
        Ok(Self {
            raster: Raster::from_image(img)?,
            projection: None,
            endian: Endian::Little,
            variant: TiffVariant::Big,
            compression: SupportedCompression::Lzw,
            tile_dimensions: (512, 512),
            filter: ResizeFilter::Nearest,
        })
    }

    /// 设置地理空间投影信息
    ///
    /// # 参数
    ///
    /// * `epsg` - EPSG 坐标系统代码
    /// * `region` - 地理范围
    pub fn with_projection(mut self, epsg: u16, region: Region<f64>) -> Self {
        self.projection = Some((epsg, region));
        self
    }

    /// 设置分块大小
    ///
    /// # 参数
    ///
    /// * `size` - 分块边长（像素）
    pub fn with_tile_size(mut self, size: u16) -> Self {
        self.tile_dimensions = (size, size);
        self
    }

    /// 设置字节序
    ///
    /// # 参数
    ///
    /// * `big` - true 表示大端，false 表示小端
    pub fn with_big_endian(mut self, big: bool) -> Self {
        self.endian = if big { Endian::Big } else { Endian::Little };
        self
    }

    /// 设置压缩方式
    ///
    /// # 参数
    ///
    /// * `compression` - 压缩算法
    pub fn with_compression(mut self, compression: SupportedCompression) -> Self {
        self.compression = compression;
        self
    }

    /// 设置是否使用 BigTIFF 格式
    ///
    /// # 参数
    ///
    /// * `big` - true 表示使用 BigTIFF，false 表示使用标准 TIFF
    pub fn with_big_tiff(mut self, big: bool) -> Self {
        self.variant = if big {
            TiffVariant::Big
        } else {
            TiffVariant::Normal
        };
        self
    }

    /// 设置重采样滤波器
    ///
    /// # 参数
    ///
    /// * `filter` - 用于生成金字塔层级的重采样算法
    pub fn with_filter(mut self, filter: ResizeFilter) -> Self {
        self.filter = filter;
        self
    }

    /// 执行编码过程
    ///
    /// 将配置好的数据编码为 COG 文件。过程包括：
    /// 1. 写入 TIFF 头部和 IFD
    /// 2. 生成并写入金字塔层级
    /// 3. 更新分块偏移量和大小
    ///
    /// # 参数
    ///
    /// * `writer` - 实现了 Write + Seek 的输出目标
    ///
    /// # 错误
    ///
    /// 可能返回的错误：
    /// - IO 错误
    /// - 编码错误
    /// - 不支持的投影
    pub fn encode<W: Write + Seek>(&self, writer: &mut W) -> EncodeResult<()> {
        // 获取基本参数
        let endian = self.endian;
        let full_dims = self.raster.dimensions;
        let bps = self.raster.bits_per_sample.clone();
        let interpretation = self.raster.interpretation;
        let planar = PlanarConfiguration::Chunky;
        let predictor = Predictor::No;

        // 将栅格数据的采样格式转换为 u16 向量
        let sample_format: Vec<u16> = self
            .raster
            .sample_format
            .iter()
            .map(|v| (*v).into())
            .collect();
        // 将额外采样数据转换为 u16 向量
        let extra_samples: Vec<u16> = self
            .raster
            .extra_samples
            .iter()
            .map(|v| (*v).into())
            .collect();

        // 设置压缩方式
        let compression = match self.compression {
            SupportedCompression::Lzw => Compression::Lzw,
            SupportedCompression::Deflate => Compression::DeflateAdobe,
            SupportedCompression::Uncompressed => Compression::Uncompressed,
        };

        // 获取投影参数
        // 从投影信息中提取参数
        // - epsg: 坐标系统代码
        // - tiepoint: 地理参考点 [I,J,K, X,Y,Z]
        // - pixel_scale: 像素分辨率 [ScaleX,ScaleY,ScaleZ]
        let (epsg, tiepoint, pixel_scale) = match self.projection {
            // 如果有投影信息,根据地理范围计算参数
            Some((epsg, region)) => (
                epsg,
                // 设置参考点为左上角
                [0.0, 0.0, 0.0, region.x.min, region.y.max, 0.0],
                // 计算每个像素对应的地理距离
                [
                    region.x.range().abs() / (full_dims.0 as f64), // X方向分辨率
                    region.y.range().abs() / (full_dims.1 as f64), // Y方向分辨率
                    0.0,                                           // Z方向分辨率(未使用)
                ],
            ),
            // 如果没有投影信息,使用默认值
            None => (4326, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
        };

        // 创建TIFF对象
        let mut tiff = Tiff::new(endian, self.variant);

        // 设置GeoTIFF标签
        // 获取第一个IFD(Image File Directory)
        let ifd0 = tiff.ifds.first_mut().unwrap(); // 安全,因为Tiff::new会创建ifd0

        // 根据参考点和像素比例创建GeoTags对象
        let mut geo = GeoTags::from_tiepoint_and_scale(tiepoint, pixel_scale);

        match epsg {
            // WGS84经纬度投影(EPSG:4326)
            4326 => {
                // 设置模型类型为地理坐标系
                geo.set_key(GeoKeyId::GTModelTypeGeoKey, GeoKeyValue::Short(vec![2]));
                // 设置栅格类型为像素表示地理坐标
                geo.set_key(GeoKeyId::GTRasterTypeGeoKey, GeoKeyValue::Short(vec![1]));
                // 设置地理坐标系统为WGS84
                geo.set_key(
                    GeoKeyId::GeographicTypeGeoKey,
                    GeoKeyValue::Short(vec![4326]),
                );
                // 设置地理坐标系统描述
                geo.set_key(
                    GeoKeyId::GeogCitationGeoKey,
                    GeoKeyValue::Ascii("WGS 84".into()),
                );
                // 设置角度单位为度
                geo.set_key(
                    GeoKeyId::GeogAngularUnitsGeoKey,
                    GeoKeyValue::Short(vec![9102]),
                );
                // 设置椭球体长半轴
                geo.set_key(
                    GeoKeyId::GeogSemiMajorAxisGeoKey,
                    GeoKeyValue::Double(vec![6378137.0]),
                );
                // 设置椭球体扁率倒数
                geo.set_key(
                    GeoKeyId::GeogInvFlatteningGeoKey,
                    GeoKeyValue::Double(vec![298.257223563]),
                );
            }
            // UTM北9区投影(EPSG:32609)
            32609 => {
                // 设置模型类型为投影坐标系
                geo.set_key(GeoKeyId::GTModelTypeGeoKey, GeoKeyValue::Short(vec![1]));
                // 设置栅格类型为像素表示投影坐标
                geo.set_key(GeoKeyId::GTRasterTypeGeoKey, GeoKeyValue::Short(vec![1]));
                // 设置投影坐标系统描述
                geo.set_key(
                    GeoKeyId::GTCitationGeoKey,
                    GeoKeyValue::Ascii("WGS 84 / UTM zone 9N".into()),
                );
                // 设置地理坐标系统描述
                geo.set_key(
                    GeoKeyId::GeogCitationGeoKey,
                    GeoKeyValue::Ascii("WGS 84".into()),
                );
                // 设置角度单位为度
                geo.set_key(
                    GeoKeyId::GeogAngularUnitsGeoKey,
                    GeoKeyValue::Short(vec![9102]),
                );
                // 设置投影坐标系统为UTM北9区
                geo.set_key(
                    GeoKeyId::ProjectedCSTypeGeoKey,
                    GeoKeyValue::Short(vec![32609]),
                );
                // 设置投影线性单位为米
                geo.set_key(
                    GeoKeyId::ProjLinearUnitsGeoKey,
                    GeoKeyValue::Short(vec![9001]),
                );
            }
            // 不支持的投影系统
            _ => {
                return Err(EncodeError::UnsupportedProjection(
                    epsg,
                    "Only EPSG 4326 supported at this time".into(),
                ))
            }
        }
        // 将地理标签信息添加到主IFD中
        geo.add_to_ifd(ifd0, endian);

        // 计算金字塔层级数
        let overview_levels = ((full_dims.0 as f32 / self.tile_dimensions.0 as f32)
            .log2()
            .max((full_dims.1 as f32 / self.tile_dimensions.1 as f32).log2())
            .ceil()) as usize;

        // 设置每个层级的IFD标签
        for i in 0..=overview_levels {
            // 计算当前层级的图像宽度
            let width = full_dims.0 / 2_u32.pow(i as u32);
            // 计算当前层级的图像高度
            let height = full_dims.1 / 2_u32.pow(i as u32);
            // 获取瓦片的宽度和高度
            let (tile_width, tile_height) = self.tile_dimensions;
            // 计算水平方向的瓦片数量
            let tile_cols = (width as f32 / tile_width as f32).ceil() as usize;
            // 计算垂直方向的瓦片数量
            let tile_rows = (height as f32 / tile_height as f32).ceil() as usize;
            // 计算总的瓦片数量
            let number_of_tiles = tile_cols * tile_rows;

            // 设置瓦片偏移量
            let tile_offsets = match self.variant {
                TiffVariant::Normal => TagData::Long(vec![0; number_of_tiles]),
                TiffVariant::Big => TagData::Long8(vec![0; number_of_tiles]),
            };

            // 获取或创建IFD
            let ifd = if i == 0 {
                tiff.ifds.first_mut().unwrap()
            } else {
                let ifd = tiff.add_ifd();
                ifd.set_tag(TagId::SubfileType, TagData::from_long(1), endian);
                ifd
            };

            // 设置图像宽度标签
            ifd.set_tag(TagId::ImageWidth, TagData::from_long(width), endian);
            // 设置图像高度标签
            ifd.set_tag(TagId::ImageHeight, TagData::from_long(height), endian);
            // 设置每个样本的位数标签
            ifd.set_tag(TagId::BitsPerSample, TagData::Short(bps.clone()), endian);
            // 设置压缩方式标签
            ifd.set_tag(
                TagId::Compression,
                TagData::from_short(compression.into()),
                endian,
            );
            // 设置光度解释标签
            ifd.set_tag(
                TagId::PhotometricInterpretation,
                TagData::from_short(interpretation.into()),
                endian,
            );
            // 设置每个像素的样本数标签
            ifd.set_tag(
                TagId::SamplesPerPixel,
                TagData::from_short(bps.len() as u16),
                endian,
            );

            // 设置数据组织方式
            // PlanarConfiguration 标签指定图像数据的存储方式:
            // - Chunky: 像素数据按 RGB RGB RGB 方式存储
            // - Planar: 像素数据按 RRR GGG BBB 方式存储
            ifd.set_tag(
                TagId::PlanarConfiguration,
                TagData::from_short(planar as u16),
                endian,
            );

            // 设置预测器类型
            // Predictor 标签指定压缩前是否使用预测器:
            // - No: 不使用预测器
            // - Horizontal: 使用水平差分预测
            ifd.set_tag(
                TagId::Predictor,
                TagData::from_short(predictor as u16),
                endian,
            );

            // 设置瓦片参数
            // TileWidth/TileLength: 瓦片的宽度和高度(像素)
            // TileOffsets: 每个瓦片数据在文件中的偏移量
            // TileByteCounts: 每个瓦片的字节数
            ifd.set_tag(TagId::TileWidth, TagData::from_short(tile_width), endian);
            ifd.set_tag(TagId::TileLength, TagData::from_short(tile_height), endian);
            ifd.set_tag(TagId::TileOffsets, tile_offsets, endian);
            ifd.set_tag(
                TagId::TileByteCounts,
                TagData::Long(vec![0; number_of_tiles]),
                endian,
            );

            // 设置采样格式
            // SampleFormat: 指定每个样本的数据类型(整型/浮点等)
            ifd.set_tag(
                TagId::SampleFormat,
                TagData::Short(sample_format.clone()),
                endian,
            );

            // ExtraSamples: 指定额外的样本类型(如 Alpha 通道)
            if extra_samples.len() > 0 {
                ifd.set_tag(
                    TagId::ExtraSamples,
                    TagData::Short(extra_samples.clone()),
                    endian,
                );
            }

            // 对标签进行排序
            ifd.0.sort_by(|a, b| a.code.cmp(&b.code));
        }

        // 编码TIFF头部和IFD
        let offsets = tiff.encode(writer)?;

        // 初始化存储瓦片偏移量和大小的向量
        // ifd_tile_offsets: 存储每个层级所有瓦片在文件中的偏移量
        // ifd_tile_bytes: 存储每个层级所有瓦片的字节大小
        let mut ifd_tile_offsets = vec![vec![]; overview_levels + 1];
        let mut ifd_tile_bytes = vec![vec![]; overview_levels + 1];

        // 获取瓦片的宽度和高度
        let (tile_width, tile_height) = self.tile_dimensions;

        // 用于存储上一层级的图像数据,用于生成下一层级的概览图
        let mut prev_overview: Option<Raster> = None;

        // 处理每个金字塔层级
        // 从最高分辨率(i=0)到最低分辨率(i=overview_levels)逐层处理
        for i in 0..=overview_levels {
            // 存储当前层级所有瓦片的偏移量和大小
            let mut tile_offsets = vec![]; // 瓦片在文件中的偏移量
            let mut tile_byte_counts = vec![]; // 瓦片的字节大小

            // 计算当前层级的图像尺寸
            // 每一层的尺寸是上一层的一半(2的幂次缩放)
            let width = full_dims.0 / 2_u32.pow(i as u32); // 当前层级的宽度
            let height = full_dims.1 / 2_u32.pow(i as u32); // 当前层级的高度

            // 计算当前层级需要的瓦片行列数
            // 向上取整以确保覆盖整个图像
            let tile_cols = (width as f32 / tile_width as f32).ceil() as u32; // 瓦片列数
            let tile_rows = (height as f32 / tile_height as f32).ceil() as u32; // 瓦片行数

            // 生成当前层级的图像
            // 如果是第一层(i=0),直接从原始图像重采样
            // 否则从上一层的概览图重采样生成
            let img = match prev_overview {
                Some(raster) => raster.resize(width, height, self.filter)?,
                None => self.raster.resize(width, height, self.filter)?,
            };

            // 按行列顺序处理每个瓦片
            for row in 0..tile_rows {
                for col in 0..tile_cols {
                    // 记录当前瓦片的起始位置
                    tile_offsets.push(writer.stream_position()?);

                    // 计算当前瓦片在图像中的区域
                    let region = Region::new(
                        col * tile_width as u32,        // 左边界
                        row * tile_height as u32,       // 上边界
                        (col + 1) * tile_width as u32,  // 右边界
                        (row + 1) * tile_height as u32, // 下边界
                    );

                    // 从图像中提取瓦片区域
                    let tile_raster = img.get_region(region)?;

                    // 压缩瓦片数据并写入文件
                    let tile_bytes = compression.encode(&tile_raster.buffer[..])?;
                    writer.write(&tile_bytes)?;

                    // 记录瓦片的字节大小
                    tile_byte_counts.push(tile_bytes.len() as u32);
                }
            }

            // 保存当前层级的图像用于生成下一层级
            prev_overview = Some(img);

            // 保存当前层级的瓦片偏移量和大小
            ifd_tile_offsets[i] = tile_offsets;
            ifd_tile_bytes[i] = tile_byte_counts;
        }

        // 更新瓦片偏移量和大小
        // 更新每个层级的瓦片偏移量和大小
        for i in 0..=overview_levels {
            // 写入瓦片偏移量
            if let Some(offset) = offsets[i].get(&TagId::TileOffsets.into()) {
                // 定位到偏移量位置
                writer.seek(SeekFrom::Start(*offset))?;

                // 根据TIFF变体类型选择编码方式
                match self.variant {
                    // 标准TIFF使用32位偏移量
                    TiffVariant::Normal => writer.write(
                        &endian.encode_all(
                            &ifd_tile_offsets[i]
                                .iter()
                                .map(|v| *v as u32)
                                .collect::<Vec<u32>>(),
                        ),
                    ),
                    // BigTIFF使用64位偏移量
                    TiffVariant::Big => writer.write(&endian.encode_all(&ifd_tile_offsets[i])),
                }?;
            }

            // 写入瓦片大小
            if let Some(offset) = offsets[i].get(&TagId::TileByteCounts.into()) {
                // 定位到大小位置
                writer.seek(SeekFrom::Start(*offset))?;
                // 写入编码后的瓦片大小数据
                writer.write(&endian.encode_all(&ifd_tile_bytes[i]))?;
            }
        }

        Ok(())
    }
}
