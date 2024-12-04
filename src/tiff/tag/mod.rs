//! TIFF标签处理模块
//!
//! 本模块提供了处理TIFF文件标签(Tag)的功能。标签包含了图像的元数据信息,
//! 如图像尺寸、颜色空间、压缩方式等。
//!
//! 参考标准:
//! - [EXIF 2.2规范](https://web.archive.org/web/20220119170528/http://www.exif.org/Exif2-2.PDF)
//! - [CIPA DC-008规范](https://web.archive.org/web/20190624045241if_/http://www.cipa.jp:80/std/documents/e/DC-008-Translation-2019-E.pdf)
//! - [MIT Media Lab EXIF文档](https://www.media.mit.edu/pia/Research/deepview/exif)

use super::Endian;
use eio::FromBytes;
use num_enum::{FromPrimitive, IntoPrimitive};
use num_traits::{cast::NumCast, ToPrimitive};
use std::fmt::Display;

mod data;
mod id;

pub use data::TagData;
pub use id::TagId;

/// TIFF标签结构体
///
/// 表示TIFF文件中的一个标签,包含标签代码、数据类型、数据计数和实际数据。
#[derive(Clone, Debug)]
pub struct Tag {
    /// 标签代码,用于标识标签类型
    pub code: u16,
    /// 标签数据类型
    pub datatype: TagType,
    /// 数据项数量
    pub count: usize,
    /// 实际数据内容
    pub data: Vec<u8>,
    /// 字节序
    pub endian: Endian,
}

impl Tag {
    /// 创建新的标签实例
    ///
    /// # 参数
    /// * `code` - 标签代码
    /// * `endian` - 字节序
    /// * `data` - 标签数据
    pub fn new(code: u16, endian: Endian, data: TagData) -> Self {
        Self {
            code,
            datatype: data.tag_type(),
            count: data.len(),
            data: data.bytes(endian),
            endian,
        }
    }

    /// 获取标签ID
    pub fn id(&self) -> Option<TagId> {
        TagId::try_from(self.code).ok()
    }

    /// 获取单个数值
    ///
    /// 当标签只包含一个值时返回该值
    pub fn value<T: NumCast + Copy>(&self) -> Option<T> {
        match self.values() {
            Some(v) if v.len() == 1 => Some(v[0]),
            _ => None,
        }
    }

    /// 获取所有数值
    ///
    /// 根据标签类型解码并返回所有数值
    pub fn values<T: NumCast>(&self) -> Option<Vec<T>> {
        match self.datatype {
            TagType::Byte => self.decode::<1, u8, T>(),
            TagType::Ascii => self.decode::<1, u8, T>(),
            TagType::Short => self.decode::<2, u16, T>(),
            TagType::Long => self.decode::<4, u32, T>(),
            TagType::SByte => self.decode::<1, i8, T>(),
            TagType::Undefined => self.decode::<1, u8, T>(),
            TagType::SShort => self.decode::<2, i16, T>(),
            TagType::SLong => self.decode::<4, i32, T>(),
            TagType::Float => self.decode::<4, f32, T>(),
            TagType::Double => self.decode::<8, f64, T>(),
            TagType::Ifd => self.decode::<4, u32, T>(),
            TagType::Long8 => self.decode::<8, u64, T>(),
            TagType::SLong8 => self.decode::<8, i64, T>(),
            TagType::Ifd8 => self.decode::<8, u64, T>(),
            TagType::Unknown => self.decode::<1, u8, T>(),
            TagType::Rational => self.decode_rational::<4, u32, T>(),
            TagType::SRational => self.decode_rational::<4, i32, T>(),
        }
    }

    /// 尝试将数据转换为字符串
    ///
    /// 仅支持ASCII、Byte和Unknown类型的转换
    pub fn try_to_string(&self) -> Option<String> {
        match self.datatype {
            TagType::Ascii | TagType::Byte | TagType::Unknown => {
                String::from_utf8(self.data.clone()).ok()
            }
            _ => None,
        }
    }

    /// 将数据转换为字符串(可能有损)
    ///
    /// 对于不同类型的数据采用不同的转换策略
    pub fn as_string_lossy(&self) -> String {
        match self.datatype {
            TagType::Ascii => String::from_utf8_lossy(&self.data).into_owned(),
            TagType::Float | TagType::Double | TagType::Rational | TagType::SRational => {
                match self.values::<f64>() {
                    Some(v) if v.len() == 1 => format!("{}", v[0]),
                    Some(v) => format!("{:?}", v),
                    None => format!("Undefined"),
                }
            }
            _ => match self.values::<i64>() {
                Some(v) if v.len() == 1 => format!("{}", v[0]),
                Some(v) => format!("{:?}", v),
                None => format!("Undefined"),
            },
        }
    }

    /// 解码普通数值类型的数据
    fn decode<const N: usize, A: FromBytes<N> + ToPrimitive, T: NumCast>(&self) -> Option<Vec<T>> {
        self.endian.decode_all_to_primative::<N, A, T>(&self.data)
    }

    /// 解码有理数类型的数据
    fn decode_rational<const N: usize, A: FromBytes<N> + ToPrimitive, T: NumCast>(
        &self,
    ) -> Option<Vec<T>> {
        self.data
            .chunks_exact(2 * N)
            .map(|chunk| {
                chunk[..N]
                    .try_into()
                    .ok()
                    .and_then(|arr| {
                        self.endian
                            .decode::<N, A>(arr)
                            .ok()
                            .and_then(|v| v.to_f64())
                    })
                    .and_then(|numerator| {
                        chunk[N..]
                            .try_into()
                            .ok()
                            .and_then(|arr| {
                                self.endian
                                    .decode::<N, A>(arr)
                                    .ok()
                                    .and_then(|v| v.to_f64())
                            })
                            .and_then(|denominator| T::from(numerator / denominator))
                    })
            })
            .collect()
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value_string = format!("{}", self.as_string_lossy().replace("\n", "\\n"));
        if value_string.len() > 100 {
            value_string = format!("{}...", &value_string[..98])
        }
        let id_string = match self.id() {
            Some(id) => format!("{id:?}"),
            None => format!("Unknown({})", self.code),
        };
        write!(
            f,
            "{} {:?}[{}]: {}",
            id_string, self.datatype, self.count, value_string
        )
    }
}

/// TIFF标签数据类型枚举
///
/// 定义了TIFF标签支持的所有数据类型
#[derive(Debug, PartialEq, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum TagType {
    /// 8位无符号整数
    Byte = 1,
    /// ASCII字符串
    Ascii = 2,
    /// 16位无符号整数
    Short = 3,
    /// 32位无符号整数
    Long = 4,
    /// 无符号有理数(两个32位无符号整数的比值)
    Rational = 5,
    /// 8位有符号整数
    SByte = 6,
    /// 未定义类型
    Undefined = 7,
    /// 16位有符号整数
    SShort = 8,
    /// 32位有符号整数
    SLong = 9,
    /// 有符号有理数(两个32位有符号整数的比值)
    SRational = 10,
    /// 32位浮点数
    Float = 11,
    /// 64位浮点数
    Double = 12,
    /// IFD偏移量
    Ifd = 13,
    /// 64位无符号整数
    Long8 = 16,
    /// 64位有符号整数
    SLong8 = 17,
    /// 64位IFD偏移量
    Ifd8 = 18,

    /// 未知类型
    #[num_enum(default)]
    Unknown = 0xFFFF,
}

impl TagType {
    /// 获取数据类型的字节大小
    pub const fn size_in_bytes(&self) -> usize {
        match self {
            TagType::Byte => 1,
            TagType::Ascii => 1,
            TagType::Short => 2,
            TagType::Long => 4,
            TagType::Rational => 8,
            TagType::SByte => 1,
            TagType::Undefined => 1,
            TagType::SShort => 2,
            TagType::SLong => 4,
            TagType::SRational => 8,
            TagType::Float => 4,
            TagType::Double => 8,
            TagType::Ifd => 4,
            TagType::Long8 => 8,
            TagType::SLong8 => 8,
            TagType::Ifd8 => 8,
            TagType::Unknown => 1,
        }
    }
}
