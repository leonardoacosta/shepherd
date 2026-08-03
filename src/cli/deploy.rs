use std::process::Command;

const DEPLOY_PROGRAM: &str = "shepherd-deploy";

pub(super) fn run_deploy_command(args: &[String]) -> std::io::Result<i32> {
    run_deploy_with(args, |program| {
        let status = Command::new(program).status()?;
        Ok(if status.success() {
            0
        } else {
            status.code().unwrap_or(1)
        })
    })
}

fn run_deploy_with(
    args: &[String],
    run: impl FnOnce(&str) -> std::io::Result<i32>,
) -> std::io::Result<i32> {
    if !args.is_empty() {
        eprintln!("usage: shepherd deploy");
        return Ok(2);
    }
    run(DEPLOY_PROGRAM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invokes_installfest_owned_deploy_entrypoint() {
        let mut invoked = None;
        let code = run_deploy_with(&[], |program| {
            invoked = Some(program.to_string());
            Ok(0)
        })
        .expect("test deploy runner should succeed");

        assert_eq!(code, 0);
        assert_eq!(invoked.as_deref(), Some("shepherd-deploy"));
    }

    #[test]
    fn rejects_arguments_without_invoking_deploy_entrypoint() {
        let mut invoked = false;
        let code = run_deploy_with(&["mac".into()], |_| {
            invoked = true;
            Ok(0)
        })
        .expect("argument validation should succeed");

        assert_eq!(code, 2);
        assert!(!invoked);
    }
}
