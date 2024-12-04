//! GeoTIFF GeoKey 标识符模块
//!
//! 本模块定义了 GeoTIFF 规范中的 GeoKey 标识符。这些键用于存储和访问地理空间元数据。
//! 实现基于 OGC GeoTIFF 1.1 标准 (OGC 19-008r4)。
//!
//! # 主要功能
//!
//! - 定义标准 GeoKey ID 枚举类型
//! - 提供 GeoKey ID 与数值的转换
//! - 支持所有标准的地理空间元数据类型
//!
//! # 参考标准
//!
//! - [GeoKey ID 和名称摘要](https://docs.ogc.org/is/19-008r4/19-008r4.html#_summary_of_geokey_ids_and_names)
//! - [坐标转换 GeoTIFF 标签](https://docs.ogc.org/is/19-008r4/19-008r4.html#_geotiff_tags_for_coordinate_transformations)
//!
//! # 示例
//!
//! ```
//! use cloudtiff::geotags::GeoKeyId;
//!
//! // 获取投影坐标系统类型键
//! let key = GeoKeyId::ProjectedCSTypeGeoKey;
//! assert_eq!(key as u16, 3072);
//! ```

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// GeoTIFF GeoKey 标识符
///
/// 定义了所有标准的 GeoKey ID，用于标识不同类型的地理空间元数据。
///
/// # 键值类别
///
/// ## 模型和栅格类型键 (1024-1026)
/// * `GTModelTypeGeoKey` (1024) - 整体坐标系统类型
/// * `GTRasterTypeGeoKey` (1025) - 栅格数据解释方式
/// * `GTCitationGeoKey` (1026) - 坐标系统的文本描述
///
/// ## 地理坐标系统键 (2048-2061)
/// * `GeographicTypeGeoKey` (2048) - 地理坐标系统代码
/// * `GeogCitationGeoKey` (2049) - 地理坐标系统的文本描述
/// * `GeogGeodeticDatumGeoKey` (2050) - 大地基准面
/// * `GeogPrimeMeridianGeoKey` (2051) - 本初子午线
/// * `GeogLinearUnitsGeoKey` (2052) - 线性单位
/// * `GeogAngularUnitsGeoKey` (2054) - 角度单位
/// * `GeogEllipsoidGeoKey` (2056) - 椭球体
/// * `GeogSemiMajorAxisGeoKey` (2057) - 椭球体长半轴
/// * `GeogInvFlatteningGeoKey` (2059) - 椭球体反扁率
///
/// ## 投影坐标系统键 (3072-3095)
/// * `ProjectedCSTypeGeoKey` (3072) - 投影坐标系统代码
/// * `PCSCitationGeoKey` (3073) - 投影坐标系统的文本描述
/// * `ProjectionGeoKey` (3074) - 投影方法
/// * `ProjLinearUnitsGeoKey` (3076) - 投影线性单位
/// * `ProjStdParallel1GeoKey` (3078) - 第一标准纬线
/// * `ProjNatOriginLongGeoKey` (3080) - 自然原点经度
/// * `ProjFalseEastingGeoKey` (3082) - 东伪偏移量
/// * `ProjFalseNorthingGeoKey` (3083) - 北伪偏移量
///
/// ## 垂直坐标系统键 (4096-4099)
/// * `VerticalCSTypeGeoKey` (4096) - 垂直坐标系统类型
/// * `VerticalCitationGeoKey` (4097) - 垂直坐标系统的文本描述
/// * `VerticalDatumGeoKey` (4098) - 垂直基准面
/// * `VerticalUnitsGeoKey` (4099) - 垂直单位
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum GeoKeyId {
    /// 整体坐标系统类型
    /// 值：1 = 投影坐标系, 2 = 地理坐标系
    GTModelTypeGeoKey = 1024,

    /// 栅格数据的空间解释方式
    /// 值：1 = RasterPixelIsArea, 2 = RasterPixelIsPoint
    GTRasterTypeGeoKey = 1025,

    /// 坐标系统的文本描述
    GTCitationGeoKey = 1026,

    /// 地理坐标系统代码（如 EPSG 代码）
    GeographicTypeGeoKey = 2048,

    /// 地理坐标系统的文本描述
    GeogCitationGeoKey = 2049,

    /// 大地基准面代码
    GeogGeodeticDatumGeoKey = 2050,

    /// 本初子午线代码
    GeogPrimeMeridianGeoKey = 2051,

    /// 地理坐标系统的线性单位代码
    GeogLinearUnitsGeoKey = 2052,

    /// 线性单位的比例因子
    GeogLinearUnitSizeGeoKey = 2053,

    /// 角度单位代码（如度、弧度）
    GeogAngularUnitsGeoKey = 2054,

    /// 角度单位的比例因子
    GeogAngularUnitSizeGeoKey = 2055,

    /// 椭球体代码
    GeogEllipsoidGeoKey = 2056,

    /// 椭球体长半轴长度
    GeogSemiMajorAxisGeoKey = 2057,

    /// 椭球体短半轴长度
    GeogSemiMinorAxisGeoKey = 2058,

    /// 椭球体反扁率
    GeogInvFlatteningGeoKey = 2059,

    /// 方位角单位代码
    GeogAzimuthUnitsGeoKey = 2060,

    /// 本初子午线的经度
    GeogPrimeMeridianLongGeoKey = 2061,

    /// 投影坐标系统代码（如 EPSG 代码）
    ProjectedCSTypeGeoKey = 3072,

    /// 投影坐标系统的文本描述
    PCSCitationGeoKey = 3073,

    /// 投影方法代码
    ProjectionGeoKey = 3074,

    /// 坐标转换方法代码
    ProjCoordTransGeoKey = 3075,

    /// 投影的线性单位代码
    ProjLinearUnitsGeoKey = 3076,

    /// 投影线性单位的比例因子
    ProjLinearUnitSizeGeoKey = 3077,

    /// 第一标准纬线
    ProjStdParallel1GeoKey = 3078,

    /// 第二标准纬线
    ProjStdParallel2GeoKey = 3079,

    /// 自然原点经度
    ProjNatOriginLongGeoKey = 3080,

    /// 自然原点纬度
    ProjNatOriginLatGeoKey = 3081,

    /// 东伪偏移量
    ProjFalseEastingGeoKey = 3082,

    /// 北伪偏移量
    ProjFalseNorthingGeoKey = 3083,

    /// 假原点经度
    ProjFalseOriginLongGeoKey = 3084,

    /// 假原点纬度
    ProjFalseOriginLatGeoKey = 3085,

    /// 假原点东偏移量
    ProjFalseOriginEastingGeoKey = 3086,

    /// 假原点北偏移量
    ProjFalseOriginNorthingGeoKey = 3087,

    /// 中心点经度
    ProjCenterLongGeoKey = 3088,

    /// 中心点纬度
    ProjCenterLatGeoKey = 3089,

    /// 中心点东偏移量
    ProjCenterEastingGeoKey = 3090,

    /// 中心点北偏移量
    ProjCenterNorthingGeoKey = 3091,

    /// 自然原点的比例因子
    ProjScaleAtNatOriginGeoKey = 3092,

    /// 中心点的比例因子
    ProjScaleAtCenterGeoKey = 3093,

    /// 方位角
    ProjAzimuthAngleGeoKey = 3094,

    /// 直立极点经度
    ProjStraightVertPoleLongGeoKey = 3095,

    /// 垂直坐标系统类型代码
    VerticalCSTypeGeoKey = 4096,

    /// 垂直坐标系统的文本描述
    VerticalCitationGeoKey = 4097,

    /// 垂直基准面代码
    VerticalDatumGeoKey = 4098,

    /// 垂直单位代码
    VerticalUnitsGeoKey = 4099,
}
