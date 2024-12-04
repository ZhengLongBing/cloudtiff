//! TIFF图像文件目录(IFD)模块
//!
//! 本模块实现了TIFF文件中IFD(Image File Directory)的解析和写入功能。
//! IFD包含了描述图像数据的标签集合。

use num_traits::NumCast;

use super::{Endian, Tag, TagData, TagId, TagType, TiffError, TiffOffsets, TiffVariant};
use std::{
    collections::HashMap,
    io::{self, Read, Seek, SeekFrom, Write},
};

/// 表示TIFF文件中的一个IFD(图像文件目录)
///
/// IFD包含一组标签,每个标签描述图像的某个属性
#[derive(Clone, Debug)]
pub struct Ifd(pub Vec<Tag>);

impl Ifd {
    /// 创建一个新的空IFD
    pub fn new() -> Self {
        Self(vec![])
    }

    /// 从输入流解析IFD
    ///
    /// # 参数
    /// * `stream` - 输入流
    /// * `offset` - IFD在文件中的偏移量
    /// * `endian` - 字节序
    /// * `variant` - TIFF变体类型(普通或BigTIFF)
    ///
    /// # 返回
    /// 返回解析出的IFD和下一个IFD的偏移量
    pub fn parse<R: Read + Seek>(
        stream: &mut R,
        offset: u64,
        endian: Endian,
        variant: TiffVariant,
    ) -> io::Result<(Ifd, u64)> {
        // 定位到IFD起始位置
        stream.seek(SeekFrom::Start(offset))?;

        // 读取标签数量
        let tag_count = match variant {
            TiffVariant::Normal => endian.read::<2, u16>(stream)? as u64,
            TiffVariant::Big => endian.read(stream)?,
        };

        // 解析每个标签
        let mut tags = Vec::with_capacity(tag_count as usize);
        for _ in 0..tag_count {
            // 读取标签基本信息
            let code = endian.read(stream)?;
            let datatype: TagType = endian.read::<2, u16>(stream)?.into();
            let count = variant.read_offset(endian, stream)? as usize;

            // 计算数据大小
            let data_size = count * datatype.size_in_bytes();
            let offset_size = variant.offset_bytesize();
            let mut data: Vec<u8> = vec![0; data_size.max(offset_size)];

            // 读取标签数据
            if data_size > offset_size {
                // 数据存储在偏移位置
                let data_offset = variant.read_offset(endian, stream)? as u64;
                let pos = stream.stream_position()?;
                stream.seek(SeekFrom::Start(data_offset))?;
                stream.read_exact(&mut data)?;
                stream.seek(SeekFrom::Start(pos))?;
            } else {
                // 数据直接存储在标签中
                stream.read_exact(&mut data)?;
                if data_size < offset_size {
                    data = data[0..data_size].to_vec();
                }
            }

            tags.push(Tag {
                code,
                datatype,
                endian,
                count,
                data,
            });
        }

        let ifd = Ifd(tags);
        let next_ifd_offset = variant.read_offset(endian, stream)? as u64;

        Ok((ifd, next_ifd_offset))
    }

    /// 通过标签代码获取标签
    pub fn get_tag_by_code(&self, code: u16) -> Option<&Tag> {
        let Self(tags) = &self;
        tags.iter().find(|tag| tag.code == code)
    }

    /// 通过标签ID获取标签
    pub fn get_tag(&self, id: TagId) -> Result<&Tag, TiffError> {
        let code: u16 = id.into();
        let Self(tags) = &self;
        tags.iter()
            .find(|tag| tag.code == code)
            .ok_or(TiffError::MissingTag(id))
    }

    /// 获取标签的多个值
    pub fn get_tag_values<T: NumCast>(&self, id: TagId) -> Result<Vec<T>, TiffError> {
        self.get_tag(id)?.values().ok_or(TiffError::BadTag(id))
    }

    /// 获取标签的单个值
    pub fn get_tag_value<T: NumCast + Copy>(&self, id: TagId) -> Result<T, TiffError> {
        self.get_tag(id)?.value().ok_or(TiffError::BadTag(id))
    }

    /// 通过标签代码设置标签
    pub fn set_tag_by_code(&self, code: u16) -> Option<&Tag> {
        let Self(tags) = &self;
        tags.iter().find(|tag| tag.code == code)
    }

    /// 设置标签的值
    pub fn set_tag<I: Into<u16>>(&mut self, id: I, data: TagData, endian: Endian) {
        let code: u16 = id.into();
        let tag = Tag::new(code, endian, data);
        let tags = &mut self.0;
        if let Some(index) = tags.iter().position(|tag| tag.code == code) {
            tags[index] = tag;
        } else {
            tags.push(tag);
        }
    }

    /// 将IFD编码写入输出流
    ///
    /// # 参数
    /// * `stream` - 输出流
    /// * `endian` - 字节序
    /// * `variant` - TIFF变体类型
    /// * `last_ifd` - 是否为最后一个IFD
    ///
    /// # 返回
    /// 返回标签偏移量映射表
    pub fn encode<W: Write + Seek>(
        &self,
        stream: &mut W,
        endian: Endian,
        variant: TiffVariant,
        last_ifd: bool,
    ) -> Result<TiffOffsets, io::Error> {
        // 写入标签数量
        let tag_count = self.0.len();
        match variant {
            TiffVariant::Normal => endian.write(stream, tag_count as u16)?,
            TiffVariant::Big => endian.write(stream, tag_count as u64)?,
        };

        // 初始化必要的变量
        let mut offsets = HashMap::new();
        let mut extra_data = vec![];
        let offset_size = variant.offset_bytesize();
        let (_header_size, tag_size) = match variant {
            TiffVariant::Normal => (2, 12),
            TiffVariant::Big => (8, 20),
        };
        let extra_data_offset =
            stream.stream_position()? + tag_size * tag_count as u64 + offset_size as u64;

        // 写入每个标签
        for (_i, tag) in self.0.iter().enumerate() {
            // 写入标签头部信息
            endian.write(stream, tag.code as u16)?;
            endian.write(stream, tag.datatype as u16)?;
            variant.write_offset(endian, stream, tag.count as u64)?;

            // 写入标签数据
            let offset = if tag.data.len() > offset_size {
                // 数据写入额外数据区
                let data_offset = extra_data_offset + extra_data.len() as u64;
                variant.write_offset(endian, stream, data_offset)?;
                extra_data.extend_from_slice(&tag.data);
                data_offset
            } else {
                // 数据直接写入标签
                let bytes: Vec<u8> = tag
                    .data
                    .clone()
                    .into_iter()
                    .chain(vec![0; offset_size].into_iter())
                    .take(offset_size)
                    .collect();
                let data_offset = stream.stream_position()?;
                stream.write_all(&bytes)?;
                data_offset
            };

            offsets.insert(tag.code, offset);
        }

        // 写入下一个IFD的偏移量
        if last_ifd {
            variant.write_offset(endian, stream, 0)?;
        } else {
            let current_pos = stream.stream_position()?;
            variant.write_offset(
                endian,
                stream,
                current_pos + extra_data.len() as u64 + offset_size as u64,
            )?;
        }

        // 写入额外数据
        stream.write_all(&extra_data)?;

        Ok(offsets)
    }
}
