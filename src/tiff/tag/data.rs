//! TIFF标签数据模块
//!
//! 本模块定义了TIFF标签的数据类型和相关操作。
//! 包括各种基本数据类型的存储和转换功能。

use super::TagType;
use crate::tiff::Endian;

/// TIFF标签数据类型枚举
///
/// 包含了TIFF规范中定义的所有数据类型
#[derive(Clone, Debug)]
pub enum TagData {
    /// 8位无符号整数数组
    Byte(Vec<u8>),
    /// ASCII字符串(以null结尾的字节数组)
    Ascii(Vec<u8>),
    /// 16位无符号整数数组
    Short(Vec<u16>),
    /// 32位无符号整数数组
    Long(Vec<u32>),
    /// 无符号有理数数组,每个元素由两个u32组成(分子,分母)
    Rational(Vec<(u32, u32)>),
    /// 8位有符号整数数组
    SByte(Vec<i8>),
    /// 未定义类型的字节数组
    Undefined(Vec<u8>),
    /// 16位有符号整数数组
    SShort(Vec<i16>),
    /// 32位有符号整数数组
    SLong(Vec<i32>),
    /// 有符号有理数数组,每个元素由两个i32组成(分子,分母)
    SRational(Vec<(i32, i32)>),
    /// 32位浮点数数组
    Float(Vec<f32>),
    /// 64位浮点数数组
    Double(Vec<f64>),
    /// IFD偏移量(32位)
    Ifd(u32),
    /// 64位无符号整数数组
    Long8(Vec<u64>),
    /// 64位有符号整数数组
    SLong8(Vec<i64>),
    /// IFD偏移量(64位)
    Ifd8(u64),
    /// 未知类型数据
    Unknown(Vec<u8>),
}

impl TagData {
    /// 从字符串创建ASCII类型的标签数据
    pub fn from_string(s: &str) -> Self {
        Self::Ascii(s.as_bytes().to_vec())
    }

    /// 从u16值创建Short类型的标签数据
    pub fn from_short(v: u16) -> Self {
        Self::Short(vec![v])
    }

    /// 从u32值创建Long类型的标签数据
    pub fn from_long(v: u32) -> Self {
        Self::Long(vec![v])
    }

    /// 获取标签数据中的元素数量
    pub fn len(&self) -> usize {
        match self {
            Self::Byte(vec) => vec.len(),
            Self::Ascii(vec) => vec.len(),
            Self::Short(vec) => vec.len(),
            Self::Long(vec) => vec.len(),
            Self::Rational(vec) => vec.len(),
            Self::SByte(vec) => vec.len(),
            Self::Undefined(vec) => vec.len(),
            Self::SShort(vec) => vec.len(),
            Self::SLong(vec) => vec.len(),
            Self::SRational(vec) => vec.len(),
            Self::Float(vec) => vec.len(),
            Self::Double(vec) => vec.len(),
            Self::Ifd(_) => 1,
            Self::Long8(vec) => vec.len(),
            Self::SLong8(vec) => vec.len(),
            Self::Ifd8(_) => 1,
            Self::Unknown(vec) => vec.len(),
        }
    }

    /// 获取标签数据的类型
    pub fn tag_type(&self) -> TagType {
        match self {
            Self::Byte(_) => TagType::Byte,
            Self::Ascii(_) => TagType::Ascii,
            Self::Short(_) => TagType::Short,
            Self::Long(_) => TagType::Long,
            Self::Rational(_) => TagType::Rational,
            Self::SByte(_) => TagType::SByte,
            Self::Undefined(_) => TagType::Undefined,
            Self::SShort(_) => TagType::SShort,
            Self::SLong(_) => TagType::SLong,
            Self::SRational(_) => TagType::SRational,
            Self::Float(_) => TagType::Float,
            Self::Double(_) => TagType::Double,
            Self::Ifd(_) => TagType::Ifd,
            Self::Long8(_) => TagType::Long8,
            Self::SLong8(_) => TagType::SLong8,
            Self::Ifd8(_) => TagType::Ifd8,
            Self::Unknown(_) => TagType::Unknown,
        }
    }

    /// 将标签数据转换为字节序列
    ///
    /// # 参数
    /// * `endian` - 字节序(大端/小端)
    ///
    /// # 返回值
    /// 返回按指定字节序编码后的字节数组
    pub fn bytes(&self, endian: Endian) -> Vec<u8> {
        match self {
            // 对于简单类型,直接使用encode_all进行编码
            Self::Byte(vec) => endian.encode_all(vec),
            Self::Ascii(vec) => endian.encode_all(vec),
            Self::Short(vec) => endian.encode_all(vec),
            Self::Long(vec) => endian.encode_all(vec),
            // 对于有理数类型,需要分别编码分子和分母
            Self::Rational(vec) => vec
                .iter()
                .map(|(a, b)| {
                    endian
                        .encode(*a)
                        .into_iter()
                        .chain(endian.encode(*b).into_iter())
                        .collect::<Vec<u8>>()
                })
                .flatten()
                .collect(),
            Self::SByte(vec) => endian.encode_all(vec),
            Self::Undefined(vec) => endian.encode_all(vec),
            Self::SShort(vec) => endian.encode_all(vec),
            Self::SLong(vec) => endian.encode_all(vec),
            Self::SRational(vec) => vec
                .iter()
                .map(|(a, b)| {
                    endian
                        .encode(*a)
                        .into_iter()
                        .chain(endian.encode(*b).into_iter())
                        .collect::<Vec<u8>>()
                })
                .flatten()
                .collect(),
            Self::Float(vec) => endian.encode_all(vec),
            Self::Double(vec) => endian.encode_all(vec),
            Self::Ifd(v) => endian.encode(*v).to_vec(),
            Self::Long8(vec) => endian.encode_all(vec),
            Self::SLong8(vec) => endian.encode_all(vec),
            Self::Ifd8(v) => endian.encode(*v).to_vec(),
            Self::Unknown(vec) => endian.encode_all(vec),
        }
    }
}
