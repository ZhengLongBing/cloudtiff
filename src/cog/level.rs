//! COG 图像金字塔层级模块
//!
//! 本模块实现了 Cloud Optimized GeoTIFF (COG) 金字塔结构中各个分辨率层级的数据管理。
//! 每个层级包含了特定分辨率的图像数据及其相关元数据。
//!
//! # 主要功能
//!
//! * 图像数据管理
//!   - 维护不同分辨率层级的图像数据
//!   - 处理图像元数据和属性信息
//!   - 支持分块存储和访问
//!
//! * 数据压缩
//!   - 实现多种压缩算法(LZW、Deflate等)
//!   - 支持压缩预测器优化
//!   - 提供高效的压缩/解压功能
//!
//! * 空间索引
//!   - 基于分块的空间索引
//!   - 支持快速定位和访问
//!   - 优化的缓存管理
//!
//! # 使用场景
//!
//! - 网络地图服务
//! - 遥感影像处理
//! - 大规模地理数据管理
//! - 分布式GIS应用
use super::compression::{Compression, Predictor};
use super::CloudTiffError;
use crate::raster::{ExtraSamples, PhotometricInterpretation, Raster, SampleFormat};
use crate::tiff::{Endian, Ifd, TagId, TiffError};
use crate::{Region, UnitFloat};
use std::fmt::Display;

/// 表示 COG 金字塔中的一个分辨率层级
///
/// # 字段说明
///
/// * `overview` - 在金字塔中的层级索引，0 表示原始分辨率
/// * `dimensions` - 图像尺寸 (宽度, 高度)
/// * `tile_width` - 分块宽度
/// * `tile_height` - 分块高度
/// * `compression` - 压缩方式
/// * `predictor` - 压缩预测器
/// * `interpretation` - 像素值的解释方式
/// * `bits_per_sample` - 每个样本的位深度
/// * `sample_format` - 样本数据格式
/// * `extra_samples` - 额外样本信息
/// * `endian` - 字节序
/// * `offsets` - 分块数据的偏移量
/// * `byte_counts` - 分块数据的字节数
#[derive(Clone, Debug)]
pub struct Level {
    /// 在金字塔中的层级索引,0表示原始分辨率
    pub overview: Option<usize>,

    /// 图像尺寸 (宽度, 高度)
    pub dimensions: (u32, u32),

    /// 分块宽度
    pub tile_width: u32,

    /// 分块高度
    pub tile_height: u32,

    /// 压缩方式
    pub compression: Compression,

    /// 压缩预测器
    pub predictor: Predictor,

    /// 像素值的解释方式
    pub interpretation: PhotometricInterpretation,

    /// 每个样本的位深度
    pub bits_per_sample: Vec<u16>,

    /// 样本数据格式
    pub sample_format: Vec<SampleFormat>,

    /// 额外样本信息
    pub extra_samples: Vec<ExtraSamples>,

    /// 字节序
    pub endian: Endian,

    /// 分块数据的偏移量
    pub offsets: Vec<u64>,

    /// 分块数据的字节数
    pub byte_counts: Vec<usize>,
}

impl Level {
    /// 从 TIFF IFD 创建新的层级
    ///
    /// # 参数
    ///
    /// * `ifd` - TIFF 图像文件目录
    /// * `endian` - 字节序
    ///
    /// # 错误
    ///
    /// 如果必需的 TIFF 标签缺失或无效，将返回错误
    pub fn from_ifd(ifd: &Ifd, endian: Endian) -> Result<Self, CloudTiffError> {
        // 从IFD中获取必需的标签值
        // 图像尺寸
        let width = ifd.get_tag_value(TagId::ImageWidth)?;
        let height = ifd.get_tag_value(TagId::ImageHeight)?;

        // 分块尺寸
        let tile_width = ifd.get_tag_value(TagId::TileWidth)?;
        let tile_height = ifd.get_tag_value(TagId::TileLength)?;

        // 压缩方式和预测器
        let compression = ifd.get_tag_value::<u16>(TagId::Compression)?.into();
        let predictor = ifd
            .get_tag_value::<u16>(TagId::Predictor)
            .unwrap_or(1) // 默认值为1,表示无预测器
            .into();

        // 样本位深度
        let bits_per_sample = ifd.get_tag_values(TagId::BitsPerSample)?;

        // 样本格式,如果未指定则默认为无符号整数
        let sample_format = ifd
            .get_tag_values::<u16>(TagId::SampleFormat)
            .map(|v| {
                v.iter()
                    .map(|v| SampleFormat::from(*v))
                    .collect::<Vec<SampleFormat>>()
            })
            .unwrap_or_else(|_| vec![SampleFormat::Unsigned; bits_per_sample.len()]);

        // 额外样本信息,如果未指定则为空
        let extra_samples = ifd
            .get_tag_values::<u16>(TagId::ExtraSamples)
            .map(|v| {
                v.iter()
                    .map(|v| ExtraSamples::from(*v))
                    .collect::<Vec<ExtraSamples>>()
            })
            .unwrap_or_else(|_| vec![]);

        // 光度解释方式,如果未指定则为未知
        let interpretation = ifd
            .get_tag_value::<u16>(TagId::PhotometricInterpretation)
            .unwrap_or(PhotometricInterpretation::Unknown.into())
            .into();

        // 分块数据的位置和大小
        let offsets = ifd.get_tag_values(TagId::TileOffsets)?;
        let byte_counts = ifd.get_tag_values(TagId::TileByteCounts)?;

        // 验证分块数据的完整性
        if offsets.len() != byte_counts.len() {
            return Err(CloudTiffError::BadTiff(TiffError::BadTag(
                TagId::TileOffsets,
            )));
        }

        Ok(Self {
            overview: None,
            dimensions: (width, height),
            tile_width,
            tile_height,
            compression,
            predictor,
            interpretation,
            bits_per_sample,
            sample_format,
            extra_samples,
            endian,
            offsets,
            byte_counts,
        })
    }

    /// 计算图像总像素数（以百万为单位）
    pub fn megapixels(&self) -> f64 {
        (self.dimensions.0 as f64 * self.dimensions.1 as f64) / 1e6
    }

    /// 获取图像宽度（像素）
    pub fn width(&self) -> u32 {
        self.dimensions.0
    }

    /// 获取图像高度（像素）
    pub fn height(&self) -> u32 {
        self.dimensions.1
    }

    /// 获取指定图像区域内的分块索引列表
    ///
    /// # 参数
    ///
    /// * `crop` - 归一化的图像区域 (0.0-1.0)
    ///
    /// # 返回值
    ///
    /// 返回包含在指定区域内的所有分块索引
    pub fn tile_indices_within_image_crop(&self, crop: Region<UnitFloat>) -> Vec<usize> {
        // 将裁剪区域的左上角转换为分块坐标
        let (left, top) = self.tile_coord_from_image_coord(crop.x.min.into(), crop.y.min.into());
        // 将裁剪区域的右下角转换为分块坐标
        let (right, bottom) =
            self.tile_coord_from_image_coord(crop.x.max.into(), crop.y.max.into());

        // 获取图像的总列数和行数
        let col_count = self.col_count();
        let row_count = self.row_count();

        // 计算裁剪区域覆盖的分块范围
        // 向下取整并限制在有效范围内
        let col_min = left.floor().max(0.0) as usize;
        let col_max = right.ceil().min(col_count as f64) as usize;
        let row_min = top.floor().max(0.0) as usize;
        let row_max = bottom.ceil().min(row_count as f64) as usize;

        // 收集所有覆盖的分块索引
        let mut indices = vec![];
        for row in row_min..row_max {
            for col in col_min..col_max {
                // 将二维坐标转换为一维索引
                indices.push(row * col_count + col);
            }
        }
        indices
    }

    /// 将归一化图像坐标转换为分块索引和分块内坐标
    ///
    /// # 参数
    ///
    /// * `x` - 归一化的 X 坐标 (0.0-1.0)
    /// * `y` - 归一化的 Y 坐标 (0.0-1.0)
    ///
    /// # 返回值
    ///
    /// 返回元组 (分块索引, 分块内 X 坐标, 分块内 Y 坐标)
    ///
    /// # 错误
    ///
    /// 如果输入坐标超出范围，返回 ImageCoordOutOfRange 错误
    pub fn index_from_image_coords(
        &self,
        x: f64,
        y: f64,
    ) -> Result<(usize, f64, f64), CloudTiffError> {
        // 验证输入坐标是否在有效范围内(0.0-1.0)
        // TODO: 使用 UnitFloat 类型来确保值在有效范围内
        if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 {
            return Err(CloudTiffError::ImageCoordOutOfRange((x, y)));
        }

        // 计算分块坐标(列和行)
        let (col, row) = self.tile_coord_from_image_coord(x, y);

        // 计算分块索引和分块内偏移量
        // 分块索引 = 行号 * 每行分块数 + 列号
        let tile_index = row.floor() as usize * self.col_count() + col.floor() as usize;
        // 分块内 x 坐标 = (列坐标小数部分) * 分块宽度
        let tile_x = (col - col.floor()) * self.tile_width as f64;
        // 分块内 y 坐标 = (行坐标小数部分) * 分块高度
        let tile_y = (row - row.floor()) * self.tile_height as f64;

        Ok((tile_index, tile_x, tile_y))
    }

    /// 将归一化图像坐标转换为分块坐标
    ///
    /// # 参数
    ///
    /// * `x` - 归一化的 X 坐标 (0.0-1.0)
    /// * `y` - 归一化的 Y 坐标 (0.0-1.0)
    ///
    /// # 返回值
    ///
    /// 返回分块坐标 (列, 行)
    pub fn tile_coord_from_image_coord(&self, x: f64, y: f64) -> (f64, f64) {
        let col: f64 = x * self.width() as f64 / self.tile_width as f64;
        let row: f64 = y * self.height() as f64 / self.tile_height as f64;
        (col, row)
    }

    /// 获取指定分块的字节范围
    ///
    /// # 参数
    ///
    /// * `index` - 分块索引
    ///
    /// # 返回值
    ///
    /// 返回字节范围 (起始偏移量, 结束偏移量)
    ///
    /// # 错误
    ///
    /// 如果分块索引超出范围，返回 TileIndexOutOfRange 错误
    pub fn tile_byte_range(&self, index: usize) -> Result<(u64, u64), CloudTiffError> {
        // 验证分块索引是否在有效范围内
        // 取 offsets 和 byte_counts 数组长度的较小值作为最大有效索引
        let max_valid_index = self.offsets.len().min(self.byte_counts.len()) - 1;
        if index > max_valid_index {
            return Err(CloudTiffError::TileIndexOutOfRange((
                index,
                max_valid_index,
            )));
        }

        // 查找分块的字节范围
        // offset: 分块数据的起始偏移量
        // byte_count: 分块数据的字节数
        let offset = self.offsets[index];
        let byte_count = self.byte_counts[index];

        // 返回分块数据的字节范围 (起始偏移量, 结束偏移量)
        Ok((offset, offset + byte_count as u64))
    }

    /// 从字节数据中提取分块图像
    ///
    /// # 参数
    ///
    /// * `bytes` - 压缩的分块数据
    ///
    /// # 返回值
    ///
    /// 返回解压缩和处理后的栅格数据
    ///
    /// # 错误
    ///
    /// 可能返回解压缩错误或数据格式错误
    pub fn extract_tile_from_bytes(&self, bytes: &[u8]) -> Result<Raster, CloudTiffError> {
        // 1. 解压缩分块数据
        let mut buffer = self.compression.decode(bytes)?;

        // TODO: 处理字节序

        // 2. 应用预测器
        // 获取位深度(暂时只使用第一个采样的位深度)
        // TODO: 考虑不同采样可能有不同位深度的情况
        let bit_depth = self.bits_per_sample[0] as usize;

        // 对解压后的数据应用预测器
        self.predictor.predict(
            buffer.as_mut_slice(),
            self.tile_width as usize,
            bit_depth,
            self.bits_per_sample.len(),
        )?;

        // 3. 栅格化处理
        // 将处理后的数据转换为栅格格式
        // 参数包括:
        // - 分块尺寸
        // - 像素数据
        // - 每个采样的位深度
        // - 颜色解释方式
        // - 采样格式
        // - 额外采样信息
        // - 字节序(TODO: 应该在解压时处理)
        Ok(Raster::new(
            (self.tile_width, self.tile_height),
            buffer,
            self.bits_per_sample.clone(),
            self.interpretation,
            self.sample_format.clone(),
            self.extra_samples.clone(),
            self.endian,
        )?)
    }

    /// 获取指定分块的归一化边界
    ///
    /// # 参数
    ///
    /// * `index` - 分块索引
    ///
    /// # 返回值
    ///
    /// 返回分块边界 (左, 上, 右, 下)，值范围 0.0-1.0
    pub fn tile_bounds(&self, index: &usize) -> (f64, f64, f64, f64) {
        // 计算分块的行列位置
        let col_count = self.col_count();
        let row = (index / col_count) as f64; // 行号
        let col = (index % col_count) as f64; // 列号

        // 计算分块的归一化边界坐标
        // 左边界 = 列号 * 分块宽度 / 总宽度
        let left = (col * self.tile_width as f64) / self.dimensions.0 as f64;
        // 上边界 = 行号 * 分块高度 / 总高度
        let top = (row * self.tile_height as f64) / self.dimensions.1 as f64;
        // 右边界 = (列号+1) * 分块宽度 / 总宽度
        let right = ((col + 1.0) * self.tile_width as f64) / self.dimensions.0 as f64;
        // 下边界 = (行号+1) * 分块高度 / 总高度
        let bottom = ((row + 1.0) * self.tile_height as f64) / self.dimensions.1 as f64;

        // 返回归一化的边界坐标
        (left, top, right, bottom)
    }

    /// 获取水平方向的分块数量
    pub fn col_count(&self) -> usize {
        (self.width() as f64 / self.tile_width as f64).ceil() as usize
    }

    /// 获取垂直方向的分块数量
    pub fn row_count(&self) -> usize {
        (self.height() as f64 / self.tile_height as f64).ceil() as usize
    }
}

/// 实现 Display trait，用于格式化输出层级信息
impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Level({}x{}, {} tiles, {:?} Compression, {:?} Predictor)",
            self.dimensions.0,
            self.dimensions.1,
            self.offsets.len(),
            self.compression,
            self.predictor
        )
    }
}
