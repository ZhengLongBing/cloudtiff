// 如果没有启用 "image" 特性，编译时会报错
#[cfg(not(feature = "image"))]
compile_error!("This example requires the 'image' feature");

// 导入必要的库和模块
use cloudtiff::CloudTiff;
use image::DynamicImage;
use std::fs::File;
use std::time::Instant;

// 定义常量
const SAMPLE_COG: &str = "data/sample.tif"; // 输入文件路径
const OUTPUT_FILE: &str = "data/demo.jpg"; // 输出文件路径
const PREVIEW_MEGAPIXELS: f64 = 1.0; // 预览图像的目标大小（以百万像素为单位）

fn main() {
    println!("Example: cloudtiff demo");

    // 打开样本 COG 文件
    let file = File::open(SAMPLE_COG).unwrap();
    // 调用 save_preview 函数处理文件
    save_preview(file);
}

fn save_preview(mut file: File) {
    // 开始计时：打开 COG 文件
    let t_cog = Instant::now();
    // 使用 CloudTiff 打开 COG 文件
    let cog = CloudTiff::open(&mut file).unwrap();
    // 打印打开 COG 文件所需的时间
    println!(
        "Opened COG in {:.3}ms",
        t_cog.elapsed().as_micros() as f64 / 1000.0
    );

    // 开始计时：生成预览图像
    let t_preview = Instant::now();
    // 使用 CloudTiff 的渲染器生成预览图像
    let preview = cog
        .renderer()
        .with_mp_limit(PREVIEW_MEGAPIXELS) // 设置预览图像的大小限制
        .with_reader(file) // 设置文件读取器
        .render() // 渲染预览图像
        .unwrap();
    // 打印生成预览图像所需的时间
    println!(
        "Got preview in {:.3}ms",
        t_preview.elapsed().as_micros() as f64 / 1000.0
    );

    // 将预览图像转换为 DynamicImage 类型
    let img: DynamicImage = preview.try_into().unwrap();
    // 保存图像到指定的输出文件
    img.save(OUTPUT_FILE).unwrap();
    // 打印保存成功的消息
    println!("Image saved to {OUTPUT_FILE}");
}
