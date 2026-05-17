use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nemo-storybook-generator")]
#[command(about = "Generate a Nemo storybook XML config from component schemas")]
struct Args {
    /// Output file path (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(path) = args.output {
        nemo_storybook_generator::generate_storybook_xml_to_file(&path)?;
        eprintln!("Storybook config written to: {}", path.display());
    } else {
        print!("{}", nemo_storybook_generator::generate_storybook_xml());
    }
    Ok(())
}
