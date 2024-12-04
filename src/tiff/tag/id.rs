//! TIFF标签ID模块
//!
//! 本模块定义了TIFF和GeoTIFF标签的ID枚举。
//! 标签ID遵循TIFF 6.0规范和OGC GeoTIFF 1.1规范。
//! 参考文档: <https://docs.ogc.org/is/19-008r4/19-008r4.html#_geotiff_tags_for_coordinate_transformations>

use num_enum::{IntoPrimitive, TryFromPrimitive};

/// TIFF标签ID枚举
///
/// 包含了标准TIFF标签和GeoTIFF扩展标签的ID值
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, TryFromPrimitive, Eq, Hash)]
#[repr(u16)]
pub enum TagId {
    /// 子文件类型标识
    SubfileType = 0x00FE,
    /// 图像宽度(像素)
    ImageWidth = 0x0100,
    /// 图像高度(像素)
    ImageHeight = 0x0101,
    /// 每个样本的位数
    BitsPerSample = 0x0102,
    /// 压缩方式
    Compression = 0x0103,
    /// 颜色空间解释方式
    PhotometricInterpretation = 0x0106,
    /// 条带数据偏移量
    StripOffsets = 0x0111,
    /// 每个像素的样本数
    SamplesPerPixel = 0x0115,
    /// 每个条带的行数
    RowsPerStrip = 0x0116,
    /// 条带字节数
    StripByteCounts = 0x0117,
    /// 样本最小值
    MinSampleValue = 0x0118,
    /// 样本最大值
    MaxSampleValue = 0x0119,
    /// X方向分辨率
    XResolution = 0x011A,
    /// Y方向分辨率
    YResolution = 0x011B,
    /// 数据存储方式配置
    PlanarConfiguration = 0x011C,
    /// 分辨率单位
    ResolutionUnit = 0x0128,
    /// 预测器类型
    Predictor = 0x013D,
    /// 颜色映射表
    ColorMap = 0x0140,
    /// 瓦片宽度
    TileWidth = 0x0142,
    /// 瓦片长度
    TileLength = 0x0143,
    /// 瓦片数据偏移量
    TileOffsets = 0x0144,
    /// 瓦片字节数
    TileByteCounts = 0x0145,
    /// 额外样本信息
    ExtraSamples = 0x0152,
    /// 样本格式
    SampleFormat = 0x0153,
    /// JPEG表
    JPEGTables = 0x015B,
    /// YCbCr子采样
    YCbCrSubSampling = 0x0212,
    /// 参考黑白点
    ReferenceBlackWhite = 0x0214,

    // GeoTIFF标签
    /// 模型像素比例
    ModelPixelScale = 0x830E,
    /// 模型控制点
    ModelTiepoint = 0x8482,
    /// 模型变换矩阵
    ModelTransformation = 0x85D8,
    /// GeoTIFF键目录
    GeoKeyDirectory = 0x87AF,
    /// GeoTIFF双精度参数
    GeoDoubleParams = 0x87B0,
    /// GeoTIFF ASCII参数
    GeoAsciiParams = 0x87B1,

    // GDAL扩展标签
    /// GDAL元数据
    GDALMetadata = 0xA480,
    /// GDAL无数据值
    GDALNoData = 0xA481,
}
