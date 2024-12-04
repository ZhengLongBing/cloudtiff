//! 渲染工具模块
//!
//! 本模块提供了用于渲染云优化地理影像(COG)的工具函数集合。
//! 主要包括:
//! - 渲染层级选择
//! - 像素映射计算
//! - 分辨率控制
//! - 瓦片信息获取

use crate::cog::{CloudTiff, CloudTiffResult, Level};
use crate::projection::{Projection, ProjectionError};
use crate::CloudTiffError;
use crate::{Region, UnitFloat};
use proj4rs::Proj;
use std::collections::HashMap;
use tracing::*;

/// 像素映射类型
///
/// 用于存储瓦片索引到像素坐标的映射关系
/// - Key: 瓦片索引
/// - Value: 向量包含((瓦片x,y), (输出x,y))坐标对
pub type PixelMap = HashMap<usize, Vec<((f64, f64), (u32, u32))>>;

/// 根据裁剪区域选择合适的渲染层级
///
/// # 参数
/// * `cog` - COG图像引用
/// * `crop` - 归一化裁剪区域(0-1)
/// * `dimensions` - 目标输出尺寸
///
/// # 返回
/// 返回最适合的渲染层级
pub fn render_level_from_crop<'a>(
    cog: &'a CloudTiff,
    crop: &Region<UnitFloat>,
    dimensions: &(u32, u32),
) -> &'a Level {
    // 转换裁剪区域为浮点坐标
    let (left, top, right, bottom) = crop.to_f64();

    // 计算最小所需层级尺寸
    let min_level_dims = (
        ((dimensions.0 as f64) / (right - left)).ceil() as u32,
        ((dimensions.1 as f64) / (top - bottom)).ceil() as u32,
    );

    // 选择满足最小尺寸要求的最小层级
    cog.levels
        .iter()
        .rev()
        .find(|level| {
            level.dimensions.0 > min_level_dims.0 && level.dimensions.1 > min_level_dims.1
        })
        .unwrap_or(&cog.levels[0])
}

/// 根据输出区域选择合适的渲染层级
///
/// # 参数
/// * `cog` - COG图像引用
/// * `epsg` - 目标投影EPSG代码
/// * `region` - 目标区域坐标
/// * `dimensions` - 目标输出尺寸
///
/// # 返回
/// 返回最适合的渲染层级,或错误
pub fn render_level_from_region<'a>(
    cog: &'a CloudTiff,
    epsg: u16,
    region: &Region<f64>,
    dimensions: &(u32, u32),
) -> CloudTiffResult<&'a Level> {
    // 转换区域边界到图像投影
    let (left, top, ..) = cog
        .projection
        .transform_from(region.x.min, region.y.min, 0.0, epsg)?;
    let (right, bottom, ..) =
        cog.projection
            .transform_from(region.x.max, region.y.max, 0.0, epsg)?;

    // 计算像素比例
    let pixel_scale_x = (right - left).abs() / dimensions.0 as f64;
    let pixel_scale_y = (top - bottom).abs() / dimensions.1 as f64;
    let min_pixel_scale = pixel_scale_x.min(pixel_scale_y);

    // 选择合适的层级
    let level_scales = cog.pixel_scales();
    let level_index = level_scales
        .iter()
        .enumerate()
        .rev()
        .find(|(_, (level_scale_x, level_scale_y))| {
            level_scale_x.max(*level_scale_y) < min_pixel_scale
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    cog.get_level(level_index)
}

/// 获取瓦片的字节范围信息
///
/// # 参数
/// * `level` - 图像层级
/// * `indices` - 瓦片索引列表
///
/// # 返回
/// 返回包含(索引, (起始字节,结束字节))的向量
pub fn tile_info_from_indices(level: &Level, indices: Vec<usize>) -> Vec<(usize, (u64, u64))> {
    indices
        .into_iter()
        .filter_map(|index| match level.tile_byte_range(index) {
            Ok(range) => Some((index, range)),
            Err(e) => {
                warn!("获取瓦片字节范围失败: {e:?}");
                None
            }
        })
        .collect()
}

/// 根据最大像素限制计算输出分辨率
///
/// # 参数
/// * `max_dimensions` - 最大允许尺寸
/// * `max_megapixels` - 最大允许百万像素数
///
/// # 返回
/// 返回满足限制的输出尺寸
pub fn resolution_from_mp_limit(max_dimensions: (u32, u32), max_megapixels: f64) -> (u32, u32) {
    // 计算宽高比
    let ar = max_dimensions.0 as f64 / max_dimensions.1 as f64;
    // 计算最大像素数
    let max_pixels = max_dimensions.0 as f64 * max_dimensions.1 as f64;
    // 计算高度，取最大百万像素数和最大像素数的较小值，除以宽高比后开平方
    let height = ((max_megapixels * 1e6).min(max_pixels) / ar).sqrt();
    // 根据宽高比计算宽度
    let width = ar * height;
    // 返回计算得到的宽度和高度，转换为u32类型
    (width as u32, height as u32)
}

/// 计算像素映射关系
///
/// # 参数
/// * `level` - 图像层级
/// * `projection` - 输入投影
/// * `epsg` - 输出投影EPSG代码
/// * `region` - 输出区域
/// * `dimensions` - 输出尺寸
///
/// # 返回
/// 返回像素映射或错误
pub fn project_pixel_map(
    level: &Level,
    projection: &Projection,
    epsg: u16,
    region: &Region<f64>,
    dimensions: &(u32, u32),
) -> CloudTiffResult<PixelMap> {
    let mut pixel_map = HashMap::new();
    // 创建输出投影
    let output_proj = Proj::from_epsg_code(epsg).map_err(|e| ProjectionError::from(e))?;

    // 计算像素步长
    let dxdi = region.x.range() / dimensions.0 as f64;
    let dydj = region.y.range() / dimensions.1 as f64;

    // 遍历输出像素
    for j in 0..dimensions.1 {
        for i in 0..dimensions.0 {
            // 计算输出坐标
            let x = region.x.min + dxdi * i as f64;
            let y = region.y.max - dydj * j as f64;

            // 投影转换并记录映射关系
            match projection.transform_from_proj(&output_proj, x, y, 0.0) {
                Ok((u, v, ..)) => {
                    // 尝试获取图像坐标对应的瓦片索引和瓦片内坐标
                    if let Ok((tile_index, tile_x, tile_y)) = level.index_from_image_coords(u, v) {
                        // 获取或创建瓦片的像素映射列表
                        let tile_pixel_map = pixel_map.entry(tile_index).or_insert(vec![]);
                        // 添加瓦片内坐标到输出坐标的映射
                        tile_pixel_map.push(((tile_x, tile_y), (i, j)));
                    }
                }
                Err(e) => warn!("像素转换失败: {e:?}"), // 记录投影转换失败的警告
            }
        }
    }

    // 检查映射结果
    // 如果像素映射为空，说明请求的区域完全超出了图像范围
    if pixel_map.is_empty() {
        return Err(CloudTiffError::RegionOutOfBounds((
            region.as_tuple(),
            projection.bounds_in_proj(&output_proj)?.as_tuple(),
        )));
    }

    // 映射结果有效，返回像素映射
    Ok(pixel_map)
}
