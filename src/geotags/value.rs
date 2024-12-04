//! GeoTIFF 键值数据类型模块
//!
//! 本模块定义了 GeoTIFF 键目录中键值的数据类型和相关操作。
//!
//! # 主要类型
//!
//! - `GeoKeyValue` - 表示 GeoTIFF 键值的枚举类型,支持以下数据类型:
//!   - `Short` - 16位无符号整数数组
//!   - `Ascii` - ASCII字符串
//!   - `Double` - 64位浮点数数组
//!   - `Undefined` - 未定义类型
//!
//! # 示例
//!
//! ```
//! use cloudtiff::geotags::GeoKeyValue;
//!
//! // 创建短整型值
//! let short_value = GeoKeyValue::Short(vec![1, 2, 3]);
//!
//! // 创建ASCII字符串值
//! let ascii_value = GeoKeyValue::Ascii("WGS84".to_string());
//! ```

use num_traits::NumCast;
use std::fmt::Display;

/// GeoTIFF 键值数据类型
///
/// 表示 GeoTIFF 键目录中键值的数据类型。每个键值可以是以下类型之一:
///
/// * `Short` - 16位无符号整数数组
/// * `Ascii` - ASCII字符串
/// * `Double` - 64位浮点数数组  
/// * `Undefined` - 未定义类型
///
/// # 示例
/// ```
/// use crate::geotags::GeoKeyValue;
///
/// // 创建短整型值
/// let short_value = GeoKeyValue::Short(vec![1, 2, 3]);
///
/// // 创建ASCII字符串值
/// let ascii_value = GeoKeyValue::Ascii("WGS84".to_string());
///
/// // 创建浮点数值
/// let double_value = GeoKeyValue::Double(vec![1.0, 2.0]);
/// ```
#[derive(Clone, Debug)]
pub enum GeoKeyValue {
    /// 16位无符号整数数组
    Short(Vec<u16>),

    /// ASCII字符串
    Ascii(String),

    /// 64位浮点数数组
    Double(Vec<f64>),

    /// 未定义类型
    Undefined,
}

impl GeoKeyValue {
    /// 尝试将值转换为字符串引用
    ///
    /// 如果值是ASCII类型则返回字符串引用,否则返回None
    ///
    /// # 返回值
    ///
    /// * `Some(&String)` - 如果值是ASCII类型
    /// * `None` - 如果值不是ASCII类型
    pub fn as_string(&self) -> Option<&String> {
        match self {
            GeoKeyValue::Ascii(s) => Some(s),
            _ => None,
        }
    }

    /// 尝试将值转换为指定数值类型
    ///
    /// 如果值是单个数值(Short或Double)则进行类型转换,否则返回None
    ///
    /// # 类型参数
    ///
    /// * `T` - 目标数值类型,必须实现 NumCast trait
    ///
    /// # 返回值
    ///
    /// * `Some(T)` - 转换成功
    /// * `None` - 值不是数值类型或转换失败
    pub fn as_number<T: NumCast>(&self) -> Option<T> {
        match self {
            // 只处理长度为1的数组
            GeoKeyValue::Double(v) if v.len() == 1 => T::from(v[0]),
            GeoKeyValue::Short(v) if v.len() == 1 => T::from(v[0]),
            _ => None,
        }
    }

    /// 尝试将值转换为指定类型的向量
    ///
    /// 如果值是数值数组(Short或Double)则对每个元素进行类型转换
    ///
    /// # 类型参数
    ///
    /// * `T` - 目标数值类型,必须实现 NumCast trait
    ///
    /// # 返回值
    ///
    /// * `Some(Vec<T>)` - 转换成功
    /// * `None` - 值不是数值类型或任一元素转换失败
    pub fn as_vec<T: NumCast>(&self) -> Option<Vec<T>> {
        match self {
            GeoKeyValue::Double(v) => v.iter().map(|x| T::from(*x)).collect(),
            GeoKeyValue::Short(v) => v.iter().map(|x| T::from(*x)).collect(),
            _ => None,
        }
    }
}

/// 实现Display trait以支持格式化输出
///
/// 根据不同的值类型采用不同的格式:
/// * ASCII - 显示字符串内容,换行符显示为"\n"
/// * 单个数值 - 直接显示数值
/// * 数值数组 - 使用调试格式显示
/// * Undefined - 显示"Undefined"
impl Display for GeoKeyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ASCII字符串,将换行符转换为"\n"显示
            GeoKeyValue::Ascii(s) => write!(f, "{}", s.replace("\n", "\\n")),

            // 单个数值直接显示
            GeoKeyValue::Double(v) if v.len() == 1 => write!(f, "{}", v[0]),
            GeoKeyValue::Short(v) if v.len() == 1 => write!(f, "{}", v[0]),

            // 数组使用调试格式
            GeoKeyValue::Double(v) => write!(f, "{v:?}"),
            GeoKeyValue::Short(v) => write!(f, "{v:?}"),

            // 未定义类型
            GeoKeyValue::Undefined => write!(f, "Undefined"),
        }
    }
}
