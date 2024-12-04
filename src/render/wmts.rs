//! WMTS (Web Map Tile Service) 相关功能模块
//! 
//! 本模块提供了处理 WMTS 瓦片坐标系统的功能,包括坐标转换和瓦片索引计算等。

use crate::{Point2D, Region};
use std::f64::consts::{PI, TAU};

/// Web墨卡托投影支持的最大纬度(度)
pub const MAX_LAT_DEG: f64 = 85.06;
/// Web墨卡托投影支持的最小纬度(度) 
pub const MIN_LAT_DEG: f64 = -85.06;

/// 计算给定边界范围内的所有瓦片索引
///
/// # 参数
/// * `bounds_lat_lon_deg` - 经纬度边界范围(度)
/// * `dimensions` - 输出图像尺寸 (宽度,高度)
/// * `tile_dim` - 瓦片尺寸 (宽度,高度)
///
/// # 返回值
/// 返回包含所有瓦片索引的向量,每个索引为(x, y, z)元组
pub fn tile_tree_indices(
    bounds_lat_lon_deg: Region<f64>,
    dimensions: (u32, u32),
    tile_dim: (u32, u32),
) -> Vec<(u32, u32, u32)> {
    let mut tree = vec![];
    // 获取WMTS边界和缩放级别范围
    let (bounds, (min_z, max_z)) = bounds_wmts(bounds_lat_lon_deg, dimensions, tile_dim);

    // 遍历每个缩放级别
    for z in min_z..=max_z {
        // 计算当前缩放级别的瓦片边界
        let tile_bounds = bounds * 2_f64.powi(z as i32);
        // 遍历y轴瓦片索引
        for y in tile_bounds.y.min.floor() as u32..tile_bounds.y.max.ceil() as u32 {
            // 遍历x轴瓦片索引
            for x in tile_bounds.x.min.floor() as u32..tile_bounds.x.max.ceil() as u32 {
                tree.push((x, y, z));
            }
        }
    }
    tree
}

/// 计算WMTS边界和缩放级别范围
///
/// # 参数
/// * `bounds_lat_lon_deg` - 经纬度边界范围(度)
/// * `dimensions` - 输出图像尺寸
/// * `tile_dim` - 瓦片尺寸
///
/// # 返回值
/// 返回元组 (缩放级别0时的边界, (最小缩放级别, 最大缩放级别))
pub fn bounds_wmts(
    bounds_lat_lon_deg: Region<f64>,
    dimensions: (u32, u32),
    tile_dim: (u32, u32),
) -> (Region<f64>, (u32, u32)) {
    // 获取输入的经纬度边界
    let bounds = bounds_lat_lon_deg;

    // 计算缩放级别0时的边界
    // 限制纬度范围在Web墨卡托投影支持的范围内
    let max_lat = bounds.y.max.clamp(MIN_LAT_DEG, MAX_LAT_DEG);
    let min_lat = bounds.y.min.clamp(MIN_LAT_DEG, MAX_LAT_DEG);
    // 定义西北角和东南角坐标点
    let north_west = Point2D {
        x: bounds.x.min,
        y: bounds.y.max,
    };
    let south_east = Point2D {
        x: bounds.x.max,
        y: bounds.y.min,
    };
    // 将经纬度坐标转换为缩放级别0的瓦片索引
    let (min_x, min_y, _) = lat_lon_deg_to_tile_index(north_west, 0.0);
    let (max_x, max_y, _) = lat_lon_deg_to_tile_index(south_east, 0.0);
    // 创建缩放级别0的边界区域
    let z0_bounds = Region::new(min_x, min_y, max_x, max_y);

    // 计算最小缩放级别(使边界能容纳在一个瓦片内)
    // 取经度和纬度方向上所需的最小缩放级别
    let mut min_z = (360.0 / bounds.x.range())
        .min((MAX_LAT_DEG - MIN_LAT_DEG) / (max_lat - min_lat))
        .log2()
        .floor() as u32;
    // 计算最小缩放级别下的边界
    let z_min_bounds = z0_bounds * 2_f64.powi(min_z as i32);
    // 如果边界跨越多个瓦片,则减小缩放级别
    if (z_min_bounds.x.min.floor() != z_min_bounds.x.max.floor())
        || (z_min_bounds.y.min.floor() != z_min_bounds.y.max.floor())
    {
        min_z -= 1;
    }

    // 计算最大缩放级别(瓦片分辨率>=原始分辨率)
    // TODO: 此处假设输入投影与WGS84对齐
    // 计算输入图像的分辨率
    let x_resolution = bounds.x.range() / dimensions.0 as f64;
    let y_resolution = bounds.y.range() / dimensions.1 as f64;
    // 计算缩放级别0的瓦片分辨率
    let z0_x_resolution = 360.0 / tile_dim.0 as f64;
    let z0_y_resolution = (MAX_LAT_DEG - MIN_LAT_DEG) / tile_dim.1 as f64;
    // 计算最大缩放级别
    let max_z = (z0_x_resolution / x_resolution)
        .max(z0_y_resolution / y_resolution)
        .log2()
        .ceil() as u32;

    // 返回缩放级别0的边界和缩放级别范围
    (z0_bounds, (min_z, max_z))
}

/// 计算瓦片的经纬度边界
///
/// # 参数
/// * `x` - 瓦片X索引
/// * `y` - 瓦片Y索引
/// * `z` - 缩放级别
///
/// # 返回值
/// 返回瓦片的经纬度边界区域,如果索引无效则返回None
pub fn tile_bounds_lat_lon_deg(x: u32, y: u32, z: u32) -> Option<Region<f64>> {
    // 计算瓦片左上角(西北)的经纬度坐标
    let nw = tile_index_to_lat_lon_deg(x as f64, y as f64, z as f64)?;
    // 计算瓦片右下角(东南)的经纬度坐标
    let se = tile_index_to_lat_lon_deg((x + 1) as f64, (y + 1) as f64, z as f64)?;
    // 创建并返回表示瓦片边界的Region对象
    // 注意：经度从西到东增加(nw.x到se.x)，纬度从北到南减少(nw.y到se.y)
    Some(Region::new(nw.x, se.y, se.x, nw.y))
}

/// 将瓦片索引转换为经纬度坐标
///
/// # 参数
/// * `x` - 瓦片X坐标
/// * `y` - 瓦片Y坐标  
/// * `z` - 缩放级别
///
/// # 返回值
/// 返回经纬度坐标点,如果索引无效则返回None
pub fn tile_index_to_lat_lon_deg(x: f64, y: f64, z: f64) -> Option<Point2D<f64>> {
    let n = 2.0_f64.powf(z);
    // 验证索引是否有效
    if x < 0.0 || x / n > 1.0 || y < 0.0 || y / n > 1.0 || z < 0.0 {
        return None;
    }
    // 计算经度
    let lon = x * TAU / n - PI;
    // 计算纬度
    let var = PI * (1.0 - 2.0 * y / n);
    let lat = (0.5 * ((var).exp() - (-var).exp())).atan();
    // 转换为度数并返回
    Some(Point2D {
        x: lon.to_degrees(),
        y: lat.to_degrees(),
    })
}

/// 将经纬度坐标转换为瓦片索引
///
/// # 参数
/// * `point` - 经纬度坐标点
/// * `z` - 缩放级别
///
/// # 返回值
/// 返回瓦片索引元组 (x, y, z)
pub fn lat_lon_deg_to_tile_index(point: Point2D<f64>, z: f64) -> (f64, f64, f64) {
    let n = 2.0_f64.powf(z);
    // 将经纬度转换为弧度
    let lon = point.x.to_radians();
    let lat = point.y.to_radians();
    // 计算瓦片X坐标
    let x = n * (lon + PI) / TAU;
    // 计算瓦片Y坐标
    let y = n * (1.0 - ((lat.tan() + 1.0 / lat.cos()).ln() / PI)) / 2.0;
    (x, y, z)
}
