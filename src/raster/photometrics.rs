//! 光度解释和样本格式相关的枚举定义
//!
//! 本模块定义了与图像数据解释相关的各种枚举类型,包括:
//! - PhotometricInterpretation: 定义图像数据如何被解释为颜色
//! - SampleFormat: 定义样本数据的格式类型
//! - PlanarConfiguration: 定义颜色分量的存储方式
//! - ExtraSamples: 定义额外样本(如alpha通道)的解释方式

use num_enum::{FromPrimitive, IntoPrimitive};

/// 光度解释方式
///
/// 定义了如何将图像数据解释为颜色值
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum PhotometricInterpretation {
    /// 白色对应数值0
    WhiteIsZero = 0,
    /// 黑色对应数值0
    BlackIsZero = 1,
    /// RGB彩色
    RGB = 2,
    /// RGB调色板索引
    RGBPalette = 3,
    /// 透明度遮罩
    TransparencyMask = 4,
    /// CMYK彩色
    CMYK = 5,
    /// YCbCr彩色空间
    YCbCr = 6,
    /// CIE L*a*b* 颜色空间
    CIELab = 8,
    /// ICC L*a*b* 颜色空间
    ICCLab = 9,
    /// ITU L*a*b* 颜色空间
    ITULab = 10,
    /// 彩色滤光片阵列(用于原始相机数据)
    ColorFilterArray = 32803,
    /// Pixar对数亮度
    PixarLogL = 32844,
    /// Pixar对数亮度和色度
    PixarLogLuv = 32845,
    /// 顺序彩色滤光片
    SequentialColorFilter = 32892,
    /// 线性原始数据
    LinearRaw = 34892,
    /// 深度图
    DepthMap = 51177,
    /// 语义分割遮罩
    SemanticMask = 52527,

    /// 未知的光度解释方式
    #[num_enum(default)]
    Unknown = 0xFFFF,
}

/// 样本格式
///
/// 定义了图像数据中每个样本的数据格式
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum SampleFormat {
    /// 无符号整数
    Unsigned = 1,
    /// 有符号整数
    Signed = 2,
    /// IEEE浮点数
    Float = 3,
    /// 未定义格式
    Undefined = 4,
    /// 复数(整数)
    ComplexInt = 5,
    /// 复数(浮点数)
    ComplexFloat = 6,

    /// 未知格式
    #[num_enum(default)]
    Unknown = 0xFFFF,
}

/// 平面配置
///
/// 定义了多个颜色分量的存储方式
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum PlanarConfiguration {
    /// 分量交错存储(RGBRGBRGB...)
    Chunky = 1,
    /// 分量分平面存储(RRR...GGG...BBB...)
    Planar = 2,

    /// 未知配置
    #[num_enum(default)]
    Unknown = 0xFFFF,
}

/// 额外样本类型
///
/// 定义了额外样本(通常是alpha通道)的解释方式
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum ExtraSamples {
    /// 未指定用途
    Unspecified = 0,
    /// 预乘alpha
    AssociatedAlpha = 1,
    /// 直接alpha
    UnassociatedAlpha = 2,

    /// 未知类型
    #[num_enum(default)]
    Unknown = 0xFFFF,
}
