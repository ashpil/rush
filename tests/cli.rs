use assert_cmd::assert::IntoOutputPredicate;
use assert_cmd::cmd::Command;
use escargot::CargoBuild;
use predicates_core::Predicate;

fn test_io<I, O, P>(
    input: I,
    output: O,
) -> Result<assert_cmd::assert::Assert, Box<dyn std::error::Error>>
where
    I: Into<Vec<u8>>,
    O: IntoOutputPredicate<P>,
    P: Predicate<[u8]>,
{
    let mut cmd = Command::from_std(
        CargoBuild::new()
            .bin(env!("CARGO_PKG_NAME"))
            .release()
            .run()?
            .command(),
    );
    let assert = cmd.write_stdin(input).assert();
    Ok(assert.stdout(output))
}

#[test]
fn echo_prints_argument() -> Result<(), Box<dyn std::error::Error>> {
    test_io(
        "echo foo",
        "$> foo
$> 
",
    )?
    .success()
    .code(0);
    Ok(())
}

#[test]
fn empty_input_prints_newline() -> Result<(), Box<dyn std::error::Error>> {
    test_io(
        "", "$> 
",
    )?
    .success()
    .code(0);
    Ok(())
}

#[test]
fn exit_prints_nothing() -> Result<(), Box<dyn std::error::Error>> {
    test_io("exit", "$> ")?.success().code(0);
    Ok(())
}
