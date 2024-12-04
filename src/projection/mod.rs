//! COG 投影模块
//!
//! 本模块提供了 Cloud Optimized GeoTIFF (COG) 的投影转换功能。
//! 主要包含以下功能:
//!
//! - 从 GeoTags 创建投影
//! - 坐标系转换
//! - 边界计算
//! - 投影变换
//!
//! # 待办事项
//! - 验证3D支持
//! - 识别单位(如度与弧度)

use crate::geotags::{GeoKeyId, GeoModel, GeoModelScaled, GeoModelTransformed, GeoTags};
use primatives::{Point2D, Region};
use proj4rs::errors::Error as Proj4Error;
use proj4rs::proj::Proj;
use proj4rs::transform::transform;

pub mod primatives;

/// 投影错误类型
#[derive(Debug)]
pub enum ProjectionError {
    /// 缺少必需的地理键
    MissingGeoKey(GeoKeyId),
    /// Proj4 库错误
    Proj4Error(Proj4Error),
    /// 无效的原点坐标
    InvalidOrigin((f64, f64, f64)),
    /// 无效的缩放比例
    InvalidScale((f64, f64, f64)),
    /// 不支持的模型变换
    UnsupportedModelTransformation,
}

impl From<Proj4Error> for ProjectionError {
    fn from(e: Proj4Error) -> Self {
        ProjectionError::Proj4Error(e)
    }
}

/// 投影结构体
///
/// 包含投影所需的基本参数:
/// - EPSG 代码
/// - proj4rs 投影对象
/// - 原点坐标
/// - 缩放比例
#[derive(Clone, Debug)]
pub struct Projection {
    /// EPSG 坐标系统代码
    pub epsg: u16,
    /// proj4rs 投影对象
    pub proj: Proj,
    /// 原点坐标 (x, y, z)
    pub origin: (f64, f64, f64),
    /// 缩放比例 (x_scale, y_scale, z_scale)
    pub scale: (f64, f64, f64),
}

impl Projection {
    /// 从 GeoTags 创建投影对象
    ///
    /// # 参数
    /// * `geo` - GeoTags 引用
    /// * `dimensions` - 图像尺寸 (宽度, 高度)
    ///
    /// # 返回
    /// * `Result<Self, ProjectionError>` - 成功返回投影对象,失败返回错误
    pub fn from_geo_tags(geo: &GeoTags, dimensions: (u32, u32)) -> Result<Self, ProjectionError> {
        // 从地理标签目录中查找投影坐标系统(ProjectedCSTypeGeoKey)或地理坐标系统(GeographicTypeGeoKey)的EPSG代码
        // 如果找不到任何一个坐标系统代码,则返回错误
        let Some(epsg) = geo
            .directory
            .keys
            .iter()
            .find(|key| {
                matches!(
                    key.id(),
                    Some(GeoKeyId::ProjectedCSTypeGeoKey | GeoKeyId::GeographicTypeGeoKey)
                )
            })
            .and_then(|key| key.value.as_number())
        else {
            return Err(ProjectionError::MissingGeoKey(
                GeoKeyId::ProjectedCSTypeGeoKey,
            ));
        };

        // 使用EPSG代码创建proj4rs投影对象
        // 如果创建失败则返回Proj4Error错误
        let proj = Proj::from_epsg_code(epsg)?;

        // 根据EPSG代码和地理角度单位计算单位增益
        // 当EPSG为4326(WGS84)且角度单位为9102(度)时,需要转换为弧度
        // 其他情况下单位增益为1.0
        let unit_gain = match (
            epsg,
            geo.directory
                .keys
                .iter()
                .find(|key| matches!(key.id(), Some(GeoKeyId::GeogAngularUnitsGeoKey)))
                .and_then(|key| key.value.as_number()),
        ) {
            (4326, Some(9102)) => 1_f64.to_radians(), // WGS84 + 度,转换为弧度
            _ => 1.0,                                 // 其他情况保持原值
        };

        // 根据地理模型类型获取定位点和像素比例
        // 如果是变换模型(Transformed)则返回不支持错误
        // 如果是缩放模型(Scaled)则提取定位点和像素比例
        let (tiepoint, pixel_scale) = match geo.model {
            GeoModel::Transformed(GeoModelTransformed {
                transformation: _,
                tiepoint: _,
            }) => return Err(ProjectionError::UnsupportedModelTransformation),
            GeoModel::Scaled(GeoModelScaled {
                tiepoint,
                pixel_scale,
            }) => (tiepoint, pixel_scale),
        };

        // 从定位点计算投影原点坐标
        // 将定位点的X、Y、Z坐标乘以单位增益得到实际坐标值
        // 如果任一坐标值不是有限数则返回无效原点错误
        let origin = (
            tiepoint[3] * unit_gain, // X坐标
            tiepoint[4] * unit_gain, // Y坐标
            tiepoint[5] * unit_gain, // Z坐标
        );
        if !origin.0.is_finite() || !origin.1.is_finite() || !origin.2.is_finite() {
            return Err(ProjectionError::InvalidOrigin(origin));
        }

        // 计算像素比例
        // 将原始像素比例乘以单位增益得到实际像素比例
        let pixel_scale = (
            pixel_scale[0] * unit_gain,
            pixel_scale[1] * unit_gain,
            pixel_scale[2] * unit_gain,
        );

        // 检查像素比例的有效性
        // 如果X或Y方向的比例不是正常数值,则返回无效比例错误
        if !pixel_scale.0.is_normal() || !pixel_scale.1.is_normal() {
            return Err(ProjectionError::InvalidScale(pixel_scale));
        }

        // 计算总体缩放比例
        // X和Y方向的总体比例为像素比例乘以对应维度
        // Z方向保持原始像素比例不变
        let scale = (
            pixel_scale.0 * dimensions.0 as f64,
            pixel_scale.1 * dimensions.1 as f64,
            pixel_scale.2,
        );

        Ok(Self {
            epsg,
            proj,
            origin,
            scale,
        })
    }

    /// 从经纬度(度)转换到投影坐标
    pub fn transform_from_lat_lon_deg(
        &self,
        lat: f64,
        lon: f64,
    ) -> Result<(f64, f64), ProjectionError> {
        // 将经纬度(度)转换为弧度
        // 调用 transform_from 方法进行坐标转换
        // 4326 是 WGS84 经纬度坐标系的 EPSG 代码
        // 忽略高度值(z坐标)
        // 返回转换后的 x 和 y 坐标
        let (x, y, _) = self.transform_from(lon.to_radians(), lat.to_radians(), 0.0, 4326)?;
        Ok((x, y))
    }

    /// 从投影坐标转换到经纬度(度)
    pub fn transform_into_lat_lon_deg(
        &self,
        x: f64,
        y: f64,
    ) -> Result<(f64, f64), ProjectionError> {
        // 从投影坐标转换到经纬度坐标系(EPSG:4326)
        // 注意:transform_from方法返回的是(经度,纬度,高度)
        // 我们只需要经度和纬度,并将其转换为度数
        let (lon, lat, _) = self.transform_from(x, y, 0.0, 4326)?;
        // 返回(纬度,经度)对,并将弧度转换为度数
        Ok((lat.to_degrees(), lon.to_degrees()))
    }

    /// 从指定 EPSG 坐标系转换到当前投影
    pub fn transform_from(
        &self,
        x: f64,
        y: f64,
        z: f64,
        epsg: u16,
    ) -> Result<(f64, f64, f64), ProjectionError> {
        // 创建一个包含输入坐标的点
        let mut point = (x, y, z);

        // 根据给定的EPSG代码创建源投影
        let from = Proj::from_epsg_code(epsg)?;

        // 执行从源投影到目标投影的坐标转换
        transform(&from, &self.proj, &mut point)?;

        // 计算转换后坐标相对于原点的偏移量,并根据比例因子进行缩放
        let u = (point.0 - self.origin.0) / self.scale.0;
        let v = (self.origin.1 - point.1) / self.scale.1; // 注意y轴方向的反转
        let w = point.2 - self.origin.2;

        // 返回转换后的坐标
        Ok((u, v, w))
    }

    /// 从指定投影转换到当前投影
    pub fn transform_from_proj(
        &self,
        from: &Proj,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<(f64, f64, f64), ProjectionError> {
        // 创建一个包含输入坐标的点
        let mut point = (x, y, z);

        // 执行从源投影到目标投影的坐标转换
        transform(from, &self.proj, &mut point)?;

        // 计算转换后坐标相对于原点的偏移量,并根据比例因子进行缩放
        let u = (point.0 - self.origin.0) / self.scale.0;
        let v = (self.origin.1 - point.1) / self.scale.1; // 注意y轴方向的反转
        let w = point.2 - self.origin.2;

        // 返回转换后的坐标
        Ok((u, v, w))
    }

    /// 从当前投影转换到指定 EPSG 坐标系
    pub fn transform_into(
        &self,
        u: f64,
        v: f64,
        w: f64,
        epsg: u16,
    ) -> Result<(f64, f64, f64), ProjectionError> {
        // 计算目标坐标系中的 x、y、z 坐标
        let x = self.origin.0 + u * self.scale.0;
        let y = self.origin.1 - v * self.scale.1; // 注意 y 轴方向的反转
        let z = self.origin.2 + w;

        // 创建包含计算后坐标的点
        let mut point = (x, y, z);

        // 根据给定的 EPSG 代码创建目标投影
        let to = Proj::from_epsg_code(epsg)?;

        // 执行从当前投影到目标投影的坐标转换
        transform(&self.proj, &to, &mut point)?;

        // 返回转换后的坐标点
        Ok(point)
    }

    /// 从当前投影转换到指定投影
    pub fn transform_into_proj(
        &self,
        to: &Proj,
        u: f64,
        v: f64,
        w: f64,
    ) -> Result<(f64, f64, f64), ProjectionError> {
        // 计算目标坐标系中的 x、y、z 坐标
        let x = self.origin.0 + u * self.scale.0;
        let y = self.origin.1 - v * self.scale.1; // 注意 y 轴方向的反转
        let z = self.origin.2 + w;

        // 创建包含计算后坐标的点
        let mut point = (x, y, z);
        // 执行从当前投影到目标投影的坐标转换
        transform(&self.proj, &to, &mut point)?;
        // 返回转换后的坐标
        Ok(point)
    }

    /// 获取经纬度边界(度)
    pub fn bounds_lat_lon_deg(&self) -> Result<Region<f64>, ProjectionError> {
        // 获取 EPSG:4326 (WGS84) 坐标系下的边界（弧度）
        let radians = self.bounds(4326);

        // 将弧度转换为度，并创建新的 Region 对象
        Ok(Region::new(
            radians.x.min.to_degrees(), // 最小经度（度）
            radians.y.min.to_degrees(), // 最小纬度（度）
            radians.x.max.to_degrees(), // 最大经度（度）
            radians.y.max.to_degrees(), // 最大纬度（度）
        ))
    }

    /// 获取指定 EPSG 坐标系下的边界
    pub fn bounds(&self, epsg: u16) -> Region<f64> {
        // 采样8个点来确定边界
        // 这些点分别位于图像的四个角落和四条边的中点
        // 通过遍历这些点并将它们投影到目标坐标系中
        // 我们可以得到一个近似的边界区域
        vec![
            [0.0, 0.0], // 左上角
            [0.5, 0.0], // 上边中点
            [1.0, 0.0], // 右上角
            [1.0, 0.5], // 右边中点
            [1.0, 1.0], // 右下角
            [0.5, 1.0], // 下边中点
            [0.0, 1.0], // 左下角
            [0.0, 0.5], // 左边中点
        ]
        .into_iter()
        .fold(
            Region::new(f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |region, [u, v]| {
                // 尝试将每个点转换到目标坐标系
                if let Ok((x, y, _)) = self.transform_into(u, v, 0.0, epsg) {
                    // 如果转换成功，则扩展边界区域
                    region.extend(&Point2D { x, y })
                } else {
                    // 如果转换失败，保持原有边界不变
                    region
                }
            },
        )
    }

    /// 获取指定投影下的边界
    pub fn bounds_in_proj(&self, proj: &Proj) -> Result<Region<f64>, ProjectionError> {
        // 获取左上角坐标
        let (left, top, _) = self.transform_into_proj(&proj, 0.0, 0.0, 0.0)?;
        // 获取右下角坐标
        let (right, bottom, _) = self.transform_into_proj(&proj, 1.0, 1.0, 0.0)?;
        // 创建并返回包含边界的 Region 对象
        Ok(Region::new(left, bottom, right, top))
    }
}
