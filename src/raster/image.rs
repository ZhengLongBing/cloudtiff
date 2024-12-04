//! 本模块提供了 Raster 与 image 库的 DynamicImage 之间的转换功能。
//!
//! 主要包含以下功能:
//! - 从 Raster 获取 RGBA 像素值
//! - 将 Raster 转换为 DynamicImage
//! - 将 DynamicImage 转换为 Raster

#![cfg(feature = "image")]

use super::{
    photometrics::PhotometricInterpretation as Style, ExtraSamples, RasterError, SampleFormat,
};
use crate::raster::Raster;
use crate::tiff::Endian;
use image::{DynamicImage, ImageBuffer, Rgba};

impl Raster {
    /// 获取指定坐标的 RGBA 像素值
    ///
    /// # 参数
    /// * `x` - 像素的 x 坐标
    /// * `y` - 像素的 y 坐标
    ///
    /// # 返回值
    /// 返回 `Option<Rgba<u8>>`，如果坐标有效且可以转换为 RGBA 格式，则返回 Some(Rgba)，否则返回 None
    pub fn get_pixel_rgba(&self, x: u32, y: u32) -> Option<Rgba<u8>> {
        let p = self.get_pixel(x, y)?;
        Some(match self.bits_per_sample.as_slice() {
            [8] => Rgba([p[0], p[0], p[0], 255]), // 8位灰度图，添加不透明的 alpha 通道
            [8, 8] => Rgba([p[0], p[0], p[0], p[1]]), // 8位灰度图 + alpha
            [8, 8, 8] => Rgba([p[0], p[1], p[2], 255]), // 24位 RGB 图，添加不透明的 alpha 通道
            [8, 8, 8, 8] => Rgba([p[0], p[1], p[2], p[3]]), // 32位 RGBA 图
            [16] => {
                // 16位灰度图，需要进行解码和缩放
                let v: i16 = self.endian.decode([p[0], p[1]]).ok()?;
                let v8 = (v / 10).clamp(0, 255) as u8;
                Rgba([v8, v8, v8, 255])
            }
            _ => return None, // 不支持的位深度
        })
    }
}

impl TryInto<DynamicImage> for Raster {
    type Error = String;

    /// 尝试将 Raster 转换为 DynamicImage
    ///
    /// # 返回值
    /// 成功时返回 `Ok(DynamicImage)`，失败时返回 `Err(String)`
    fn try_into(self) -> Result<DynamicImage, Self::Error> {
        let Raster {
            dimensions: (width, height),
            buffer,
            bits_per_sample,
            interpretation: _,
            endian,
            ..
        } = self;

        // 根据不同的位深度和通道数创建对应的 DynamicImage
        match bits_per_sample.as_slice() {
            [8] => {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageLuma8(ib))
            }
            [8, 8] => {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageLumaA8(ib))
            }
            [16] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageLuma16(ib))
            }),
            [16, 16] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer)
                    .map(|ib| DynamicImage::ImageLumaA16(ib))
            }),
            [8, 8, 8] => {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageRgb8(ib))
            }
            [8, 8, 8, 8] => {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageRgba8(ib))
            }
            [16, 16, 16] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageRgb16(ib))
            }),
            [16, 16, 16, 16] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageRgba16(ib))
            }),
            [32, 32, 32] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer).map(|ib| DynamicImage::ImageRgb32F(ib))
            }),
            [32, 32, 32, 32] => endian.decode_all(&buffer).and_then(|buffer| {
                ImageBuffer::from_raw(width, height, buffer)
                    .map(|ib| DynamicImage::ImageRgba32F(ib))
            }),
            _ => None,
        }
        .ok_or("不支持的图像格式".to_string())
    }
}

impl Raster {
    /// 将 Raster 转换为 DynamicImage
    ///
    /// # 返回值
    /// 成功时返回 `Ok(DynamicImage)`，失败时返回 `Err(String)`
    pub fn into_image(self) -> Result<DynamicImage, String> {
        self.try_into()
    }

    /// 从 DynamicImage 创建 Raster
    ///
    /// # 参数
    /// * `img` - 输入的 DynamicImage
    ///
    /// # 返回值
    /// 成功时返回 `Ok(Raster)`，失败时返回 `Err(RasterError)`
    pub fn from_image(img: &DynamicImage) -> Result<Self, RasterError> {
        let dimensions = (img.width(), img.height());
        let buffer = img.as_bytes().to_vec();
        let endian = if cfg!(target_endian = "big") {
            Endian::Big
        } else {
            Endian::Little
        };

        // 根据不同的 DynamicImage 类型设置对应的参数
        let (interpretation, bits_per_sample, sample_format, extra_samples) = match img {
            DynamicImage::ImageLuma16(_) => (
                Style::BlackIsZero,
                vec![16],
                vec![SampleFormat::Unsigned],
                vec![],
            ),
            DynamicImage::ImageLuma8(_) => (
                Style::BlackIsZero,
                vec![8],
                vec![SampleFormat::Unsigned],
                vec![],
            ),
            DynamicImage::ImageLumaA8(_) => (
                Style::BlackIsZero,
                vec![8, 8],
                vec![SampleFormat::Unsigned; 2],
                vec![ExtraSamples::AssociatedAlpha],
            ),
            DynamicImage::ImageRgb8(_) => (
                Style::RGB,
                vec![8, 8, 8],
                vec![SampleFormat::Unsigned; 3],
                vec![],
            ),
            DynamicImage::ImageRgba8(_) => (
                Style::RGB,
                vec![8, 8, 8, 8],
                vec![SampleFormat::Unsigned; 4],
                vec![ExtraSamples::AssociatedAlpha],
            ),
            DynamicImage::ImageLumaA16(_) => (
                Style::BlackIsZero,
                vec![16, 16],
                vec![SampleFormat::Unsigned; 2],
                vec![ExtraSamples::AssociatedAlpha],
            ),
            DynamicImage::ImageRgb16(_) => (
                Style::RGB,
                vec![16, 16, 16],
                vec![SampleFormat::Unsigned; 3],
                vec![],
            ),
            DynamicImage::ImageRgba16(_) => (
                Style::RGB,
                vec![16, 16, 16, 16],
                vec![SampleFormat::Unsigned; 4],
                vec![ExtraSamples::AssociatedAlpha],
            ),
            DynamicImage::ImageRgb32F(_) => (
                Style::RGB,
                vec![32, 32, 32],
                vec![SampleFormat::Float; 3],
                vec![],
            ),
            DynamicImage::ImageRgba32F(_) => (
                Style::RGB,
                vec![32, 32, 32, 32],
                vec![SampleFormat::Float; 4],
                vec![ExtraSamples::AssociatedAlpha],
            ),
            _ => (
                Style::Unknown,
                vec![8],
                vec![SampleFormat::Unsigned],
                vec![],
            ),
        };

        Self::new(
            dimensions,
            buffer,
            bits_per_sample,
            interpretation,
            sample_format,
            extra_samples,
            endian,
        )
    }
}
