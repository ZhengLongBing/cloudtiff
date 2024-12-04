//! GeoTIFF 地理空间标签模块
//!
//! 本模块实现了 GeoTIFF 规范中的地理空间标签处理功能，用于管理和操作地理空间元数据。
//! 基于 OGC GeoTIFF 1.1 标准实现。
//!
//! # 主要功能
//!
//! - 地理空间标签的解析和序列化 - 支持读写 GeoTIFF 标签数据
//! - 坐标转换模型的管理 - 提供变换矩阵和比例尺两种坐标转换模型
//! - GeoKey 目录的处理 - 管理 GeoTIFF 键值对元数据
//! - 地理空间参数的存取 - 访问投影、基准面等地理参数
//!
//! # 参考标准
//!
//! - [OGC GeoTIFF 1.1 规范](https://docs.ogc.org/is/19-008r4/19-008r4.html)
//! - [坐标转换标签](https://docs.ogc.org/is/19-008r4/19-008r4.html#_geotiff_tags_for_coordinate_transformations)
//! - [GeoKey 目录标准](https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_geokeydirectorytag)

use crate::tiff::{Endian, Ifd, Tag, TagData, TagId};
use keys::GeoKey;
use num_traits::NumCast;
use std::fmt::Display;

mod error;
mod id;
mod keys;
mod value;

pub use error::GeoTiffError;
pub use id::GeoKeyId;
pub use keys::GeoKeyDirectory;
pub use value::GeoKeyValue;

/// GeoTIFF 地理空间标签集合
///
/// 包含完整的地理空间元数据信息，包括：
/// - GeoKey 目录
/// - 坐标转换模型
///
/// # 示例
///
/// ```no_run
/// use cloudtiff::geotags::GeoTags;
///
/// // 创建带有比例尺和参考点的标签
/// let tags = GeoTags::from_tiepoint_and_scale(
///     [0.0, 0.0, 0.0, -180.0, 90.0, 0.0],  // 参考点
///     [0.1, 0.1, 0.0]                       // 像素比例
/// );
/// ```
#[derive(Clone, Debug)]
pub struct GeoTags {
    /// GeoKey 目录，存储地理空间键值对
    pub directory: GeoKeyDirectory,
    /// 坐标转换模型
    pub model: GeoModel,
}

/// 地理空间坐标转换模型
///
/// GeoTIFF 支持两种坐标转换模型：
/// - 变换矩阵模型：使用 4x4 矩阵进行坐标转换
/// - 比例尺模型：使用参考点和像素比例进行转换
#[derive(Clone, Debug)]
pub enum GeoModel {
    /// 使用变换矩阵的模型
    Transformed(GeoModelTransformed),
    /// 使用比例尺的模型
    Scaled(GeoModelScaled),
}

/// 变换矩阵模型
///
/// 使用 4x4 矩阵进行坐标转换，可选包含参考点。
#[derive(Clone, Debug)]
pub struct GeoModelTransformed {
    /// 4x4 变换矩阵，按行优先顺序存储
    pub transformation: [f64; 16],
    /// 可选的参考点 [I,J,K, X,Y,Z]
    pub tiepoint: Option<[f64; 6]>,
}

/// 比例尺模型
///
/// 使用参考点和像素比例进行坐标转换。
#[derive(Clone, Debug)]
pub struct GeoModelScaled {
    /// 像素比例 [ScaleX, ScaleY, ScaleZ]
    pub pixel_scale: [f64; 3],
    /// 参考点 [I,J,K, X,Y,Z]
    pub tiepoint: [f64; 6],
}

impl Display for GeoTags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "GeoTIFF Tags:")?;
        match &self.model {
            GeoModel::Transformed(model) => {
                writeln!(f, "  Tiepoint: {:?}", model.tiepoint)?;
                writeln!(f, "  Transformation: {:?}", model.transformation)?;
            }
            GeoModel::Scaled(model) => {
                writeln!(f, "  Tiepoint: {:?}", model.tiepoint)?;
                writeln!(f, "  Pixel Scale: {:?}", model.pixel_scale)?;
            }
        }
        write!(
            f,
            "  Directory: {{version: {}, revision: {}.{}}}",
            self.directory.version, self.directory.revision.0, self.directory.revision.1,
        )?;
        if self.directory.keys.len() > 0 {
            write!(f, "\n  Keys:")?;
            for key in self.directory.keys.iter() {
                write!(f, "\n    {key}")?;
            }
        }
        Ok(())
    }
}

impl GeoTags {
    /// 从参考点和像素比例创建标签
    ///
    /// # 参数
    ///
    /// * `tiepoint` - 参考点坐标 [I,J,K, X,Y,Z]
    /// * `pixel_scale` - 像素比例 [ScaleX, ScaleY, ScaleZ]
    pub fn from_tiepoint_and_scale(tiepoint: [f64; 6], pixel_scale: [f64; 3]) -> Self {
        Self {
            model: GeoModel::Scaled(GeoModelScaled {
                tiepoint,
                pixel_scale,
            }),
            directory: GeoKeyDirectory::new(),
        }
    }

    /// 从参考点和变换矩阵创建标签
    ///
    /// # 参数
    ///
    /// * `tiepoint` - 参考点坐标 [I,J,K, X,Y,Z]
    /// * `transformation` - 4x4 变换矩阵，按行优先顺序
    pub fn from_tiepoint_and_transformation(tiepoint: [f64; 6], transformation: [f64; 16]) -> Self {
        Self {
            model: GeoModel::Transformed(GeoModelTransformed {
                tiepoint: Some(tiepoint),
                transformation,
            }),
            directory: GeoKeyDirectory::new(),
        }
    }

    /// 从 TIFF IFD 解析地理空间标签
    ///
    /// # 参数
    ///
    /// * `ifd` - TIFF 图像文件目录
    ///
    /// # 错误
    ///
    /// - 如果缺少必需的标签
    /// - 如果标签数据格式错误
    /// - 如果 GeoKey 目录解析失败
    pub fn parse(ifd: &Ifd) -> Result<Self, GeoTiffError> {
        // 获取地理空间模型相关标签
        let tiepoint = get_tag_as_array(ifd, TagId::ModelTiepoint).ok();
        let pixel_scale = get_tag_as_array(ifd, TagId::ModelPixelScale).ok();
        let transformation = get_tag_as_array(ifd, TagId::ModelTransformation).ok();

        // 根据获取到的标签构建地理空间模型
        let model = match (tiepoint, pixel_scale, transformation) {
            // 如果有参考点和像素比例,构建缩放模型
            (Some(tiepoint), Some(pixel_scale), _) => GeoModel::Scaled(GeoModelScaled {
                tiepoint,
                pixel_scale,
            }),
            // 如果有变换矩阵,构建变换模型
            (tiepoint, _, Some(transformation)) => GeoModel::Transformed(GeoModelTransformed {
                tiepoint,
                transformation,
            }),
            // 如果缺少必要标签,返回错误
            _ => return Err(GeoTiffError::MissingTag(TagId::ModelPixelScale)),
        };

        // 解析GeoKey目录
        let directory = GeoKeyDirectory::parse(ifd)?;

        // 返回构建的GeoTags结构
        Ok(Self { model, directory })
    }

    /// 将地理空间标签写入 TIFF IFD
    ///
    /// # 参数
    ///
    /// * `ifd` - 目标 TIFF 图像文件目录
    /// * `endian` - 字节序
    pub fn add_to_ifd(&self, ifd: &mut Ifd, endian: Endian) {
        // 根据地理空间模型类型写入相应的标签
        match &self.model {
            // 对于变换矩阵模型
            GeoModel::Transformed(model) => {
                // 写入变换矩阵标签
                ifd.set_tag(
                    TagId::ModelTransformation,
                    TagData::Double(model.transformation.to_vec()),
                    endian,
                );
                // 如果存在参考点,写入参考点标签
                if let Some(tiepoint) = model.tiepoint {
                    ifd.set_tag(
                        TagId::ModelTiepoint,
                        TagData::Double(tiepoint.to_vec()),
                        endian,
                    );
                }
            }
            // 对于缩放模型
            GeoModel::Scaled(model) => {
                // 写入参考点标签
                ifd.set_tag(
                    TagId::ModelTiepoint,
                    TagData::Double(model.tiepoint.to_vec()),
                    endian,
                );
                // 写入像素比例标签
                ifd.set_tag(
                    TagId::ModelPixelScale,
                    TagData::Double(model.pixel_scale.to_vec()),
                    endian,
                );
            }
        }
        // 写入GeoKey目录
        self.directory.add_to_ifd(ifd, endian);
    }

    /// 设置 GeoKey 值
    ///
    /// # 参数
    ///
    /// * `id` - GeoKey 标识符
    /// * `value` - GeoKey 值
    ///
    /// 如果键已存在，更新其值；否则添加新键。
    pub fn set_key<I: Into<u16>>(&mut self, id: I, value: GeoKeyValue) {
        // 将输入的id转换为u16类型的键代码
        let code: u16 = id.into();

        // 使用键代码和值创建新的GeoKey
        let key = GeoKey { code, value };

        // 获取目录中的键列表的可变引用
        let keys = &mut self.directory.keys;

        // 查找是否已存在相同代码的键
        if let Some(index) = keys.iter().position(|key| key.code == code) {
            // 如果找到相同代码的键,更新其值
            keys[index] = key;
        } else {
            // 如果没有找到相同代码的键,添加新键
            keys.push(key);
        }
    }
}

/// 从 TIFF IFD 获取标签
///
/// 内部辅助函数，用于获取标签并处理错误。
fn get_geo_tag(ifd: &Ifd, id: TagId) -> Result<&Tag, GeoTiffError> {
    ifd.get_tag(id).ok().ok_or(GeoTiffError::MissingTag(id))
}

/// 从 TIFF IFD 获取标签值
///
/// 内部辅助函数，用于获取标签值并进行类型转换。
fn get_geo_tag_values<T: NumCast>(ifd: &Ifd, id: TagId) -> Result<Vec<T>, GeoTiffError> {
    get_geo_tag(ifd, id)?
        .values()
        .ok_or(GeoTiffError::BadTag(id))
}

/// 从 TIFF IFD 获取固定长度数组
///
/// 内部辅助函数，用于获取标签值并转换为固定长度数组。
fn get_tag_as_array<const N: usize, T: NumCast>(
    ifd: &Ifd,
    id: TagId,
) -> Result<[T; N], GeoTiffError> {
    get_geo_tag_values::<T>(ifd, id)?
        .try_into()
        .ok()
        .ok_or(GeoTiffError::BadTag(id))
}
