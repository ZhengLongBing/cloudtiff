// 导入必要的库和模块
use cloudtiff;
use std::env;
use std::fs::File;
use std::io::BufReader;

// 定义常量：默认的COG文件路径
const SAMPLE_COG: &str = "data/sample.tif";

fn main() {
    println!("Example: cloudtiff disect");

    // 获取命令行参数，如果没有提供参数，则使用默认的SAMPLE_COG
    let args: Vec<String> = env::args().chain(vec![SAMPLE_COG.to_string()]).collect();
    let path = &args[1];

    // 文件访问
    println!("Opening `{path}`");
    // 打开指定路径的文件
    let file = File::open(path).unwrap();
    // 创建一个带缓冲的读取器，提高读取效率
    let reader = &mut BufReader::new(file);

    println!("Diesecting COG:");
    // 使用cloudtiff库的disect函数解析COG文件
    // 这个函数会打印出COG文件的详细结构信息
    cloudtiff::disect(reader).unwrap();
}
