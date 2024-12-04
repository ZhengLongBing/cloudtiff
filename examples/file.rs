// 如果没有启用 "image" 特性，编译时会报错
#[cfg(not(feature = "image"))]
compile_error!("This example requires the 'image' feature");

// 导入必要的库和模块
use cloudtiff::CloudTiff;
use image::DynamicImage;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

// 定义常量
const SAMPLE_COG: &str = "data/sample.tif"; // 输入COG文件路径
const OUTPUT_FILE: &str = "data/filesystem.jpg"; // 输出JPEG文件路径
const PREVIEW_MEGAPIXELS: f64 = 1.0; // 预览图像的目标大小（以百万像素为单位）

fn main() {
    println!("Example: cloudtiff file");

    // 文件访问
    println!("Opening `{SAMPLE_COG}`");
    let file = File::open(SAMPLE_COG).unwrap(); // 打开COG文件
    let mut reader = BufReader::new(file); // 创建带缓冲的读取器，提高读取效率

    // CloudTiff索引
    let t_cog = Instant::now(); // 开始计时
    let cog = CloudTiff::open(&mut reader).unwrap(); // 打开并索引COG文件
    println!("Indexed COG in {}us", t_cog.elapsed().as_micros()); // 打印索引COG所需的时间（微秒）
    println!("{cog}"); // 打印COG文件的信息

    // 瓦片提取和预览图像生成
    let t_tile = Instant::now(); // 开始计时
    let preview = cog
        .renderer()
        .with_mp_limit(PREVIEW_MEGAPIXELS) // 设置预览图像的大小限制
        .with_reader(reader) // 设置读取器
        .render() // 渲染预览图像
        .unwrap();
    println!(
        "Got preview in {:.3}ms",
        t_tile.elapsed().as_secs_f32() * 1e3
    ); // 打印生成预览图像所需的时间（毫秒）
    println!("{}", preview); // 打印预览图像的信息

    // 图像输出
    let img: DynamicImage = preview.try_into().unwrap(); // 将预览转换为DynamicImage类型
    img.save(OUTPUT_FILE).unwrap(); // 保存图像到指定的输出文件
    println!("Image saved to {OUTPUT_FILE}"); // 打印保存成功的消息
}
