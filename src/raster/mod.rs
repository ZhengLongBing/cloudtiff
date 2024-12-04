//! 栅格图像处理模块
//!
//! 本模块提供了处理和操作栅格图像数据的功能。

use crate::tiff::Endian;
use std::fmt::Display;

mod image;
mod ops;
mod photometrics;

pub use ops::ResizeFilter;
pub use photometrics::{
    ExtraSamples, PhotometricInterpretation, PlanarConfiguration, SampleFormat,
};

// TODO: 处理奇特的位序问题。已经遇到过两种不同的情况。

/// 栅格操作过程中可能出现的错误
#[derive(Debug)]
pub enum RasterError {
    /// 缓冲区大小不匹配错误
    /// 包含 (实际大小, (宽度, 高度), 每个样本的位数, 每个像素的字节数)
    BufferSize((usize, (u32, u32), Vec<u16>, u32)),
    /// 不支持的操作错误
    NotSupported(String),
}

/// 表示一个栅格图像
#[derive(Clone, Debug)]
pub struct Raster {
    /// 图像尺寸 (宽度, 高度)
    pub dimensions: (u32, u32),
    /// 图像数据缓冲区
    pub buffer: Vec<u8>,
    /// 每个样本的位数
    pub bits_per_sample: Vec<u16>,
    /// 图像的光度解释方式
    pub interpretation: PhotometricInterpretation,
    /// 样本格式
    pub sample_format: Vec<SampleFormat>,
    /// 额外的样本信息
    pub extra_samples: Vec<ExtraSamples>,
    /// 字节序
    pub endian: Endian,
    /// 每个像素的总位数（bits_per_sample的总和）
    bits_per_pixel: u32,
}

impl Raster {
    /// 创建一个新的 Raster 实例
    ///
    /// # 参数
    /// * `dimensions` - 图像尺寸 (宽度, 高度)
    /// * `buffer` - 图像数据缓冲区
    /// * `bits_per_sample` - 每个样本的位数
    /// * `interpretation` - 图像的光度解释方式
    /// * `sample_format` - 样本格式
    /// * `extra_samples` - 额外的样本信息
    /// * `endian` - 字节序
    ///
    /// # 返回
    /// * `Result<Self, RasterError>` - 成功则返回 Raster 实例，失败则返回错误
    pub fn new(
        dimensions: (u32, u32),
        buffer: Vec<u8>,
        bits_per_sample: Vec<u16>,
        interpretation: PhotometricInterpretation,
        sample_format: Vec<SampleFormat>,
        extra_samples: Vec<ExtraSamples>,
        endian: Endian,
    ) -> Result<Self, RasterError> {
        let bits_per_pixel = bits_per_sample.iter().sum::<u16>() as u32;
        let bytes_per_pixel = bits_per_pixel / 8;
        let required_bytes =
            dimensions.0 as usize * dimensions.1 as usize * bytes_per_pixel as usize;

        // 检查缓冲区大小是否正确
        if buffer.len() != required_bytes as usize {
            Err(RasterError::BufferSize((
                buffer.len(),
                dimensions,
                bits_per_sample,
                bytes_per_pixel,
            )))
        } else {
            Ok(Self {
                dimensions,
                buffer,
                bits_per_sample,
                interpretation,
                sample_format,
                extra_samples,
                endian,
                bits_per_pixel,
            })
        }
    }

    /// 创建一个空白的 Raster 实例
    ///
    /// # 参数
    /// * `dimensions` - 图像尺寸 (宽度, 高度)
    /// * `bits_per_sample` - 每个样本的位数
    /// * `interpretation` - 图像的光度解释方式
    /// * `sample_format` - 样本格式
    /// * `extra_samples` - 额外的样本信息
    /// * `endian` - 字节序
    ///
    /// # 返回
    /// * `Self` - 新创建的 Raster 实例
    pub fn blank(
        dimensions: (u32, u32),
        bits_per_sample: Vec<u16>,
        interpretation: PhotometricInterpretation,
        sample_format: Vec<SampleFormat>,
        extra_samples: Vec<ExtraSamples>,
        endian: Endian,
    ) -> Self {
        // 计算每个像素的总位数
        let bits_per_pixel = bits_per_sample.iter().sum::<u16>() as u32;

        // 计算所需的总字节数
        // 宽度 * 高度 * 每像素位数 / 8 (转换为字节)
        let required_bytes =
            dimensions.0 as usize * dimensions.1 as usize * bits_per_pixel as usize / 8;

        // 创建一个填充为0的缓冲区
        let buffer = vec![0; required_bytes];

        // 构造并返回 Raster 实例
        Self {
            dimensions,
            buffer,
            bits_per_sample,
            interpretation,
            sample_format,
            extra_samples,
            endian,
            bits_per_pixel,
        }
    }

    /// 获取指定位置的像素值
    ///
    /// # 参数
    /// * `x` - 像素的 x 坐标
    /// * `y` - 像素的 y 坐标
    ///
    /// # 返回
    /// * `Option<Vec<u8>>` - 如果坐标有效，返回像素值；否则返回 None
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Vec<u8>> {
        // 检查坐标是否在图像范围内
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return None;
        }

        let bytes_per_row = self.row_size();
        let row_offset: u32 = y * bytes_per_row;

        // 计算像素在缓冲区中的起始和结束位置
        let start_col_offset_bits = x * self.bits_per_pixel;
        let start_col_offset_bytes = start_col_offset_bits / 8;
        let start = (row_offset + start_col_offset_bytes) as usize;

        let end_col_offset_bits = x * self.bits_per_pixel + self.bits_per_pixel;
        let end_col_offset_bytes = (end_col_offset_bits + 7) / 8;
        let end = (row_offset + end_col_offset_bytes) as usize;

        let mut pixel = self.buffer[start..end].to_vec();
        let n = end - start;

        // 处理起始字节的位对齐
        let start_mask = 0xFF_u8 >> (start_col_offset_bits - start_col_offset_bytes * 8);
        pixel[0] &= start_mask;

        // 处理结束字节的位对齐
        let end_mask =
            ((0xFF_u16 << (end_col_offset_bytes * 8 - end_col_offset_bits)) & 0xFF_u16) as u8;
        pixel[n - 1] &= end_mask;

        Some(pixel)
    }

    /// 设置指定位置的像素值
    ///
    /// # 参数
    /// * `x` - 像素的 x 坐标
    /// * `y` - 像素的 y 坐标
    /// * `pixel` - 要设置的像素值
    ///
    /// # 返回
    /// * `Result<(), String>` - 成功则返回 Ok(()), 失败则返回错误信息
    pub fn put_pixel(&mut self, x: u32, y: u32, pixel: Vec<u8>) -> Result<(), String> {
        // 检查坐标是否在图像范围内
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return Err("无效的像素索引".into());
        }

        let bytes_per_row = self.row_size();
        let row_offset: u32 = y * bytes_per_row;

        // 计算像素在缓冲区中的起始和结束位置
        let start_col_offset_bits = x * self.bits_per_pixel;
        let start_col_offset_bytes = start_col_offset_bits / 8;
        let start = (row_offset + start_col_offset_bytes) as usize;

        let end_col_offset_bits = x * self.bits_per_pixel + self.bits_per_pixel;
        let end_col_offset_bytes = (end_col_offset_bits + 7) / 8;
        let end = (row_offset + end_col_offset_bytes) as usize;

        let n = end - start;

        // 检查提供的像素数据大小是否正确
        if pixel.len() != n {
            return Err("像素大小不匹配".into());
        }

        // 处理起始字节的位对齐
        let start_mask = 0xFF_u8 >> (start_col_offset_bits - start_col_offset_bytes * 8);
        self.buffer[start] = (self.buffer[start] & !start_mask) | (pixel[0] & start_mask);

        // 处理结束字节的位对齐（如果像素跨越多个字节）
        if n > 1 {
            let end_mask =
                ((0xFF_u16 << (end_col_offset_bytes * 8 - end_col_offset_bits)) & 0xFF_u16) as u8;
            self.buffer[end - 1] = (self.buffer[end - 1] & !end_mask) | (pixel[n - 1] & end_mask);
        }

        // 复制像素数据到缓冲区
        for i in 0..n {
            self.buffer[start + i] = pixel[i];
        }

        Ok(())
    }

    /// 计算每行的字节数
    ///
    /// # 返回
    /// * `u32` - 每行的字节数
    pub fn row_size(&self) -> u32 {
        (self.dimensions.0 * self.bits_per_pixel + 7) / 8
    }

    /// 获取样本大小（以位为单位）
    ///
    /// # 返回
    /// * `Result<u16, RasterError>` - 如果所有样本大小相同，返回样本大小；否则返回错误
    pub fn sample_size(&self) -> Result<u16, RasterError> {
        // 检查每个样本的位数是否为空
        if self.bits_per_sample.is_empty() {
            return Err(RasterError::NotSupported("每个样本的位数为空".into()));
        }

        // 获取第一个样本的位数
        let first = self.bits_per_sample[0];

        // 检查所有样本的位数是否相同
        if self.bits_per_sample.iter().all(|v| *v == first) {
            // 如果所有样本位数相同，返回该位数
            Ok(first)
        } else {
            // 如果样本位数不同，返回错误
            Err(RasterError::NotSupported(
                "调整大小滤镜最大值仅适用于每个样本8位或16位的情况".into(),
            ))
        }
    }
}

/// 实现 Display trait，用于格式化输出 Raster 信息
impl Display for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Raster({}x{}, {:?}, {:?}, {}字节, {:?}字节序)",
            self.dimensions.0,
            self.dimensions.1,
            self.bits_per_sample,
            self.interpretation,
            self.buffer.len(),
            self.endian
        )
    }
}
