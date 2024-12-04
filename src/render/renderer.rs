//! 渲染器模块
//!
//! 本模块提供了对云优化地理影像(COG)进行渲染的核心功能实现。
//! 包括同步和异步渲染、图像裁剪和投影转换等功能。

use super::CloudTiffResult;
use super::{tiles, util};
use super::{RenderBuilder, RenderRegion, SyncReader};
use crate::cog::Level;
use crate::raster::Raster;
use crate::{Region, UnitFloat};
use std::collections::HashMap;

impl<'a> RenderBuilder<'a, SyncReader> {
    /// 执行同步渲染操作
    ///
    /// 根据配置的渲染区域类型(输入裁剪或输出区域)执行相应的渲染逻辑
    pub fn render(&self) -> CloudTiffResult<Raster> {
        let dimensions = self.resolution;
        match self.region {
            // 处理输入裁剪模式
            RenderRegion::InputCrop(crop) => {
                // 确定合适的渲染层级
                let level = util::render_level_from_crop(self.cog, &crop, &dimensions);
                // 获取裁剪区域内的瓦片索引
                let indices = level.tile_indices_within_image_crop(crop);
                // 读取所需瓦片数据
                let tile_cache = tiles::get_tiles(&self.reader, level, indices);
                // 渲染裁剪后的图像
                Ok(render_image_crop_from_tile_cache(
                    &tile_cache,
                    level,
                    &crop,
                    &dimensions,
                ))
            }
            // 处理输出区域模式(需要投影转换)
            RenderRegion::OutputRegion((epsg, region)) => {
                // 确定合适的渲染层级
                let level = util::render_level_from_region(self.cog, epsg, &region, &dimensions)?;
                // 计算像素映射关系
                let pixel_map = util::project_pixel_map(
                    level,
                    &self.input_projection,
                    epsg,
                    &region,
                    &dimensions,
                )?;
                // 获取需要的瓦片索引
                let indices = pixel_map.iter().map(|(i, _)| *i).collect();
                // 读取瓦片数据
                let tile_cache = tiles::get_tiles(&self.reader, level, indices);
                // 根据像素映射渲染图像
                render_pixel_map(&pixel_map, level, &tile_cache, &dimensions)
            }
        }
    }
}

#[cfg(feature = "async")]
mod not_sync {
    use super::super::AsyncReader;
    use super::*;

    impl<'a> RenderBuilder<'a, AsyncReader> {
        /// 执行异步渲染操作
        ///
        /// 与同步渲染逻辑相同,但使用异步IO操作
        pub async fn render_async(&'a self) -> CloudTiffResult<Raster> {
            let dimensions = self.resolution;
            match self.region {
                RenderRegion::InputCrop(crop) => {
                    let level = util::render_level_from_crop(self.cog, &crop, &dimensions);
                    let indices = level.tile_indices_within_image_crop(crop);
                    let tile_cache: HashMap<usize, Raster> =
                        tiles::get_tiles_async(&self.reader, level, indices).await;
                    Ok(render_image_crop_from_tile_cache(
                        &tile_cache,
                        level,
                        &crop,
                        &dimensions,
                    ))
                }
                RenderRegion::OutputRegion((epsg, region)) => {
                    let level =
                        util::render_level_from_region(self.cog, epsg, &region, &dimensions)?;
                    let pixel_map = util::project_pixel_map(
                        level,
                        &self.input_projection,
                        epsg,
                        &region,
                        &dimensions,
                    )?;
                    let indices = pixel_map.iter().map(|(i, _)| *i).collect();
                    let tile_cache = tiles::get_tiles_async(&self.reader, level, indices).await;
                    render_pixel_map(&pixel_map, level, &tile_cache, &dimensions)
                }
            }
        }
    }
}

/// 从瓦片缓存中渲染裁剪后的图像
///
/// # 参数
/// * `tile_cache` - 瓦片数据缓存
/// * `level` - 渲染使用的图像层级
/// * `crop` - 裁剪区域
/// * `dimensions` - 输出图像尺寸
pub fn render_image_crop_from_tile_cache(
    tile_cache: &HashMap<usize, Raster>,
    level: &Level,
    crop: &Region<UnitFloat>,
    dimensions: &(u32, u32),
) -> Raster {
    // 创建空白输出栅格
    let mut render_raster = Raster::blank(
        dimensions.clone(),
        level.bits_per_sample.clone(),
        level.interpretation,
        level.sample_format.clone(),
        level.extra_samples.clone(),
        level.endian,
    );

    // 计算采样步长
    let dxdi = crop.x.range().as_f64() / dimensions.0 as f64;
    let dydj = crop.y.range().as_f64() / dimensions.1 as f64;

    // 遍历输出像素
    let mut y = crop.y.min.as_f64();
    for j in 0..dimensions.1 {
        let mut x = crop.x.min.as_f64();
        for i in 0..dimensions.0 {
            // 计算对应的瓦片索引和瓦片内坐标
            if let Ok((tile_index, u, v)) = level.index_from_image_coords(x, y) {
                if let Some(tile) = tile_cache.get(&tile_index) {
                    if let Some(pixel) = tile.get_pixel(u as u32, v as u32) {
                        let _ = render_raster.put_pixel(i, j, pixel);
                    }
                }
            }
            x += dxdi;
        }
        y += dydj;
    }
    render_raster
}

/// 根据像素映射关系渲染图像
///
/// # 参数
/// * `pixel_map` - 源图像到目标图像的像素映射关系
/// * `level` - 渲染使用的图像层级
/// * `tile_cache` - 瓦片数据缓存
/// * `dimensions` - 输出图像尺寸
fn render_pixel_map(
    pixel_map: &util::PixelMap,
    level: &Level,
    tile_cache: &HashMap<usize, Raster>,
    dimensions: &(u32, u32),
) -> CloudTiffResult<Raster> {
    // 创建空白输出栅格
    let mut render_raster = Raster::blank(
        dimensions.clone(),
        level.bits_per_sample.clone(),
        level.interpretation,
        level.sample_format.clone(),
        level.extra_samples.clone(),
        level.endian,
    );

    // 遍历像素映射进行渲染
    for (tile_index, tile_pixel_map) in pixel_map.iter() {
        if let Some(tile) = tile_cache.get(tile_index) {
            for (from, to) in tile_pixel_map {
                // TODO: 实现更多插值方法,目前仅使用最近邻插值
                if let Some(pixel) = tile.get_pixel(from.0 as u32, from.1 as u32) {
                    let _ = render_raster.put_pixel(to.0, to.1, pixel);
                }
            }
        }
    }
    Ok(render_raster)
}
