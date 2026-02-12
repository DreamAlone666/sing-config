pub mod lazy;

use crate::sing_box;

pub trait LoadProvider {
    type Error;

    /// 加载一个 provider，返回不可变引用。
    fn load_provider(&self, tag: &str) -> Result<&sing_box::Config, Self::Error>;
}
