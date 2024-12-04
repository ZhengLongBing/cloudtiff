// 如果没有启用 "async" 特性，编译时会报错
#[cfg(not(feature = "async"))]
compile_error!("This example requires the ['image', 'async'] features");

// 导入必要的库和模块
use cloudtiff::CloudTiff;
use image::DynamicImage;
use std::sync::Arc;
use std::time::Instant;
use tokio;
use tokio::fs::File;
use tokio::sync::Mutex;
use tracing_subscriber;

// 定义常量
const SAMPLE_COG: &str = "data/sample.tif"; // 输入文件路径
const OUTPUT_FILE: &str = "data/async.jpg"; // 输出文件路径
const PREVIEW_MEGAPIXELS: f64 = 1.0; // 预览图像的目标大小（以百万像素为单位）

// 主函数，使用 tokio 运行时
#[tokio::main]
async fn main() {
    println!("Example: cloudtiff async file");

    // 初始化日志记录器
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG) // 设置最大日志级别为 DEBUG
        .with_thread_ids(true) // 在日志中包含线程 ID
        .init();

    // 打开并读取 COG 文件
    let t_cog = Instant::now(); // 开始计时
    let mut file = File::open(SAMPLE_COG).await.unwrap(); // 异步打开文件
    let cog = CloudTiff::open_async(&mut file).await.unwrap(); // 异步读取 COG 文件
    println!(
        "Opened COG in {:.3}ms",
        t_cog.elapsed().as_micros() as f64 / 1000.0 // 计算并打印打开 COG 文件所需的时间
    );

    // 渲染预览图像
    let t0 = Instant::now(); // 开始计时
    let thread_safe_file = Arc::new(Mutex::new(file)); // 创建线程安全的文件句柄
    let preview = cog
        .renderer()
        .with_mp_limit(PREVIEW_MEGAPIXELS) // 设置预览图像的大小限制
        .with_async_reader(thread_safe_file) // 设置异步读取器
        .render_async() // 异步渲染
        .await
        .unwrap();
    println!(
        "Got preview in {:.3}ms",
        t0.elapsed().as_micros() as f64 / 1000.0 // 计算并打印渲染预览图像所需的时间
    );

    // 保存预览图像
    let img: DynamicImage = preview.try_into().unwrap(); // 将预览转换为动态图像
    img.save(OUTPUT_FILE).unwrap(); // 保存图像到文件
    println!("Image saved to {OUTPUT_FILE}"); // 打印保存成功的消息
}
