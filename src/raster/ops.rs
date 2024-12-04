//! 栅格图像操作模块
//!
//! 本模块提供了对栅格图像进行各种操作的功能,包括:
//!
//! - 调整图像大小
//! - 裁剪图像
//! - 应用滤镜
//!
//! 主要的操作都是通过 `Raster` 结构体的方法来实现的。
//!
//! # 示例
//!
//! ```
//! use crate::raster::{Raster, ResizeFilter};
//!
//! let raster = Raster::new(...);
//! let resized = raster.resize(800, 600, ResizeFilter::Nearest).unwrap();

use super::{Raster, RasterError};
use crate::Region;

/// 定义调整大小时使用的滤镜类型
#[derive(Debug, Copy, Clone)]
pub enum ResizeFilter {
    /// 最近邻插值
    Nearest,
    /// 最大值滤波
    Maximum,
    /// Catmull-Rom 插值（仅在启用 "image" 特性时可用）
    #[cfg(feature = "image")]
    CatmulRod,
}

impl Raster {
    /// 调整栅格图像的大小
    ///
    /// # 参数
    /// * `width` - 目标宽度
    /// * `height` - 目标高度
    /// * `filter` - 使用的调整大小滤镜
    ///
    /// # 返回
    /// 返回调整大小后的新 Raster 实例，或者在出错时返回 RasterError
    pub fn resize(
        &self,
        width: u32,
        height: u32,
        filter: ResizeFilter,
    ) -> Result<Self, RasterError> {
        // 检查像素是否按字节对齐
        // 如果像素不是按字节对齐的,返回错误
        if self.bits_per_pixel % 8 != 0 {
            return Err(RasterError::NotSupported(format!(
                "像素不是按字节对齐的: {} 位",
                self.bits_per_pixel
            )));
        }

        // 计算每个像素的字节数
        let bytes_per_pixel = (self.bits_per_pixel / 8) as usize;
        // 创建新的缓冲区,用于存储调整大小后的图像数据
        let mut buffer = vec![0; ((width * height) as usize) * bytes_per_pixel];

        // 计算原始图像和目标图像的宽高比
        let full_width = self.dimensions.0 as f32;
        let full_height = self.dimensions.1 as f32;
        let scale = (full_width / width as f32, full_height / height as f32);

        match filter {
            ResizeFilter::Nearest => {
                // 最近邻插值
                // 对每个目标像素进行处理
                for j in 0..height {
                    // 计算源图像中对应的垂直坐标
                    let v = (j as f32 * scale.1) as u32;
                    for i in 0..width {
                        // 计算源图像中对应的水平坐标
                        let u = (i as f32 * scale.0) as u32;
                        // 计算源图像中像素的起始位置
                        let src = (v * self.dimensions.0 + u) as usize * bytes_per_pixel;
                        // 计算目标图像中像素的起始位置
                        let dst = (j * width + i) as usize * bytes_per_pixel;
                        // 将源像素数据复制到目标位置
                        buffer[dst..dst + bytes_per_pixel]
                            .copy_from_slice(&self.buffer[src..src + bytes_per_pixel]);
                    }
                }
            }
            ResizeFilter::Maximum => {
                // 最大值滤波
                // 获取样本大小
                let sample_size = self.sample_size()?;
                // 检查样本大小是否为8位
                if sample_size != 8 {
                    return Err(RasterError::NotSupported(format!(
                        "ResizeFilter::Maximum 不支持样本大小 {sample_size}"
                    )));
                }
                // 获取样本数量
                let samples = self.bits_per_sample.len();
                // 遍历目标图像的每个像素
                for j in 0..height {
                    // 计算源图像中对应的垂直范围
                    let v_start = (j as f32 * scale.1) as u32;
                    let v_end = ((j + 1) as f32 * scale.1) as u32;
                    for i in 0..width {
                        // 计算源图像中对应的水平范围
                        let u_start = (i as f32 * scale.0) as u32;
                        let u_end = ((i + 1) as f32 * scale.0) as u32;
                        // 计算目标像素在缓冲区中的位置
                        let dst = (j * width + i) as usize * bytes_per_pixel;
                        // 对每个样本进行处理
                        for s in 0..samples {
                            let mut value: u8 = 0;
                            // 在源图像的对应区域内查找最大值
                            for v in v_start..v_end {
                                for u in u_start..u_end {
                                    let src =
                                        (v * self.dimensions.0 + u) as usize * bytes_per_pixel;
                                    value = value.max(self.buffer[src + s]);
                                }
                            }
                            // 将最大值赋给目标像素
                            buffer[dst + s] = value;
                        }
                    }
                }
            }
            #[cfg(feature = "image")]
            ResizeFilter::CatmulRod => {
                // Catmull-Rom 插值（需要 "image" 特性）
                // 将当前 Raster 实例转换为 DynamicImage
                let img = match self.clone().into_image() {
                    Ok(img) => img,
                    Err(e) => {
                        return Err(RasterError::NotSupported(format!(
                            "无法转换为 DynamicImage，请使用其他 ResizeFilter: {e}"
                        )))
                    }
                };
                // 使用 Catmull-Rom 算法调整图像大小
                let img_resized = img.resize(width, height, image::imageops::CatmullRom);
                // 将调整大小后的 DynamicImage 转换回 Raster 实例
                return Raster::from_image(&img_resized);
            }
        }

        // 创建新的 Raster 实例
        Self::new(
            (width, height),
            buffer,
            self.bits_per_sample.clone(),
            self.interpretation,
            self.sample_format.clone(),
            self.extra_samples.clone(),
            self.endian,
        )
    }

    /// 从栅格图像中提取指定区域
    ///
    /// # 参数
    /// * `region` - 要提取的区域
    ///
    /// # 返回
    /// 返回包含指定区域的新 Raster 实例，或者在出错时返回 RasterError
    pub fn get_region(&self, region: Region<u32>) -> Result<Self, RasterError> {
        // 检查像素是否按字节对齐
        if self.bits_per_pixel % 8 != 0 {
            return Err(RasterError::NotSupported(format!(
                "像素不是按字节对齐的: {} 位",
                self.bits_per_pixel
            )));
        }

        // 计算每个像素的字节数
        let bytes_per_pixel = (self.bits_per_pixel / 8) as usize;

        // 计算提取区域的宽度和高度
        let width = region.x.range();
        let height = region.y.range();

        // 创建新的缓冲区来存储提取的区域数据
        let mut buffer = vec![0; ((width * height) as usize) * bytes_per_pixel];

        // 复制指定区域的像素数据
        for j in region.y.min..region.y.max.min(self.dimensions.1 - 1) {
            for i in region.x.min..region.x.max.min(self.dimensions.0 - 1) {
                // 计算源图像中像素的起始位置
                let src = (j * self.dimensions.0 + i) as usize * bytes_per_pixel;

                // 计算目标缓冲区中像素的起始位置
                let dst =
                    ((j - region.y.min) * width + i - region.x.min) as usize * bytes_per_pixel;

                // 获取源图像中的像素数据
                let pixel = &self.buffer[src..src + bytes_per_pixel];

                // 将像素数据复制到目标缓冲区
                buffer[dst..dst + bytes_per_pixel].copy_from_slice(pixel);
            }
        }

        // 创建新的 Raster 实例
        Self::new(
            (width, height),
            buffer,
            self.bits_per_sample.clone(),
            self.interpretation,
            self.sample_format.clone(),
            self.extra_samples.clone(),
            self.endian,
        )
    }
}
