// 如果没有启用 "http" 特性，编译时会报错
#[cfg(not(feature = "http"))]
compile_error!("This example requires the 'http' feature");

// 导入必要的库和模块
use cloudtiff::{AsyncReadRange, HttpReader};
use std::time::Instant;
use tokio::io::AsyncReadExt;

// 定义常量：COG文件的URL地址
const URL: &str = "http://sentinel-cogs.s3.amazonaws.com/sentinel-s2-l2a-cogs/9/U/WA/2024/8/S2A_9UWA_20240806_0_L2A/TCI.tif";

// 使用tokio运行时的主函数
#[tokio::main]
async fn main() {
    // 创建HTTP读取器
    let mut reader = HttpReader::new(URL).unwrap();
    // 创建一个10字节的缓冲区
    let mut buf = vec![0; 10];

    // 测试 AsyncReadRange 从文件开头读取
    let t0 = Instant::now();
    reader.read_range_async(0, &mut buf).await.unwrap();
    println!(
        "AsyncReadRange in {:.3}ms: 0x{:02X?}",
        t0.elapsed().as_secs_f32() * 1e3,
        buf
    );

    // 测试 AsyncRead 从当前位置读取
    let t0 = Instant::now();
    reader.read(&mut buf).await.unwrap();
    println!(
        "AsyncRead      in {:.3}ms: 0x{:02X?}",
        t0.elapsed().as_secs_f32() * 1e3,
        buf
    );

    // 测试 AsyncReadRange 从偏移量10开始读取
    let t0 = Instant::now();
    reader.read_range_async(10, &mut buf).await.unwrap();
    println!(
        "AsyncReadRange in {:.3}ms: 0x{:02X?}",
        t0.elapsed().as_secs_f32() * 1e3,
        buf
    );

    // 再次测试 AsyncRead 从当前位置读取
    let t0 = Instant::now();
    reader.read(&mut buf).await.unwrap();
    println!(
        "AsyncRead      in {:4.3}ms: 0x{:02X?}",
        t0.elapsed().as_secs_f32() * 1e3,
        buf
    );
}
