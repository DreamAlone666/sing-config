use std::{
    fs::File,
    io::{Read, stdout},
    path::PathBuf,
};

use ::sing_config::{convert::convert_outbounds, load::lazy::LazyLoader, sing_box, sing_config};
use anyhow::{Context, bail};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// 输入文件的路径
    path: PathBuf,
    /// 输出文件的路径
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    build(&cli).with_context(|| format!("未能构建文件 `{}`", cli.path.display()))?;
    Ok(())
}

fn build(args: &Cli) -> anyhow::Result<()> {
    let mut file = File::open(&args.path).context("未能打开文件")?;
    let extension = args
        .path
        .extension()
        .context("文件应当具有支持的扩展名")?
        .to_string_lossy();
    let config: sing_config::Config = match extension.as_bytes() {
        b"toml" => {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).context("未能完全读取文件")?;
            toml::from_slice(&buf).context("未能解析文件")?
        }
        b"json" => serde_json::from_reader(file).context("未能解析文件")?,
        _ => bail!("不支持扩展名 `{extension}`"),
    };

    let loader = LazyLoader::new(config.providers);
    let outbounds = convert_outbounds(config.outbounds, &loader)?;
    let new_config = sing_box::Config {
        outbounds: outbounds.into_values().collect(),
        extra: config.extra,
    };

    if let Some(path) = &args.output {
        let file =
            File::create(path).with_context(|| format!("未能创建输出文件 `{}`", path.display()))?;
        serde_json::to_writer_pretty(file, &new_config)
            .with_context(|| format!("未能将结果写入文件 `{}`", path.display()))?;
    } else {
        let lock = stdout().lock();
        serde_json::to_writer_pretty(lock, &new_config).context("未能将结果写入 `stdout`")?;
    }

    Ok(())
}
