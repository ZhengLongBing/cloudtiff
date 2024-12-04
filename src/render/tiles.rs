//! 瓦片处理模块
//!
//! 本模块提供了从COG图像中读取和处理瓦片数据的功能。
//! 包括同步和异步两种读取方式。

use super::SyncReader;
use crate::cog::Level;
use crate::raster::Raster;
use std::collections::HashMap;
use tracing::*;

/// 瓦片缓存类型,用于存储索引到栅格数据的映射
pub type TileCache = HashMap<usize, Raster>;

use super::util;

/// 同步读取瓦片数据
///
/// # 参数
/// * `reader` - 同步读取器
/// * `level` - COG图像层级
/// * `indices` - 需要读取的瓦片索引列表
///
/// # 返回
/// 返回包含瓦片数据的缓存映射
pub fn get_tiles(reader: &SyncReader, level: &Level, indices: Vec<usize>) -> TileCache {
    // 获取瓦片的位置信息
    let tile_infos = util::tile_info_from_indices(level, indices);

    // 同步读取和解压瓦片数据
    tile_infos
        .into_iter()
        .filter_map(|(index, (start, end))| {
            // 计算瓦片大小并分配缓冲区
            let n = (end - start) as usize;
            let mut buf = vec![0; n];

            // 读取瓦片字节数据
            match reader.0.read_range_exact(start, &mut buf) {
                Ok(_) => {
                    // 从字节数据中解压提取瓦片
                    match level.extract_tile_from_bytes(&buf) {
                        Ok(tile) => Some((index, tile)),
                        Err(e) => {
                            warn!("瓦片解压失败: {e:?}");
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!("瓦片读取失败: {e:?}");
                    None
                }
            }
        })
        .collect()
}

#[cfg(feature = "async")]
pub use not_sync::*;

#[cfg(feature = "async")]
mod not_sync {
    use super::super::AsyncReader;
    use super::*;
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    /// 异步读取瓦片数据
    ///
    /// # 参数
    /// * `reader` - 异步读取器
    /// * `level` - COG图像层级
    /// * `indices` - 需要读取的瓦片索引列表
    ///
    /// # 返回
    /// 返回包含瓦片数据的缓存映射
    pub async fn get_tiles_async(
        reader: &AsyncReader,
        level: &Level,
        indices: Vec<usize>,
    ) -> TileCache {
        // 获取瓦片位置信息
        let tile_infos = util::tile_info_from_indices(level, indices);

        // 使用futures::future::join_all并发执行多个异步任务
        // 每个任务负责读取一个瓦片的字节数据
        let byte_results: Vec<_> = futures::future::join_all(
            tile_infos
                .into_iter()
                // 为每个瓦片信息克隆reader以支持并发
                .map(|info| (info, reader.0.clone()))
                // 将每个瓦片信息转换为异步任务
                .map(|((index, (start, end)), reader_clone)| {
                    tokio::spawn(async move {
                        // 计算需要读取的字节数
                        let n = (end - start) as usize;
                        // 创建缓冲区
                        let mut buf = vec![0; n];
                        // 异步读取指定范围的字节数据
                        reader_clone
                            .read_range_exact_async(start, &mut buf)
                            .await
                            .map(|_| (index, buf))
                    })
                }),
        )
        .await
        // 处理读取结果
        .into_iter()
        .filter_map(|result| match result {
            // 读取成功
            Ok(Ok(tile_bytes)) => Some(tile_bytes),
            // 读取字节失败
            Ok(Err(e)) => {
                warn!("瓦片字节读取失败: {e:?}");
                None
            }
            // 任务执行失败
            Err(e) => {
                warn!("瓦片读取任务失败: {e:?}");
                None
            }
        })
        .collect();

        // 使用rayon并行解压瓦片数据
        let tile_results: Vec<_> = byte_results
            .into_iter()
            .map(|(index, bytes)| (level.clone(), index, bytes))
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|(level_clone, index, bytes)| {
                // 尝试从字节数据中提取瓦片
                level_clone
                    .extract_tile_from_bytes(&bytes)
                    .map(|tile| (index, tile))
            })
            .collect();

        // 将解压后的瓦片数据存入缓存
        let mut tile_cache: HashMap<usize, Raster> = HashMap::new();
        for result in tile_results {
            match result {
                Ok((index, tile)) => {
                    // 成功解压的瓦片插入缓存
                    tile_cache.insert(index, tile);
                }
                Err(e) => {
                    // 解压失败时记录警告日志
                    warn!("瓦片解压失败: {e:?}")
                }
            }
        }

        tile_cache
    }
}
