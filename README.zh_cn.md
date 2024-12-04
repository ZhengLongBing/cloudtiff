# cloudtiff

一个用于 Rust 的云优化地理 TIFF (Cloud Optimized GeoTIFF) 库

### 目标

* 专注于 COG
* 高性能
* 稳健的读取器
* 准确的写入器
* 纯 Rust 实现

### 功能特性

- [x] 无需提取的 TIFF 解码
- [x] 瓦片提取和解压缩
- [x] 从标签获取地理参考信息 (proj4rs)
- [x] 瓦片重新渲染 (WMTS)
- [x] 编码
- [x] 集成 S3 和 HTTP

### 限制

* 预测器仅支持无预测或 8 位水平预测
* 解压缩仅支持无压缩、Lzw 或 Deflate

## 使用方法

```rs
use cloudtiff::CloudTiff;
use image::DynamicImage;
use std::fs::File;
use std::io::BufReader;

fn save_preview(file: File) {
    let reader = &mut BufReader::new(file);
    let cog = CloudTiff::open(reader).unwrap();

    let preview = cog.render_image_with_mp_limit(reader, 1.0).unwrap();

    let img: DynamicImage = preview.try_into().unwrap();
    img.save("preview.jpg").unwrap();
}
```

## 开发

### 环境配置

获取示例数据：
```
mkdir data
aws s3 cp --no-sign-request s3://sentinel-cogs/sentinel-s2-l2a-cogs/9/U/WA/2024/8/S2A_9UWA_20240806_0_L2A/TCI.tif data/sample.tif
```

运行示例：
```
cargo run --example wmts
```

### 设计原则
* 与集成方式无关的库。专注于编码和解码，而不是读写
* 示例展示特定集成用法
* 异步和多线程作为可选功能
* 专注于 COG，不实现完整的 GeoTIFF 或 TIFF 格式
* 无冗余，依赖项也必须保持专注
* 仅使用 Rust 依赖

### 参考资料
[TIFF 6.0 规范](https://download.osgeo.org/geotiff/spec/tiff6.pdf)  
[BigTIFF 规范](https://web.archive.org/web/20240622111852/https://www.awaresystems.be/imaging/tiff/bigtiff.html)  
[OGC GeoTIFF 标准](https://docs.ogc.org/is/19-008r4/19-008r4.html)  
[GeoTIFF 论文](https://www.geospatialworld.net/wp-content/uploads/images/pdf/117.pdf)  
[Cloud Optimized GeoTIFF 规范](https://github.com/cogeotiff/cog-spec/blob/master/spec.md)  
[COG 规范文章](https://cogeotiff.github.io/rio-cogeo/Is_it_a_COG/)  
[COG 介绍文章](https://developers.planet.com/docs/planetschool/an-introduction-to-cloud-optimized-geotiffs-cogs-part-1-overview/)  
[COG 使用文章](https://medium.com/@_VincentS_/do-you-really-want-people-using-your-data-ec94cd94dc3f)  
[AWS 上的 COG 文章](https://opengislab.com/blog/2021/4/17/hosting-and-accessing-cloud-optimized-geotiffs-on-aws-s3)  

### 示例数据
[AWS Sentinel-2](https://registry.opendata.aws/sentinel-2-l2a-cogs/)  
[NASA EarthData](https://www.earthdata.nasa.gov/engage/cloud-optimized-geotiffs)  
[rio-tiler](https://github.com/cogeotiff/rio-tiler/tree/6.4.0/tests/fixtures)  
[OpenAerialMap](https://map.openaerialmap.org/)

### 相关库
[cog3pio](https://github.com/weiji14/cog3pio) (仅读取)  
[tiff](https://crates.io/crates/tiff) (解码对 COG 不够优化)  
[geo](https://crates.io/crates/geo) (坐标转换和投影)  
[geotiff](https://crates.io/crates/geotiff) (解码对 COG 不够优化)  
[geotiff-rs](https://github.com/fizyk20/geotiff-rs)  
[gdal](https://crates.io/crates/gdal) (GDAL 的 Rust 绑定)  

### 工具
[QGIS](https://cogeo.org/qgis-tutorial.html)  
[GDAL](https://gdal.org/en/latest/drivers/raster/cog.html)  
[rio-cogeo](https://github.com/cogeotiff/rio-cogeo)  
[rio-tiler](https://github.com/cogeotiff/rio-tiler)  