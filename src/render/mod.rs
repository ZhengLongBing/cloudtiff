//! 渲染模块
//!
//! 本模块提供了对云优化地理影像(COG)进行渲染的功能。主要包括:
//! - 同步和异步读取器的抽象
//! - 渲染构建器用于配置渲染参数
//! - 区域和分辨率控制

use crate::cog::{CloudTiff, CloudTiffResult};
use crate::io::ReadRange;
use crate::projection::Projection;
use crate::{Region, UnitFloat};
use std::io::{Read, Seek};
use std::sync::Mutex;

#[cfg(feature = "async")]
use {
    crate::io::AsyncReadRange,
    std::sync::Arc,
    tokio::io::{AsyncRead, AsyncSeek},
    tokio::sync::Mutex as AsyncMutex,
};

pub mod renderer;
pub mod tiles;
pub mod util;

/// 表示需要读取器的占位符类型
pub struct ReaderRequired;

/// 同步读取器包装类型
pub struct SyncReader(Arc<dyn ReadRange>);

/// 异步读取器包装类型
#[cfg(feature = "async")]
#[derive(Clone)]
pub struct AsyncReader(Arc<dyn AsyncReadRange>);

/// 渲染构建器
///
/// 用于配置和构建渲染操作的参数
#[derive(Debug)]
pub struct RenderBuilder<'a, R> {
    /// COG 影像引用
    pub cog: &'a CloudTiff,
    /// 读取器实例
    pub reader: R,
    /// 输入投影
    pub input_projection: Projection,
    /// 渲染区域
    pub region: RenderRegion,
    /// 输出分辨率
    pub resolution: (u32, u32),
}

/// 渲染区域类型
#[derive(Debug)]
pub enum RenderRegion {
    /// 输入裁剪区域,使用归一化坐标(0-1)
    InputCrop(Region<UnitFloat>),
    /// 输出区域,包含EPSG代码和实际坐标
    OutputRegion((u16, Region<f64>)),
}

impl CloudTiff {
    /// 创建一个新的渲染构建器
    pub fn renderer(&self) -> RenderBuilder<ReaderRequired> {
        RenderBuilder {
            cog: self,
            reader: ReaderRequired,
            input_projection: self.projection.clone(),
            region: RenderRegion::InputCrop(Region::unit()),
            resolution: self.full_dimensions(),
        }
    }
}

impl<'a, S> RenderBuilder<'a, S> {
    /// 设置读取器
    fn set_reader<R>(self, reader: R) -> RenderBuilder<'a, R> {
        let Self {
            cog,
            reader: _,
            input_projection,
            region,
            resolution,
        } = self;
        RenderBuilder {
            cog,
            reader,
            input_projection,
            region,
            resolution,
        }
    }
}

impl<'a> RenderBuilder<'a, ReaderRequired> {
    /// 使用同步读取器
    pub fn with_reader<R: Read + Seek + 'static>(self, reader: R) -> RenderBuilder<'a, SyncReader> {
        self.set_reader(SyncReader(Arc::new(Mutex::new(reader))))
    }

    /// 使用`Arc<Mutex>`包装的同步读取器
    pub fn with_arc_mutex_reader<R: Read + Seek + 'static>(
        self,
        reader: Arc<Mutex<R>>,
    ) -> RenderBuilder<'a, SyncReader> {
        self.set_reader(SyncReader(reader))
    }

    /// 使用实现了ReadRange的读取器
    pub fn with_range_reader<R: ReadRange + 'static>(
        self,
        reader: R,
    ) -> RenderBuilder<'a, SyncReader> {
        self.set_reader(SyncReader(Arc::new(reader)))
    }

    /// 使用异步读取器
    #[cfg(feature = "async")]
    pub fn with_async_reader<R: AsyncRead + AsyncSeek + Send + Sync + Unpin + 'static>(
        self,
        reader: Arc<AsyncMutex<R>>,
    ) -> RenderBuilder<'a, AsyncReader> {
        self.set_reader(AsyncReader(reader))
    }

    /// 使用实现了AsyncReadRange的异步读取器
    #[cfg(feature = "async")]
    pub fn with_async_range_reader<R: AsyncReadRange + 'static>(
        self,
        reader: R,
    ) -> RenderBuilder<'a, AsyncReader> {
        self.set_reader(AsyncReader(Arc::new(reader)))
    }

    /// 使用Arc包装的异步读取器
    #[cfg(feature = "async")]
    pub fn with_async_arc_range_reader<R: AsyncReadRange + 'static>(
        self,
        reader: Arc<R>,
    ) -> RenderBuilder<'a, AsyncReader> {
        self.set_reader(AsyncReader(reader))
    }
}

impl<'a, S> RenderBuilder<'a, S> {
    /// 设置精确的输出分辨率
    pub fn with_exact_resolution(mut self, resolution: (u32, u32)) -> Self {
        self.resolution = resolution;
        self
    }

    /// 根据最大兆像素限制设置分辨率
    pub fn with_mp_limit(mut self, max_megapixels: f64) -> Self {
        self.resolution =
            util::resolution_from_mp_limit(self.cog.full_dimensions(), max_megapixels);
        self
    }

    /// 设置输入裁剪区域
    pub fn of_crop(mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        self.region = RenderRegion::InputCrop(Region::new_saturated(min_x, min_y, max_x, max_y));
        self
    }

    /// 使用经纬度设置输出区域(单位:度)
    pub fn of_output_region_lat_lon_deg(
        self,
        west: f64,
        south: f64,
        north: f64,
        east: f64,
    ) -> Self {
        self.of_output_region(
            4326,
            west.to_radians(),
            south.to_radians(),
            north.to_radians(),
            east.to_radians(),
        )
    }

    /// 设置输出区域
    pub fn of_output_region(
        mut self,
        epsg: u16,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Self {
        self.region = RenderRegion::OutputRegion((epsg, Region::new(min_x, min_y, max_x, max_y)));
        self
    }
}
