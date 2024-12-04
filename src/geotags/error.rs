//! GeoTIFF 标签处理错误模块
//!
//! 本模块定义了处理 GeoTIFF 地理空间标签时可能出现的错误类型。
//!
//! # 主要功能
//!
//! - 定义标签处理错误枚举类型
//! - 提供错误处理和转换方法
//! - 支持标签缺失和内容无效错误
//!
//! # 错误类型
//!
//! - `MissingTag` - 必需的标签缺失
//! - `BadTag` - 标签内容无效或格式错误
//!
//! # 示例
//!
//! ```
//! use cloudtiff::geotags::GeoTiffError;
//! use cloudtiff::tiff::TagId;
//!
//! // 处理缺失标签错误
//! let error = GeoTiffError::MissingTag(TagId::ModelTiepointTag);
//! ```
use crate::tiff::TagId;

/// GeoTIFF 标签处理过程中可能出现的错误
///
/// # 错误类型
///
/// ## 标签缺失错误
/// * `MissingTag` - 必需的 GeoTIFF 标签不存在
///   - 例如：缺少 ModelTiepointTag 或 ModelPixelScaleTag
///   - 影响：无法确定图像的地理参考信息
///
/// ## 标签内容错误
/// * `BadTag` - 标签存在但内容无效或格式错误
///   - 例如：数据类型不匹配、数值范围错误
///   - 影响：无法正确解析地理参考信息
///
/// # 示例
///
/// ```no_run
/// use crate::geotags::GeoTiffError;
/// use crate::tiff::TagId;
///
/// // 处理缺失的 ModelTiepointTag
/// let error = GeoTiffError::MissingTag(TagId::ModelTiepointTag);
///
/// // 处理无效的 ModelPixelScaleTag
/// let error = GeoTiffError::BadTag(TagId::ModelPixelScaleTag);
/// ```
#[derive(Debug)]
pub enum GeoTiffError {
    /// 必需的 GeoTIFF 标签缺失
    ///
    /// 当解析 GeoTIFF 文件时发现缺少必需的标签时返回此错误。
    /// 参数是缺失标签的 TagId。
    MissingTag(TagId),

    /// GeoTIFF 标签内容无效
    ///
    /// 当标签存在但其内容无法正确解析时返回此错误。
    /// 可能的原因包括：
    /// - 数据类型不匹配
    /// - 数值范围超出有效范围
    /// - 数组长度不符合要求
    ///
    /// 参数是有问题的标签的 TagId。
    BadTag(TagId),
}
