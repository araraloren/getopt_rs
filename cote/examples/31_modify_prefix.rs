use cote::prelude::*;

#[derive(Debug, Cote, PartialEq, Eq)]
#[cote(parser_mod(parser))]
pub struct Cli {
    #[arg(name = "---bar")]
    bar: usize,
}

fn parser_mod<Set, Ser>(parser: &mut Parser<'_, Set, Ser>) -> Result<(), cote::Error>
where
    Set: PrefixedValidator,
{
    parser.reg_prefix("---").map_err(Into::into)?;
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    assert!(Cli::parse(Args::from(["app", "---bar=42"])).is_ok());
    assert!(Cli::parse(Args::from(["app", "---bar", "42"])).is_ok());

    Ok(())
}