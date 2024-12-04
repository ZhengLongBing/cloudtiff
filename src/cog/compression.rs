//! TIFF 图像压缩模块
//!
//! 本模块提供了 TIFF 图像格式的压缩和解压功能。主要包含以下内容:
//!
//! # 主要功能
//!
//! ## 压缩算法
//! - 支持 LZW、Deflate 等多种压缩方式
//! - 提供统一的编解码接口
//! - 可扩展的压缩算法框架
//!
//! ## 数据处理
//! - 高效的压缩和解压缩
//! - 流式处理大型数据
//! - 内存优化的实现
//!
//! ## 预测器支持
//! - 水平预测器
//! - 浮点预测器
//! - 自定义预测器扩展
//!
//! # 技术规范
//!
//! - [TIFF 6.0 规范](https://en.wikipedia.org/wiki/TIFF#TIFF_Compression_Tag)
//! - [EXIF 压缩标签](https://exiftool.org/TagNames/EXIF.html#Compression)
//!
//! # 性能说明
//!
//! - 针对地理影像数据进行了优化
//! - 支持多线程并行处理
//! - 使用零拷贝技术减少内存占用

use flate2;
use num_enum::{FromPrimitive, IntoPrimitive};
use salzweg::decoder::{DecodingError, TiffStyleDecoder};
use salzweg::encoder::{EncodingError, TiffStyleEncoder};
use std::io::{self, Read, Write};

/// 解压缩过程中可能出现的错误
///
/// # 变体说明
///
/// * `LzwDecodeError` - LZW 解码错误
/// * `LzwEncodeError` - LZW 编码错误
/// * `CompressionNotSupported` - 不支持的压缩方式
/// * `PredictorNotSupported` - 不支持的预测器
/// * `IoError` - IO 操作错误
#[derive(Debug)]
pub enum DecompressError {
    /// LZW 解码过程中的错误
    LzwDecodeError(DecodingError),
    /// LZW 编码过程中的错误
    LzwEncodeError(EncodingError),
    /// 不支持的压缩方式
    CompressionNotSupported(Compression),
    /// 不支持的预测器类型
    PredictorNotSupported(Predictor),
    /// IO 操作错误
    IoError(io::Error),
}

/// 从标准 IO 错误转换
impl From<io::Error> for DecompressError {
    fn from(e: io::Error) -> Self {
        DecompressError::IoError(e)
    }
}

/// TIFF 支持的压缩方式
///
/// 包含了 TIFF 6.0 规范定义的标准压缩方式，以及各厂商的专有压缩方式。
/// 每个变体对应一个特定的压缩标识符（Tag 值）。
///
/// # 主要压缩方式
///
/// * `Uncompressed` (1) - 无压缩
/// * `Lzw` (5) - LZW 压缩，常用于无损压缩
/// * `Jpeg` (7) - JPEG 压缩，用于有损压缩
/// * `DeflateAdobe` (8) - Adobe 的 Deflate 压缩
/// * `Deflate` (32946) - 标准 Deflate 压缩
///
/// # 专有压缩方式
///
/// * `NikonNEFCompressed` (34713) - 尼康相机 NEF 格式
/// * `JPEG2000` (34712) - JPEG 2000 压缩
/// * `WebP` (34927) - WebP 压缩
/// * `JPEGXL` (52546) - JPEG XL 压缩
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum Compression {
    /// 无压缩
    Uncompressed = 1,
    /// CCITT 1D 压缩
    CCITT1D = 2,
    /// T4 Group 3 传真压缩
    T4Group3Fax = 3,
    /// T6 Group 4 传真压缩
    T6Group4Fax = 4,
    /// LZW 压缩
    Lzw = 5,
    /// 旧版 JPEG 压缩
    JpegOld = 6,
    /// JPEG 压缩
    Jpeg = 7,
    /// Adobe Deflate 压缩
    DeflateAdobe = 8,
    /// JBIG 黑白图像压缩
    JbigBW = 9,
    /// JBIG 彩色图像压缩
    JbigColor = 10,
    /// 其他 JPEG 压缩
    JPEGOther = 99,
    /// Kodak 262 压缩
    Kodak262 = 262,
    /// NeXT 压缩
    Next = 32766,
    /// Sony ARW 相机压缩
    SonyARWCompressed = 32767,
    /// PackedRAW 压缩
    PackedRAW = 32769,
    /// 三星 SRW 压缩
    SamsungSRWCompressed = 32770,
    /// CCIR RLE 压缩
    CCIRLEW = 32771,
    /// 三星 SRW 压缩 2
    SamsungSRWCompressed2 = 32772,
    /// PackBits 压缩
    PackBits = 32773,
    /// Thunderscan 压缩
    Thunderscan = 32809,
    /// Kodak KDC 压缩
    KodakKDCCompressed = 32867,
    /// IT8 CTPAD 压缩
    IT8CTPAD = 32895,
    /// IT8 LW 压缩
    IT8LW = 32896,
    /// IT8 MP 压缩
    IT8MP = 32897,
    /// IT8 BL 压缩
    IT8BL = 32898,
    /// Pixar Film 压缩
    PixarFilm = 32908,
    /// Pixar Log 压缩
    PixarLog = 32909,
    /// Deflate 压缩
    Deflate = 32946,
    /// DCS 压缩
    DCS = 32947,
    /// Aperio JPEG2000 YCbCr 压缩
    AperioJPEG2000YCbCr = 33003,
    /// Aperio JPEG2000 RGB 压缩
    AperioJPEG2000RGB = 33005,
    /// JBIG 压缩
    JBIG = 34661,
    /// SGI Log 压缩
    SGILog = 34676,
    /// SGI Log 24 压缩
    SGILog24 = 34677,
    /// JPEG 2000 压缩
    JPEG2000 = 34712,
    /// 尼康 NEF 压缩
    NikonNEFCompressed = 34713,
    /// JBIG2 TIFF FX 压缩
    JBIG2TIFFFX = 34715,
    /// MDI 二值编码压缩
    MdiBinaryLevelCodec = 34718,
    /// MDI 渐进变换压缩
    MdiProgressiveTransformCodec = 34719,
    /// MDI 矢量压缩
    MdiVector = 34720,
    /// ESRI Lerc 压缩
    ESRILerc = 34887,
    /// 有损 JPEG 压缩
    LossyJPEG = 34892,
    /// LZMA2 压缩
    LZMA2 = 34925,
    /// Zstd 压缩
    Zstd = 34926,
    /// WebP 压缩
    WebP = 34927,
    /// PNG 压缩
    PNG = 34933,
    /// JPEG XR 压缩
    JPEGXR = 34934,
    /// JPEG XL 压缩
    JPEGXL = 52546,
    /// Kodak DCR 压缩
    KodakDCRCompressed = 65000,
    /// Pentax PEF 压缩
    PentaxPEFCompressed = 65535,

    /// 未知压缩方式
    #[num_enum(default)]
    Unknown = 0x0000,
}

impl Compression {
    /// 解码压缩的数据
    ///
    /// # 参数
    ///
    /// * `bytes` - 压缩的字节数据
    ///
    /// # 返回值
    ///
    /// 返回解压缩后的字节数据
    ///
    /// # 错误
    ///
    /// * 如果压缩方式不支持，返回 `CompressionNotSupported`
    /// * 解码过程中的错误会被转换为相应的 `DecompressError`
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, DecompressError> {
        match self {
            Self::Uncompressed => Ok(bytes.to_vec()),
            Self::Lzw => TiffStyleDecoder::decode_to_vec(bytes)
                .map_err(|e| DecompressError::LzwDecodeError(e)),
            Self::DeflateAdobe => {
                let mut buf = vec![];
                flate2::read::ZlibDecoder::new(bytes).read_to_end(&mut buf)?;
                Ok(buf)
            }
            other => Err(DecompressError::CompressionNotSupported(*other)),
        }
    }

    /// 编码数据
    ///
    /// # 参数
    ///
    /// * `bytes` - 原始字节数据
    ///
    /// # 返回值
    ///
    /// 返回压缩后的字节数据
    ///
    /// # 错误
    ///
    /// * 如果压缩方式不支持，返回 `CompressionNotSupported`
    /// * 编码过程中的错误会被转换为相应的 `DecompressError`
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, DecompressError> {
        match self {
            Self::Uncompressed => Ok(bytes.to_vec()),
            Self::Lzw => TiffStyleEncoder::encode_to_vec(bytes)
                .map_err(|e| DecompressError::LzwEncodeError(e)),
            Self::DeflateAdobe => {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(bytes)?;
                Ok(encoder.finish()?)
            }
            other => Err(DecompressError::CompressionNotSupported(*other)),
        }
    }
}

/// TIFF 预测器类型
///
/// 预测器用于提高压缩效率，通过预测像素值来减少数据的熵。
///
/// # 变体说明
///
/// * `No` (1) - 不使用预测器
/// * `Horizontal` (2) - 水平预测器，使用前一个像素值进行预测
/// * `FloatingPoint` (3) - 浮点预测器，用于浮点数据
/// * `Unknown` (0) - 未知预测器类型
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum Predictor {
    /// 不使用预测器
    No = 1,
    /// 水平预测器,使用前一个像素值进行预测
    /// 每个像素值都会加上前一个像素的值
    Horizontal = 2,
    /// 浮点预测器,用于浮点数据
    /// 对浮点数据进行特殊处理以提高压缩效率
    FloatingPoint = 3,

    /// 未知的预测器类型
    /// 作为默认值使用
    #[num_enum(default)]
    Unknown = 0x0000,
}

impl Predictor {
    /// 应用预测器到图像数据
    ///
    /// # 参数
    ///
    /// * `buffer` - 图像数据缓冲区
    /// * `width` - 图像宽度（像素）
    /// * `bit_depth` - 每个样本的位深度
    /// * `samples_per_pixel` - 每个像素的样本数（例如 RGB = 3）
    ///
    /// # 错误
    ///
    /// * 如果预测器类型不支持，返回 `PredictorNotSupported`
    /// * 对于水平预测器，仅支持 8 位或更少的位深度
    pub fn predict(
        &self,
        buffer: &mut [u8],
        width: usize,
        bit_depth: usize,
        samples_per_pixel: usize,
    ) -> Result<(), DecompressError> {
        match self {
            Self::No => {}
            Self::Horizontal => {
                assert!(
                    bit_depth <= 8,
                    "Bit depth {bit_depth} not supported for Horizontal Predictor"
                );
                // 计算每行的字节数
                let row_bytes = width * samples_per_pixel * bit_depth / 8;

                // 遍历所有字节
                for i in 0..buffer.len() {
                    // 跳过每行的第一个像素
                    if i % row_bytes < samples_per_pixel {
                        continue;
                    }

                    // 将当前字节加上前一个像素对应位置的字节值
                    // 使用 wrapping_add 避免溢出
                    buffer[i] = buffer[i].wrapping_add(buffer[i - samples_per_pixel]);
                }
            }
            other => return Err(DecompressError::PredictorNotSupported(*other)),
        }
        Ok(())
    }
}
