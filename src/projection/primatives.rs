//! 投影模块的基本数据类型
//!
//! 本模块定义了投影计算中常用的基本数据类型,包括:
//!
//! - `UnitFloat`: 表示 [0, 1] 范围内的浮点数
//! - `Point2D`: 表示二维平面上的点
//! - `Region`: 表示二维平面上的矩形区域
//!
//! 这些类型为投影转换和边界计算提供了基础支持。

use core::f64;
use std::fmt;
use std::ops::{Mul, Sub};

/// 表示 [0, 1] 范围内的浮点数
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitFloat(f64);

impl UnitFloat {
    /// 最小值 0.0
    pub const MIN: UnitFloat = UnitFloat(0.0);
    /// 最大值 1.0
    pub const MAX: UnitFloat = UnitFloat(1.0);

    /// 创建一个新的 UnitFloat 实例
    ///
    /// # 参数
    /// * `value` - 可以转换为 f64 的值
    ///
    /// # 返回
    /// * `Ok(UnitFloat)` - 如果值在 [0, 1] 范围内
    /// * `Err(String)` - 如果值不在 [0, 1] 范围内或无法转换为 f64
    pub fn new<V: TryInto<f64>>(value: V) -> Result<Self, String> {
        let Ok(v) = value.try_into() else {
            return Err("无法将值解释为 f64".to_string());
        };
        if v >= 0.0 && v <= 1.0 {
            Ok(Self(v))
        } else {
            Err("值必须在闭区间 [0.0, 1.0] 内".to_string())
        }
    }

    /// 创建一个新的 UnitFloat 实例，将值限制在 [0, 1] 范围内
    ///
    /// # 参数
    /// * `v` - 输入的 f64 值
    pub fn new_saturated(v: f64) -> Self {
        Self(v.clamp(0.0, 1.0))
    }

    /// 将 UnitFloat 转换为 f64
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// 返回值为 0 的 UnitFloat
    pub fn zero() -> Self {
        Self::MIN
    }

    /// 返回值为 1 的 UnitFloat
    pub fn one() -> Self {
        Self::MAX
    }

    /// 返回最小值 (0)
    pub fn min() -> Self {
        Self::MIN
    }

    /// 返回最大值 (1)
    pub fn max() -> Self {
        Self::MAX
    }
}

impl From<UnitFloat> for f64 {
    fn from(unit_float: UnitFloat) -> Self {
        unit_float.as_f64()
    }
}

impl fmt::Display for UnitFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Sub for UnitFloat {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new_saturated(self.0 - rhs.0)
    }
}

/// 表示二维平面上的点
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D<T> {
    pub x: T,
    pub y: T,
}

/// 表示一个区间
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval<T> {
    pub min: T,
    pub max: T,
}

impl<T> Interval<T> {
    /// 创建一个新的区间
    pub fn new(min: T, max: T) -> Self {
        Self { min, max }
    }
}

impl<T: Copy + Sub<Output = T>> Interval<T> {
    /// 计算区间的范围
    pub fn range(&self) -> T {
        self.max - self.min
    }
}

impl Interval<UnitFloat> {
    /// 创建一个 [0, 1] 的单位区间
    fn unit() -> Self {
        Self {
            min: UnitFloat::MIN,
            max: UnitFloat::MAX,
        }
    }

    /// 创建一个新的饱和区间，确保值在 [0, 1] 范围内
    fn new_saturated(min: f64, max: f64) -> Self {
        let low = min.min(max);
        let high = min.max(max);
        Self {
            min: UnitFloat::new_saturated(low),
            max: UnitFloat::new_saturated(high),
        }
    }
}

/// 表示二维区域
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region<T> {
    pub x: Interval<T>,
    pub y: Interval<T>,
}

impl<T> Region<T> {
    /// 创建一个新的区域
    pub fn new(min_x: T, min_y: T, max_x: T, max_y: T) -> Self {
        Self {
            x: Interval::new(min_x, max_x),
            y: Interval::new(min_y, max_y),
        }
    }

    /// 获取 x 轴区间
    pub fn x(&self) -> &Interval<T> {
        &self.x
    }

    /// 获取 y 轴区间
    pub fn y(&self) -> &Interval<T> {
        &self.y
    }
}

impl Region<f64> {
    /// 将当前区域限制在另一个区域内
    pub fn clamp(self, other: &Self) -> Self {
        Self::new(
            self.x.min.max(other.x.min),
            self.y.min.max(other.y.min),
            self.x.max.min(other.x.max),
            self.y.max.min(other.y.max),
        )
    }

    /// 扩展区域以包含指定点
    pub fn extend(self, point: &Point2D<f64>) -> Self {
        Self::new(
            self.x.min.min(point.x),
            self.y.min.min(point.y),
            self.x.max.max(point.x),
            self.y.max.max(point.y),
        )
    }
}

impl Mul<f64> for Region<f64> {
    type Output = Self;

    /// 将区域的所有坐标乘以一个标量
    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(
            self.x.min * rhs,
            self.y.min * rhs,
            self.x.max * rhs,
            self.y.max * rhs,
        )
    }
}

impl<T: Copy> Region<T> {
    /// 将区域转换为元组 (min_x, min_y, max_x, max_y)
    pub fn as_tuple(&self) -> (T, T, T, T) {
        (self.x.min, self.y.min, self.x.max, self.y.max)
    }

    /// 获取 x 轴最小值
    pub fn x_min(&self) -> T {
        self.x.min
    }

    /// 获取 y 轴最小值
    pub fn y_min(&self) -> T {
        self.y.min
    }

    /// 获取 x 轴最大值
    pub fn x_max(&self) -> T {
        self.x.max
    }

    /// 获取 y 轴最大值
    pub fn y_max(&self) -> T {
        self.y.max
    }
}

impl Region<UnitFloat> {
    /// 创建一个 [0, 1] x [0, 1] 的单位区域
    pub fn unit() -> Self {
        Self {
            x: Interval::unit(),
            y: Interval::unit(),
        }
    }

    /// 创建一个新的饱和区域，确保所有值都在 [0, 1] 范围内
    pub fn new_saturated(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            x: Interval::new_saturated(min_x, max_x),
            y: Interval::new_saturated(min_y, max_y),
        }
    }
}

impl<T: Into<f64> + Copy> Region<T> {
    /// 将区域转换为 f64 元组
    pub fn to_f64(&self) -> (f64, f64, f64, f64) {
        (
            self.x.min.into(),
            self.y.min.into(),
            self.x.max.into(),
            self.y.max.into(),
        )
    }
}

impl<T: fmt::Display> fmt::Display for Region<T> {
    /// 格式化输出区域
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Region(")?;
        self.x.min.fmt(f)?;
        write!(f, " -> ")?;
        self.x.max.fmt(f)?;
        write!(f, ", ")?;
        self.y.min.fmt(f)?;
        write!(f, " -> ")?;
        self.y.max.fmt(f)?;
        write!(f, ")")
    }
}
