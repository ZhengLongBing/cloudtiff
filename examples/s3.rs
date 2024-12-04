// 确保 's3' 特性被启用，否则编译时会报错
#[cfg(not(feature = "s3"))]
compile_error!("This example requires the 's3' feature");

// 导入必要的库和模块
use aws_config::{self, Region};
use aws_sdk_s3::{config::Config, Client};
use cloudtiff::{CloudTiff, S3Reader};
use image::DynamicImage;
use std::io::{self, Write};
use std::time::Instant;
use tokio;

// 相关文档链接
// https://docs.rs/object_store/0.11.0/object_store/
// https://crates.io/crates/aws-sdk-s3

// 定义常量
const BUCKET_NAME: &str = "sentinel-cogs";
const OBJECT_NAME: &str = "sentinel-s2-l2a-cogs/9/U/WA/2024/8/S2A_9UWA_20240806_0_L2A/TCI.tif";
const OUTPUT_FILE: &str = "data/s3.jpg";
const PREVIEW_MEGAPIXELS: f64 = 1.0;

// 主函数，使用 tokio 运行时
#[tokio::main]
async fn main() {
    println!("Example: cloudtiff async s3");

    // 请求用户同意使用 AWS 凭证
    let consent: &str = "ok";
    print!(
        r#"This example will use your default AWS environmental credentials to make a request. Type "{consent}" to continue: "#
    );
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if input.trim().to_lowercase() != consent {
        println!("Exiting.");
        return;
    }

    // 配置 S3 读取器
    let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let config = Config::new(&sdk_config)
        .to_builder()
        .region(Some(Region::from_static("us-west-2")))
        .build();
    let client = Client::from_conf(config);
    let reader = S3Reader::new(client, BUCKET_NAME, OBJECT_NAME);

    // 使用 S3 读取器读取云端 TIFF 文件
    handler(reader).await;
}

// 处理 S3 读取器的异步函数
async fn handler(mut source: S3Reader) {
    // 打开并索引 COG (Cloud Optimized GeoTIFF) 文件
    let t_cog = Instant::now();
    let cog = CloudTiff::open_from_async_range_reader(&mut source)
        .await
        .unwrap();
    println!("Indexed COG in {}ms", t_cog.elapsed().as_millis());
    println!("{cog}");

    // 渲染预览图像
    let t_preview = Instant::now();
    let preview = cog
        .renderer()
        .with_mp_limit(PREVIEW_MEGAPIXELS)
        .with_async_range_reader(source)
        .render_async()
        .await
        .unwrap();

    println!(
        "Got preview in {:.6} seconds",
        t_preview.elapsed().as_secs_f64()
    );
    println!("{}", preview);

    // 将预览转换为图像并保存
    let img: DynamicImage = preview.try_into().unwrap();
    img.save(OUTPUT_FILE).unwrap();
    println!("Image saved to {OUTPUT_FILE}");
}
