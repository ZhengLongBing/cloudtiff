// 如果没有启用 "http" 特性，编译时会报错
#[cfg(not(feature = "http"))]
compile_error!("This example requires the 'http' feature");

// 导入必要的库和模块
use cloudtiff::{CloudTiff, HttpReader};
use image::DynamicImage;
use std::time::Instant;
use tokio;

// 定义常量
// COG文件的URL地址
const URL: &str = "http://sentinel-cogs.s3.amazonaws.com/sentinel-s2-l2a-cogs/9/U/WA/2024/8/S2A_9UWA_20240806_0_L2A/TCI.tif";
// 输出文件的路径
const OUTPUT_FILE: &str = "data/http.jpg";
// 预览图像的目标大小（以百万像素为单位）
const PREVIEW_MEGAPIXELS: f64 = 1.0;

// 使用tokio运行时的主函数
#[tokio::main]
async fn main() {
    println!("Example: cloudtiff async http");

    // 调用异步处理函数
    handler().await;
}

// 异步处理函数
async fn handler() {
    // COG文件处理
    let t_cog = Instant::now(); // 开始计时
                                // 创建HTTP读取器
    let mut http_reader = HttpReader::new(URL).unwrap();
    // 异步打开COG文件
    let cog = CloudTiff::open_async(&mut http_reader).await.unwrap();
    // 打印索引COG文件所需的时间
    println!("Indexed COG in {}ms", t_cog.elapsed().as_millis());
    // 打印COG文件的信息
    println!("{cog}");

    // 生成预览图像
    let t_preview = Instant::now(); // 开始计时
                                    // 使用COG渲染器生成预览图像
    let preview = cog
        .renderer()
        .with_mp_limit(PREVIEW_MEGAPIXELS) // 设置预览图像的大小限制
        .with_async_range_reader(http_reader) // 设置异步范围读取器
        .render_async() // 异步渲染
        .await
        .unwrap();
    // 打印生成预览图像所需的时间
    println!(
        "Got preview in {:.6} seconds",
        t_preview.elapsed().as_secs_f64()
    );
    // 打印预览图像的信息
    println!("{}", preview);

    // 保存图像
    // 将预览图像转换为DynamicImage类型
    let img: DynamicImage = preview.try_into().unwrap();
    // 保存图像到指定的输出文件
    img.save(OUTPUT_FILE).unwrap();
    // 打印保存成功的消息
    println!("Image saved to {OUTPUT_FILE}");
}
