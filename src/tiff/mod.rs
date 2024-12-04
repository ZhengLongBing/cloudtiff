//! TIFF文件格式处理模块
//!
//! 本模块提供了读取、写入和操作TIFF（Tagged Image File Format）文件的功能。
//! 支持标准TIFF和BigTIFF格式。

use std::collections::HashMap;
use std::fmt::Display;
use std::io::{self, Read, Seek, Write};

mod endian;
mod error;
mod ifd;
mod tag;

pub use endian::Endian;
pub use error::TiffError;
pub use ifd::Ifd;
pub use tag::{Tag, TagData, TagId, TagType};

/// TIFF变体枚举，用于区分标准TIFF和BigTIFF格式
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TiffVariant {
    /// 标准TIFF格式
    Normal,
    /// BigTIFF格式
    Big,
}

impl TiffVariant {
    /// 根据TIFF变体读取偏移量
    fn read_offset<R: Read>(&self, endian: Endian, stream: &mut R) -> io::Result<u64> {
        match self {
            TiffVariant::Normal => endian.read::<4, u32>(stream).map(|v| v as u64),
            TiffVariant::Big => endian.read(stream),
        }
    }

    /// 根据TIFF变体写入偏移量
    fn write_offset<W: Write>(
        &self,
        endian: Endian,
        stream: &mut W,
        offset: u64,
    ) -> io::Result<()> {
        match self {
            TiffVariant::Normal => endian.write(stream, offset as u32),
            TiffVariant::Big => endian.write(stream, offset),
        }
    }

    /// 返回偏移量的字节大小
    const fn offset_bytesize(&self) -> usize {
        match self {
            TiffVariant::Normal => 4,
            TiffVariant::Big => 8,
        }
    }
}

/// TIFF偏移量类型，用于存储标签ID和对应的偏移量
pub type TiffOffsets = HashMap<u16, u64>;

/// TIFF结构体，表示一个TIFF文件
#[derive(Clone, Debug)]
pub struct Tiff {
    /// 字节序
    pub endian: Endian,
    /// TIFF变体（标准TIFF或BigTIFF）
    pub variant: TiffVariant,
    /// IFD（图像文件目录）列表
    pub ifds: Vec<Ifd>,
}

impl Tiff {
    /// 创建一个新的TIFF实例
    pub fn new(endian: Endian, variant: TiffVariant) -> Self {
        Self {
            endian,
            variant,
            ifds: vec![Ifd::new()],
        }
    }

    /// 从流中读取TIFF文件
    pub fn open<R: Read + Seek>(stream: &mut R) -> Result<Self, TiffError> {
        // 读取TIFF头部
        let mut buf = [0; 4];
        stream.read_exact(&mut buf)?;

        // 确定字节序
        let endian = match &buf[..2] {
            b"II" => Endian::Little,
            b"MM" => Endian::Big,
            _ => return Err(TiffError::BadMagicBytes),
        };

        // 确定TIFF变体
        let variant = match &buf[2..4] {
            b"\0*" | b"*\0" => TiffVariant::Normal,
            b"\0+" | b"+\0" => TiffVariant::Big,
            _ => return Err(TiffError::BadMagicBytes),
        };

        // 处理BigTIFF额外的头部信息
        if TiffVariant::Big == variant {
            let _offset_bytesize: u16 = endian.read(stream)?; // 应该是0x0008
            let _: u16 = endian.read(stream)?; // 应该是0x0000
        }

        // 读取IFDs
        let mut ifds = vec![];
        let mut ifd_offset = variant.read_offset(endian, stream)?;
        while ifd_offset != 0 {
            let (ifd, next_offset) = Ifd::parse(stream, ifd_offset, endian, variant)?;
            ifd_offset = next_offset;
            ifds.push(ifd);
        }

        Ok(Self {
            endian,
            variant,
            ifds,
        })
    }

    /// 获取第一个IFD（IFD0）
    pub fn ifd0(&self) -> Result<&Ifd, TiffError> {
        self.ifds.get(0).ok_or(TiffError::NoIfd0)
    }

    /// 添加一个新的IFD
    pub fn add_ifd(&mut self) -> &mut Ifd {
        self.ifds.push(Ifd::new());
        let n = self.ifds.len();
        self.ifds.get_mut(n - 1).unwrap()
    }

    /// 将TIFF结构编码到流中
    pub fn encode<W: Write + Seek>(&self, stream: &mut W) -> Result<Vec<TiffOffsets>, io::Error> {
        let endian = self.endian;

        // 写入字节序标记
        match endian {
            Endian::Little => stream.write(b"II")?,
            Endian::Big => stream.write(b"MM")?,
        };

        // 写入TIFF版本标记
        match self.variant {
            TiffVariant::Normal => endian.write(stream, 0x002A_u16)?,
            TiffVariant::Big => endian.write(stream, 0x002B_u16)?,
        };

        // 对于BigTIFF，写入额外的头部信息
        if self.variant == TiffVariant::Big {
            endian.write(stream, 0x0008_u16)?; // 偏移量字节大小
            endian.write(stream, 0x0000_u16)?; // 保留字段
        }

        // 写入IFD0偏移量
        if self.variant == TiffVariant::Big {
            endian.write(stream, 16 as u64)?;
        } else {
            endian.write(stream, 8 as u32)?;
        }

        // 编码并写入IFDs
        let mut offsets = vec![];
        for (i, ifd) in self.ifds.iter().enumerate() {
            let ifd_offsets = ifd.encode(stream, endian, self.variant, i == self.ifds.len() - 1)?;
            offsets.push(ifd_offsets);
        }

        Ok(offsets)
    }
}

/// 实现Display trait，用于格式化输出TIFF结构
impl Display for Tiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tiff: {{{:?} Endian, {:?} Variant}}",
            self.endian, self.variant
        )?;
        for (i, ifd) in self.ifds.iter().enumerate() {
            write!(f, "\n  IFD {i}:")?;
            for tag in ifd.0.iter() {
                write!(f, "\n    {}", tag)?;
            }
        }
        Ok(())
    }
}
