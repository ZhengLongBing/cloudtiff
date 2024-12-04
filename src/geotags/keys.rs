//! GeoTIFF 键目录管理模块
//!
//! 本模块实现了 GeoTIFF 规范中的 GeoKeyDirectory 结构，用于管理和存储地理空间元数据。
//! 基于 OGC GeoTIFF 1.1 标准的键目录标签规范实现。
//!
//! # 主要功能
//!
//! - GeoKey 目录的解析和序列化 - 支持读写 GeoTIFF 键目录数据
//! - GeoKey 值的存储和访问 - 提供键值对的增删改查操作
//! - 多种数据类型支持 - 包括短整型、ASCII字符串、双精度浮点型
//! - TIFF IFD 集成 - 与 TIFF 标签系统无缝对接
//!
//! # 参考标准
//!
//! - [GeoKeyDirectoryTag 规范](https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_geokeydirectorytag)
//! - [GeoKey 数据类型](https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_geokeydatatypes)
//!
//! # 示例
//!
//! ```
//! use cloudtiff::geotags::GeoKeyDirectory;
//!
//! // 创建新的键目录
//! let mut directory = GeoKeyDirectory::new();
//!
//! // 添加键值对
//! directory.add_key(GeoKeyId::GTModelTypeGeoKey, GeoKeyValue::Short(vec![2]));
//! ```

use std::fmt::Display;

use super::{get_geo_tag_values, GeoKeyId, GeoKeyValue, GeoTiffError};
use crate::tiff::{Endian, Ifd, TagData, TagId, TagType};

/// GeoTIFF 键目录结构
///
/// 存储和管理 GeoTIFF 文件中的地理空间元数据键值对。
///
/// # 字段说明
///
/// * `version` - 键目录版本号
/// * `revision` - 修订版本号，格式为 (主版本号, 次版本号)
/// * `keys` - 存储的 GeoKey 列表
#[derive(Clone, Debug)]
pub struct GeoKeyDirectory {
    pub version: u16,
    pub revision: (u16, u16),
    pub keys: Vec<GeoKey>,
}

/// GeoTIFF 键值对
///
/// 表示单个地理空间元数据项。
///
/// # 字段说明
///
/// * `code` - 键的数字标识符
/// * `value` - 键对应的值，支持多种数据类型
#[derive(Clone, Debug)]
pub struct GeoKey {
    pub code: u16,
    pub value: GeoKeyValue,
}

impl GeoKey {
    /// 获取键的标准标识符
    ///
    /// 尝试将数字代码转换为标准的 GeoKeyId 枚举值。
    /// 如果代码不是标准键值，返回 None。
    pub fn id(&self) -> Option<GeoKeyId> {
        GeoKeyId::try_from(self.code).ok()
    }
}

impl GeoKeyDirectory {
    /// 创建新的键目录
    ///
    /// 返回一个初始化的键目录，包含：
    /// - 版本号 1
    /// - 修订版本 1.0
    /// - 空的键列表
    pub fn new() -> Self {
        Self {
            version: 1,
            revision: (1, 0),
            keys: vec![],
        }
    }

    /// 从 TIFF IFD 解析键目录
    ///
    /// # 参数
    ///
    /// * `ifd` - TIFF 图像文件目录
    ///
    /// # 返回值
    ///
    /// 返回解析后的键目录或错误
    ///
    /// # 错误
    ///
    /// - 如果目录标签缺失或格式错误
    /// - 如果目录数据长度无效
    /// - 如果键值数据无法解析
    pub fn parse(ifd: &Ifd) -> Result<Self, GeoTiffError> {
        // 从 TIFF IFD 获取 GeoKeyDirectory 标签值
        let directory_values = get_geo_tag_values(ifd, TagId::GeoKeyDirectory)?;

        // 验证目录至少包含头部信息(4个值)
        if directory_values.len() < 4 {
            return Err(GeoTiffError::BadTag(TagId::GeoKeyDirectory));
        }

        // 解析目录头部信息
        let version: u16 = directory_values[0]; // GeoTIFF 版本号
        let revision: u16 = directory_values[1]; // 主修订版本号
        let minor_revision: u16 = directory_values[2]; // 次修订版本号
        let key_count: u16 = directory_values[3]; // 键值对数量

        // 验证目录总长度是否足够存储所有键值对
        // 每个键值对需要4个值:键ID、位置、计数、偏移量
        let min_valid_directory_size = 4 + key_count * 4;
        if directory_values.len() < min_valid_directory_size as usize {
            return Err(GeoTiffError::BadTag(TagId::GeoKeyDirectory));
        }
        // 解析所有键值对
        // 每个键值对包含4个值:
        // - code: 键ID
        // - location: 值存储位置(0表示直接存储,其他表示标签ID)
        // - count: 值的数量
        // - offset: 值的偏移量或直接值
        let keys: Vec<GeoKey> = (0..key_count as usize)
            .map(|i| {
                // 计算当前键值对在目录中的偏移量
                // 每个键值对占用4个值,从第5个值开始存储
                let entry_offset = (i + 1) * 4;

                // 获取键值对的4个组成部分
                let code = directory_values[entry_offset + 0]; // 键ID
                let location = directory_values[entry_offset + 1]; // 存储位置
                let count = directory_values[entry_offset + 2]; // 值数量
                let offset = directory_values[entry_offset + 3]; // 偏移量

                // 解析键值
                let value = if location == 0 {
                    // location=0表示值直接存储在offset中
                    GeoKeyValue::Short(vec![offset])
                } else {
                    // 否则需要从指定标签中读取值
                    let start = offset as usize; // 值的起始位置
                    let end = (offset + count) as usize; // 值的结束位置
                    let tag = ifd.get_tag_by_code(location); // 获取标签

                    // 根据标签类型解析值
                    tag.and_then(|tag| match tag.datatype {
                        // ASCII类型:转换为字符串并去除结尾的分隔符
                        TagType::Ascii => tag.try_to_string().map(|s| {
                            GeoKeyValue::Ascii(
                                s[start..end]
                                    .to_string()
                                    .trim_end_matches(|c| c == '|' || c == '\0')
                                    .to_string(),
                            )
                        }),
                        // 短整型:直接获取指定范围的值
                        TagType::Short => tag
                            .values()
                            .map(|v| GeoKeyValue::Short(v[start..end].to_vec())),
                        // 双精度浮点型:直接获取指定范围的值
                        TagType::Double => tag
                            .values()
                            .map(|v| GeoKeyValue::Double(v[start..end].to_vec())),
                        // 其他类型:返回None
                        _ => None,
                    })
                    // 如果解析失败则返回未定义值
                    .unwrap_or(GeoKeyValue::Undefined)
                };

                // 创建并返回GeoKey
                GeoKey { code, value }
            })
            .collect();

        // 返回解析结果
        Ok(Self {
            version,
            revision: (revision, minor_revision),
            keys,
        })
    }

    /// 将键目录添加到 TIFF IFD
    ///
    /// # 参数
    ///
    /// * `ifd` - 目标 TIFF 图像文件目录
    /// * `endian` - 字节序
    ///
    /// 将键目录序列化并写入以下 TIFF 标签：
    /// - GeoKeyDirectory：键目录结构
    /// - GeoAsciiParams：ASCII 参数值
    /// - GeoDoubleParams：双精度浮点参数值
    pub fn add_to_ifd(&self, ifd: &mut Ifd, endian: Endian) {
        // 序列化键目录数据
        let (key_directory, ascii_params, double_params) = self.unparse();

        // 写入键目录标签
        ifd.set_tag(
            TagId::GeoKeyDirectory,
            TagData::Short(key_directory),
            endian,
        );

        // 如果存在ASCII参数,写入ASCII参数标签
        if ascii_params.len() > 0 {
            ifd.set_tag(TagId::GeoAsciiParams, TagData::Ascii(ascii_params), endian);
        }

        // 如果存在双精度参数,写入双精度参数标签
        if double_params.len() > 0 {
            ifd.set_tag(
                TagId::GeoDoubleParams,
                TagData::Double(double_params),
                endian,
            );
        }
    }

    /// 序列化键目录
    ///
    /// 将键目录转换为 TIFF 标签可用的数据格式。
    ///
    /// # 返回值
    ///
    /// 返回元组 (目录数据, ASCII参数, 双精度参数)：
    /// - 目录数据：包含键目录结构和短整型值
    /// - ASCII参数：所有ASCII值的连接
    /// - 双精度参数：所有双精度浮点值
    pub fn unparse(&self) -> (Vec<u16>, Vec<u8>, Vec<f64>) {
        // 初始化存储向量
        let mut directory = vec![]; // 存储键目录结构
        let mut shorts = vec![]; // 存储短整型值
        let mut asciis = vec![]; // 存储ASCII字符串
        let mut doubles = vec![]; // 存储双精度浮点数

        // 计算目录头部大小(每个键4个u16值,加上头部4个u16)
        let dir_size = 4 * (self.keys.len() + 1) as u16;

        // 写入目录头部信息
        directory.push(self.version); // 版本号
        directory.push(self.revision.0); // 主版本号
        directory.push(self.revision.1); // 次版本号
        directory.push(self.keys.len() as u16); // 键的数量

        // 处理每个键
        for key in &self.keys {
            // 将键代码添加到目录
            directory.push(key.code);

            // 根据键值的类型处理不同的情况
            match &key.value {
                GeoKeyValue::Short(vec) => match vec.len() {
                    // 如果向量为空，添加三个占位符
                    0 => directory.extend([0, 0, 0]),
                    // 如果向量只包含一个元素，添加其内容
                    1 => {
                        directory.push(0); // 数据类型为0代表数字在占位符中
                        directory.push(1); // 数据长度为1
                        directory.push(vec[0]); // 向量的单个值
                    }
                    // 如果向量包含多个元素，存储在短整型扩展区
                    n => {
                        directory.push(TagId::GeoKeyDirectory as u16); // 指定使用短整型扩展
                        directory.push(n as u16); // 存储数据的长度
                        directory.push(dir_size + shorts.len() as u16); // 存储的偏移量
                        shorts.extend(vec); // 添加实际数据
                    }
                },
                GeoKeyValue::Ascii(s) => {
                    // ASCII字符串的情况
                    directory.push(TagId::GeoAsciiParams as u16); // 指定使用ASCII扩展
                    directory.push(s.len() as u16); // ASCII字符串的长度
                    directory.push(asciis.len() as u16); // 储存的起始偏移量
                    asciis.extend(s.bytes()); // 添加ASCII字符串数据
                }
                GeoKeyValue::Double(vec) => {
                    // 双精度浮点数的情况
                    directory.push(TagId::GeoDoubleParams as u16); // 指定使用双精度扩展
                    directory.push(vec.len() as u16); // 数据长度
                    directory.push(doubles.len() as u16); // 储存的起始偏移量
                    doubles.extend(vec); // 添加双精度数据
                }
                GeoKeyValue::Undefined => directory.extend([0, 0, 0]), // 未定义的键值类型处理占位符
            }
        }
        // 如果ASCII字符串存在，添加一个终止空字符
        if asciis.len() > 0 {
            asciis.push(0); // 添加null作为结束字符
        }

        // 返回包含目录数据、ASCII字符串和双精度浮点数的元组
        ([directory, shorts].concat(), asciis, doubles)
    }
}

/// 实现键的显示格式化
///
/// 格式化输出包括：
/// - 标准键ID的符号名称或十六进制代码
/// - 键值的字符串表示
impl Display for GeoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id_string = match self.id() {
            Some(id) => format!("{id:?}"),
            None => format!("0x{:04X}", self.code),
        };
        write!(f, "{}: {}", id_string, self.value)
    }
}
