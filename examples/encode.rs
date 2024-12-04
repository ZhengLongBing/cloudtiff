// 如果没有启用 "image" 特性，编译时会报错
#[cfg(not(feature = "image"))]
compile_error!("This example requires the 'image' feature");

// 导入必要的库和模块
use cloudtiff::{Encoder, Region};
use image;
use std::fs::File;

// 定义常量：输入文件和输出文件的路径
const INPUT_FILE: &str = "data/demo.jpg";
const OUTPUT_COG: &str = "data/encode.tif";

fn main() {
    println!("Example: cloudtiff encode");

    // 打开输入图像文件
    let img = image::open(INPUT_FILE).unwrap();

    // 定义地理参考信息
    let tiepoint = (499980.0, 6100020.0, 0.0); // 定义图像左上角的地理坐标
    let pixel_scale = (10.0, 10.0, 10.0); // 定义每个像素的地理尺寸
    let full_dim = (10980, 10980); // 定义完整图像的尺寸

    // 创建并配置编码器
    let encoder = Encoder::from_image(&img)
        .unwrap()
        .with_projection(
            32609, // 设置投影系统（这里是UTM Zone 9N）
            Region::new(
                tiepoint.0,                                     // 左边界
                tiepoint.1 - pixel_scale.1 * full_dim.1 as f64, // 下边界
                tiepoint.0 + pixel_scale.0 * full_dim.0 as f64, // 右边界
                tiepoint.1,                                     // 上边界
            ),
        )
        .with_tile_size(256) // 设置瓦片大小为256x256像素
        .with_filter(cloudtiff::ResizeFilter::Nearest) // 设置重采样方法为最近邻
        .with_big_tiff(false); // 不使用BigTIFF格式

    // 创建输出文件
    let mut file = File::create(OUTPUT_COG).unwrap();
    // 将编码后的数据写入文件
    encoder.encode(&mut file).unwrap();
    println!("Saved COG to {OUTPUT_COG}");
}
