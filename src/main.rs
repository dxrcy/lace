fn main() -> miette::Result<()> {
    let traps = lace::Traps::default();

    lace::main(traps)?;

    Ok(())
}
